use tauri::Manager;

pub struct KokoroState {
    pub engine: tokio::sync::Mutex<Option<kokoro_micro::TtsEngine>>,
    pub status: std::sync::Arc<std::sync::Mutex<String>>,
}

pub enum PlaySource {
    Samples(Vec<f32>),
    File(String),
}

struct PlayRequest {
    source: PlaySource,
    volume: f32,
    speed: f32,
    // Sinaliza para play_voice quando o áudio terminar (natural ou por stop()).
    // Buffer 1 para que o send não bloqueie caso o receiver já tenha sido descartado.
    done_tx: std::sync::mpsc::SyncSender<()>,
}

pub struct VoicePlayer {
    sender: std::sync::Mutex<std::sync::mpsc::Sender<PlayRequest>>,
    sink: std::sync::Arc<std::sync::Mutex<Option<rodio::Sink>>>,
}

impl VoicePlayer {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<PlayRequest>();
        let sink = std::sync::Arc::new(std::sync::Mutex::new(None::<rodio::Sink>));
        let sink_clone = sink.clone();

        std::thread::spawn(move || {
            println!("[VoicePlayer] Inicializando dispositivo de áudio nativo na thread dedicada...");
            if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                println!("[VoicePlayer] Dispositivo de áudio padrão aberto com sucesso!");
                while let Ok(req) = rx.recv() {
                    // Para qualquer áudio tocando antes de iniciar o novo
                    {
                        let mut active_sink = sink_clone.lock().unwrap();
                        if let Some(s) = active_sink.take() {
                            s.stop();
                        }
                    }

                    if let Ok(new_sink) = rodio::Sink::try_new(&handle) {
                        new_sink.set_volume(req.volume);
                        new_sink.set_speed(req.speed);

                        match req.source {
                            PlaySource::Samples(samples) => {
                                let source = rodio::buffer::SamplesBuffer::new(1, 24000, samples);
                                new_sink.append(source);
                            }
                            PlaySource::File(path_str) => {
                                if let Ok(file) = std::fs::File::open(&path_str) {
                                    if let Ok(source) = rodio::Decoder::new(std::io::BufReader::new(file)) {
                                        new_sink.append(source);
                                    } else {
                                        eprintln!("[VoicePlayer] Erro ao decodificar WAV: {}", path_str);
                                    }
                                } else {
                                    eprintln!("[VoicePlayer] Erro ao abrir WAV no disco: {}", path_str);
                                }
                            }
                        }

                        // Armazena sink ativo para que stop() possa cancelar
                        {
                            let mut active_sink = sink_clone.lock().unwrap();
                            *active_sink = Some(new_sink);
                        }

                        // Aguarda término natural (sink.empty()) ou cancelamento (sink.take() pelo stop()).
                        // Polling de 50ms — baixíssimo custo de CPU, mas detecta o fim rapidamente.
                        loop {
                            std::thread::sleep(std::time::Duration::from_millis(50));
                            let is_done = {
                                let guard = sink_clone.lock().unwrap();
                                // None → stop() limpou o sink; empty() → playback terminou naturalmente
                                guard.as_ref().map(|s| s.empty()).unwrap_or(true)
                            };
                            if is_done { break; }
                        }

                        // Remove o sink da guard (pode já ser None se stop() foi chamado)
                        { sink_clone.lock().unwrap().take(); }
                    }

                    // Sinaliza para play_voice que a reprodução terminou
                    let _ = req.done_tx.send(());
                }
            } else {
                eprintln!("[VoicePlayer] ERRO crítico ao inicializar dispositivo de som nativo.");
            }
        });

        Self {
            sender: std::sync::Mutex::new(tx),
            sink,
        }
    }

    /// Envia o áudio para a thread de reprodução e retorna um Receiver que completa
    /// quando o áudio terminar (natural ou por stop()).
    pub fn play_source(&self, source: PlaySource, volume: f32, speed: f32) -> Result<std::sync::mpsc::Receiver<()>, String> {
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(1);
        let req = PlayRequest { source, volume, speed, done_tx };
        let sender = self.sender.lock().unwrap();
        sender.send(req).map_err(|e| format!("Falha ao despachar áudio: {}", e))?;
        Ok(done_rx)
    }

    pub fn stop(&self) {
        let mut active_sink = self.sink.lock().unwrap();
        if let Some(sink) = active_sink.take() {
            sink.stop();
        }
    }
}

