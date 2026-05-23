mod bridge;
mod config;
mod db;
mod ddragon;
mod game_flow;
mod groq;
mod lca;
mod lcu;
pub mod post_game;
mod riot_api;
pub mod live_coach;
pub mod voice;
mod window;

use tauri::{Emitter, Manager};

pub use voice::{KokoroState, VoicePlayer, PlaySource};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Verifica e instala a atualização disponível (chamado pelo frontend após confirmação do usuário).
#[tauri::command]
async fn download_and_install_update(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater_builder().build().map_err(|e| e.to_string())?;
    match updater.check().await.map_err(|e| e.to_string())? {
        Some(update) => {
            println!("[Updater] Baixando versão {}...", update.version);
            update
                .download_and_install(|_chunk, _total| {}, || {})
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        None => Err("Nenhuma atualização disponível.".into()),
    }
}

#[tauri::command]
fn is_db_ready(app: tauri::AppHandle) -> bool {
    app.try_state::<crate::db::DbState>().is_some()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    window.app_handle().exit(0);
                }
            }
        })
        .setup(|app| {
            // Janelas de overlay puras devem ignorar cliques do mouse — o jogador precisa
            // interagir com o League of Legends por baixo delas sem interferência.
            for label in &["build", "flashcard", "rune-overlay", "ward-map"] {
                if let Some(win) = app.get_webview_window(label) {
                    let _ = win.set_ignore_cursor_events(true);
                }
            }

            let handle = app.handle().clone();
            let handle_for_spawn = handle.clone();

            // Initialize database asynchronously in the background to prevent Tokio deadlocks
            // during new table creation when starting with an empty database.
            tauri::async_runtime::spawn(async move {
                println!("[System] Inicializando banco de dados...");
                match crate::db::init_db(&handle_for_spawn).await {
                    Ok(pool) => {
                        let state = crate::db::DbState(pool.clone());
                        handle_for_spawn.manage(state);
                        println!("Database V2 initialized successfully");

                        // --- LIMPEZA AUTOMÁTICA DE CACHE DE VOZ EXPIRADO (Garbage Collection) ---
                        let pool_clone = pool.clone();
                        tauri::async_runtime::spawn(async move {
                            println!("[Voice Cache GC] Iniciando verificação de arquivos expirados...");
                            let expired_files: Result<Vec<(String,)>, _> = sqlx::query_as(
                                "SELECT audio_path FROM voice_cache WHERE expires_at <= datetime('now')"
                            )
                            .fetch_all(&pool_clone)
                            .await;

                            if let Ok(files) = expired_files {
                                let mut deleted_count = 0;
                                for (path_str,) in files {
                                    let path = std::path::Path::new(&path_str);
                                    if path.exists() {
                                        if let Ok(_) = std::fs::remove_file(path) {
                                            deleted_count += 1;
                                        }
                                    }
                                }

                                // Deleta os registros da tabela
                                let delete_res = sqlx::query("DELETE FROM voice_cache WHERE expires_at <= datetime('now')")
                                    .execute(&pool_clone)
                                    .await;

                                if deleted_count > 0 || delete_res.is_ok() {
                                    println!(
                                        "[Voice Cache GC] Limpeza concluída: {} arquivos físicos expirados removidos do disco.",
                                        deleted_count
                                    );
                                } else {
                                    println!("[Voice Cache GC] Nenhum arquivo expirado encontrado para limpeza.");
                                }
                            }
                        });
                    }
                    Err(e) => eprintln!(
                        "[System] ERRO crítico ao inicializar o banco de dados: {}",
                        e
                    ),
                }
            });

            // Register KokoroState and launch background downloader/loader
            let kokoro_status = std::sync::Arc::new(std::sync::Mutex::new("loading".to_string()));
            let kokoro_status_clone = kokoro_status.clone();
            app.manage(voice::KokoroState {
                engine: std::sync::Arc::new(std::sync::Mutex::new(None::<kokoro_micro::TtsEngine>)),
                status: kokoro_status,
            });

            // Garante que o ONNX Runtime use todos os cores disponíveis via OpenMP.
            // Deve ser definido ANTES de qualquer Session ser criada.
            let cpu_count = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
            unsafe {
                std::env::set_var("OMP_NUM_THREADS", cpu_count.to_string());
            }
            println!("[Kokoro] OMP_NUM_THREADS={} (cores disponíveis)", cpu_count);

            let handle_for_kokoro = handle.clone();
            tauri::async_runtime::spawn(async move {
                println!("[Kokoro] Inicializando o motor de voz local Kokoro (isto pode baixar o modelo de 300MB+ se for a primeira vez)...");
                match kokoro_micro::TtsEngine::new().await {
                    Ok(mut engine) => {
                        println!("[Kokoro] Motor de voz Kokoro carregado com sucesso! Realizando aquecimento (warm-up)...");
                        let start_warmup = std::time::Instant::now();
                        // Warm-up com frase representativa de tips reais: inclui números, % e travessão
                        // para forçar o eSpeak e o ONNX a JIT-compilar os paths usados no coaching.
                        // "a" sozinho não aquece eSpeak para tokens numéricos/especiais em pt-br.
                        let warmup_phrases = [
                            "Bana Amumu — dominante neste ELO.",
                            "Bana Ekko ou Amumu, 66 por cento de vitória.",
                        ];
                        for phrase in &warmup_phrases {
                            if let Err(e) = engine.synthesize_with_options(phrase, Some("pf_dora"), 1.0, 0.0, Some("pt-br")) {
                                eprintln!("[Kokoro] Erro no warm-up '{}': {}", phrase, e);
                            }
                        }
                        println!("[Kokoro] Aquecimento concluído em {:?}!", start_warmup.elapsed());

                        if let Some(state) = handle_for_kokoro.try_state::<voice::KokoroState>() {
                            if let Ok(mut lock) = state.engine.lock() {
                                *lock = Some(engine);
                            }
                        }
                        if let Ok(mut status_lock) = kokoro_status_clone.lock() {
                            *status_lock = "ready".to_string();
                        }
                    }
                    Err(e) => {
                        eprintln!("[Kokoro] Falha crítica ao inicializar o Kokoro: {}", e);
                        if let Ok(mut status_lock) = kokoro_status_clone.lock() {
                            *status_lock = format!("error: {}", e);
                        }
                    }
                }
            });

            // Initialize and register native VoicePlayer state
            let player = voice::VoicePlayer::new();
            app.manage(player);
            println!("[VoicePlayer] Inicializado e gerenciado pelo Rust com sucesso!");

            bridge::start_background_bridge(handle.clone());

            // Auto-sync DDragon essentials (itens e campeões) se tabela vazia
            if let Some(db_state) = handle.try_state::<crate::db::DbState>() {
                let pool_ddragon = db_state.0.clone();
                tauri::async_runtime::spawn(async move {
                    let item_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM items")
                        .fetch_one(&pool_ddragon).await.unwrap_or(0);

                    if item_count == 0 {
                        println!("[DDragon] Tabela de itens vazia — sincronizando automaticamente...");
                        match crate::ddragon::get_latest_version_internal(&pool_ddragon).await {
                            Ok(version) => {
                                let client = crate::ddragon::DDragonClient::new();
                                let lang = crate::config::DEFAULT_LANG;

                                if let Ok(data) = client.get(&format!("/cdn/{}/data/{}/item.json", version, lang)).await {
                                    match crate::db::sync_service::sync_items(&pool_ddragon, &data).await {
                                        Ok(_) => println!("[DDragon] Itens sincronizados com sucesso."),
                                        Err(e) => eprintln!("[DDragon] Erro ao salvar itens: {}", e),
                                    }
                                }
                                if let Ok(data) = client.get(&format!("/cdn/{}/data/{}/champion.json", version, lang)).await {
                                    match crate::db::sync_service::sync_champions(&pool_ddragon, &data).await {
                                        Ok(_) => println!("[DDragon] Campeões sincronizados com sucesso."),
                                        Err(e) => eprintln!("[DDragon] Erro ao salvar campeões: {}", e),
                                    }
                                }
                            }
                            Err(e) => eprintln!("[DDragon] Erro ao obter versão: {}", e),
                        }
                    } else {
                        println!("[DDragon] {} itens já no banco — pulando sync.", item_count);
                    }
                });
            }

            // Verifica atualizações em background (5s após inicialização para não atrasar o startup)
            let update_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                use tauri_plugin_updater::UpdaterExt;
                let Ok(updater) = update_handle.updater_builder().build() else { return };
                match updater.check().await {
                    Ok(Some(update)) => {
                        println!("[Updater] Nova versão disponível: {}", update.version);
                        let _ = update_handle.emit("update-available", serde_json::json!({
                            "version": update.version,
                            "notes": update.body.clone().unwrap_or_default()
                        }));
                    }
                    Ok(None)   => println!("[Updater] App está na versão mais recente."),
                    Err(e)     => println!("[Updater] Erro ao verificar updates: {e}"),
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            is_db_ready,
            lcu::get_lcu_status,
            lcu::get_current_summoner,
            lcu::get_user_rank,
            game_flow::get_game_state,
            lca::get_lca_status,
            lca::get_all_game_data,
            lca::get_active_player,
            lca::get_player_list,
            riot_api::get_account_by_riot_id,
            riot_api::get_summoner_by_puuid,
            riot_api::get_match_history,
            riot_api::get_match_details,
            riot_api::trigger_post_game_analysis,
            post_game::get_post_game_report,
            post_game::set_post_game_report,
            ddragon::get_ddragon_versions,
            ddragon::get_ddragon_champions,
            ddragon::get_ddragon_items,
            ddragon::get_ddragon_spells,
            ddragon::get_ddragon_champion_details,
            ddragon::hydrate_builds,
            groq::get_groq_settings,
            groq::set_groq_settings,
            groq::test_groq_connection,
            db::coach_service::update_meta_build_command,
            db::coach_service::get_db_champions,
            db::coach_service::get_top_champions,
            db::coach_service::get_blitz_recommendations,
            db::coach_service::get_tactical_tips_command,
            db::coach_service::get_rune_overlay_data_command,
            db::coach_service::get_recommended_builds_command,
            db::matchup_service::sync_matchups_command,
            db::matchup_service::get_champion_matchups_command,
            db::matchup_service::get_matchup_count_command,
            db::build_sync_service::sync_builds_command,
            db::build_sync_service::get_champion_build_command,
            db::build_sync_service::get_situational_items_command,
            db::build_sync_service::get_champion_runes_command,
            db::rune_sync_service::sync_and_validate_runes_command,
            db::rune_sync_service::reset_builds_and_runes_command,
            db::blitz_service::sync_blitz_command,
            db::vercel_sync_service::sync_vercel_command,
            db::vercel_sync_service::force_sync_vercel_command,
            db::vercel_sync_service::get_sync_coverage,
            db::vercel_sync_service::get_sync_state,
            db::player_style_service::get_player_style_analysis,
            db::player_style_service::save_lcu_summoner_command,
            db::player_style_service::get_saved_summoner_command,
            window::get_monitor_dimensions,
            window::resize_main_window,
            voice::get_kokoro_status,
            voice::get_audio_status,
            voice::play_voice,
            voice::stop_voice,
            voice::test_audio_output,
            download_and_install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