#[tauri::command]
pub(crate) async fn get_kokoro_status(app: tauri::AppHandle) -> Result<String, String> {
    if let Some(state) = app.try_state::<KokoroState>() {
        let status = state.status.lock().unwrap().clone();
        Ok(status)
    } else {
        Ok("loading".to_string())
    }
}

fn write_wav_file(path: &std::path::Path, samples: &[f32]) -> Result<(), std::io::Error> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;

    let num_samples = samples.len();
    let sample_rate = 24000u32;
    let bits_per_sample = 16u16;
    let num_channels = 1u16;
    let byte_rate = sample_rate * num_channels as u32 * (bits_per_sample / 8) as u32;
    let block_align = num_channels * (bits_per_sample / 8);
    let subchunk2_size = num_samples as u32 * (bits_per_sample / 8) as u32;
    let chunk_size = 36 + subchunk2_size;

    // Header
    file.write_all(b"RIFF")?;
    file.write_all(&chunk_size.to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // Subchunk1Size
    file.write_all(&1u16.to_le_bytes())?;  // AudioFormat (1 = PCM)
    file.write_all(&num_channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&subchunk2_size.to_le_bytes())?;

    // Data
    for &sample in samples {
        let pcm = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        file.write_all(&pcm.to_le_bytes())?;
    }

    Ok(())
}

#[tauri::command]
pub(crate) async fn play_voice(
    text: String,
    voice: String,
    volume: f32,
    speed: f32,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let state = app.try_state::<KokoroState>()
        .ok_or_else(|| "O motor de voz Kokoro não foi registrado.".to_string())?;

    println!("[play_voice] Chamado com text: '{}', voice: '{}', volume: {}, speed: {}", text, voice, volume, speed);

    let kokoro_voice = if voice.to_lowercase() == "francisca" {
        "pf_dora"
    } else if voice.to_lowercase() == "antonio" {
        "pm_alex"
    } else {
        &voice
    };

    let phrase_key = format!("{}:{:.2}:{}", kokoro_voice, speed, text);

    // 1. Acessa o Pool do banco de dados SQLite
    let pool = app.try_state::<crate::db::DbState>()
        .ok_or_else(|| "Banco de dados não inicializado.".to_string())?;
    let db = &pool.0;

    // 2. Tenta recuperar do SQLite um cache existente e não expirado
    let cached_entry: Option<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT id, audio_path FROM voice_cache WHERE phrase_key = ? AND expires_at > datetime('now')"
    )
    .bind(&phrase_key)
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Erro ao consultar cache de voz: {}", e))?;

    let player = app.try_state::<VoicePlayer>()
        .ok_or_else(|| "O motor de áudio nativo do Rust não foi inicializado (verifique seus dispositivos de som).".to_string())?;

    // 3. Se existir a entrada de cache e o arquivo físico existir no disco
    if let Some((_id, audio_path_str)) = cached_entry {
        let path = std::path::Path::new(&audio_path_str);
        if path.exists() {
            println!("[Kokoro Cache DB] HIT! Áudio recuperado do cache SQLite e disco: '{}'", audio_path_str);
            let done_rx = player.inner().play_source(PlaySource::File(audio_path_str), volume, 1.0)?;
            // Aguarda o término real do áudio (não retorna até o Kokoro terminar de falar)
            tokio::task::spawn_blocking(move || { let _ = done_rx.recv(); })
                .await.map_err(|e| e.to_string())?;
            return Ok(());
        }
        // Se o arquivo foi excluído fisicamente, removemos o registro inválido para autocura
        let _ = sqlx::query("DELETE FROM voice_cache WHERE phrase_key = ?").bind(&phrase_key).execute(db).await;
    }

    // 4. Cache MISS: Gerar nova voz neural via Kokoro ONNX
    println!("[Kokoro Cache DB] MISS. Sintetizando voz neural local via ONNX...");
    let mut engine_lock = state.engine.lock().await;
    let engine = engine_lock.as_mut().ok_or_else(|| {
        let status = state.status.lock().unwrap().clone();
        if status.starts_with("error") {
            format!("O motor de voz Kokoro falhou ao carregar: {}", status)
        } else {
            "O motor de voz Kokoro ainda está sendo inicializado/baixado. Por favor, aguarde alguns instantes.".to_string()
        }
    })?;

    let start_time = std::time::Instant::now();
    let samples = engine.synthesize_with_options(
        &text,
        Some(kokoro_voice),
        speed,
        1.0, // gain
        Some("pt-br")
    ).map_err(|e| format!("Erro na síntese Kokoro: {}", e))?;

    // Libera o lock do Kokoro assim que a síntese termina. As etapas seguintes
    // (escrita em disco, SQLite e playback) não precisam bloquear novas sínteses.
    drop(engine_lock);

    let duration = start_time.elapsed();
    println!("[Kokoro] Síntese concluída em: {:?}", duration);

    // 5. Salva o arquivo de áudio WAV físico na pasta de cache do aplicativo
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let cache_dir = app_dir.join("voice_cache");
    if !cache_dir.exists() {
        let _ = std::fs::create_dir_all(&cache_dir);
    }

    // Gera um nome único de arquivo com base no timestamp
    let file_id = format!("voice_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let audio_file_path = cache_dir.join(format!("{}.wav", file_id));
    let audio_path_str = audio_file_path.to_string_lossy().to_string();

    let mut saved_to_db = false;
    if let Err(e) = write_wav_file(&audio_file_path, &samples) {
        eprintln!("[Kokoro Cache DB] Falha ao escrever arquivo WAV em disco: {}", e);
    } else {
        // Arquivo WAV escrito com sucesso, calcula o tamanho e salva no SQLite com expiração de 7 dias
        if let Ok(metadata) = std::fs::metadata(&audio_file_path) {
            let file_size = metadata.len();
            let db_res = sqlx::query(
                "INSERT OR REPLACE INTO voice_cache (phrase_key, audio_path, file_size, audio_format, expires_at)
                 VALUES (?, ?, ?, 'wav', datetime('now', '+7 days'))"
            )
            .bind(&phrase_key)
            .bind(&audio_path_str)
            .bind(file_size as i64)
            .execute(db)
            .await;

            if db_res.is_ok() {
                saved_to_db = true;
                println!("[Kokoro Cache DB] Áudio salvo com sucesso no banco e disco (7 dias de expiração): '{}'", audio_path_str);
            } else if let Err(err) = db_res {
                eprintln!("[Kokoro Cache DB] Erro ao registrar áudio no SQLite: {}", err);
            }
        }
    }

    // 6. Toca as amostras recém-geradas diretamente da memória para velocidade máxima na primeira reprodução
    let done_rx = player.inner().play_source(PlaySource::Samples(samples), volume, 1.0)?;
    // Aguarda o término real do áudio antes de retornar ao frontend
    tokio::task::spawn_blocking(move || { let _ = done_rx.recv(); })
        .await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub(crate) async fn stop_voice(app: tauri::AppHandle) -> Result<(), String> {
    println!("[stop_voice] Chamado");
    if let Some(player) = app.try_state::<VoicePlayer>() {
        player.inner().stop();
    }
    Ok(())
}
