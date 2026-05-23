pub mod helpers;
mod in_game;

use std::time::Duration;
use tauri::{Emitter, Manager};
use serde_json::json;
use crate::{lcu, lca, game_flow, db, config};

pub fn start_background_bridge(handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut lcu_conn: Option<lcu::LcuConnection> = None;
        let mut last_phase: Option<game_flow::ChampSelectPhase> = None;
        let mut coach_state = crate::live_coach::CoachState::new();
        let mut last_tip_emit_time: f64 = -30.0; // start negative so first tip fires immediately
        let mut cached_profile: Option<crate::db::player_style_service::PlayerAggregateProfile> = None;
        let lca_conn = lca::LcaConnection::new();
        // Rastreia se estava em jogo no poll anterior para detectar início de nova partida
        let mut was_in_game = false;

        loop {
            // ... (keep LCU connection logic)
            if let Some(ref conn) = lcu_conn {
                if !conn.is_alive().await {
                    lcu_conn = None;
                }
            }

            if lcu_conn.is_none() {
                lcu_conn = lcu::LcuConnection::new();
            }

            let (status, summoner, game_state, champ_name, debug_info, elo) = match &lcu_conn {
                Some(conn) => {
                    let s = conn.get("/lol-summoner/v1/current-summoner").await.unwrap_or(json!({}));

                    let active_puuid = s["puuid"].as_str().unwrap_or("").to_string();
                    if !active_puuid.is_empty() && cached_profile.is_none() {
                        if let Some(state) = handle.try_state::<db::DbState>() {
                            let pool = &state.0;
                            if let Ok(profile) = crate::db::player_style_service::build_player_profile(pool, &active_puuid).await {
                                cached_profile = Some(profile);
                            } else {
                                // Evitar chamadas infinitas caso o banco não tenha partidas do usuário
                                cached_profile = Some(crate::db::player_style_service::PlayerAggregateProfile {
                                    total_games: 0,
                                    win_rate: 0.0,
                                    avg_kda: 0.0,
                                    avg_cs_per_min: 0.0,
                                    avg_vision_score_per_min: 0.0,
                                    avg_deaths: 0.0,
                                    avg_damage: 0.0,
                                    style_tag: "".to_string(),
                                    style_description: "".to_string(),
                                    coaching_tips: "".to_string(),
                                    recent_matches: vec![],
                                });
                            }
                        }
                    }

                    let g_res = game_flow::get_game_state_from_conn(conn).await;
                    let elo = helpers::get_current_soloq_rank(conn).await;

                    let mut resolved_name = None;
                    let mut raw_id = 0;
                    let mut db_status = "No Pool".to_string();

                    if let Ok(game_flow::GameState::ChampSelect { phase, champion_id, role, my_team, their_team, pick_index, banned_champion_ids }) = &g_res {
                        raw_id = *champion_id;



                        // COACH LOGIC: Trigger on phase transition
                        let is_new_phase = last_phase.as_ref().map_or(true, |p| format!("{:?}", p) != format!("{:?}", phase));
                        if is_new_phase {
                            if let game_flow::ChampSelectPhase::Banning = phase {
                                // ... (ban logic)
                                let handle_clone = handle.clone();
                                let role_clone = role.clone();
                                let my_team_data = my_team.clone();
                                let elo_for_thread = elo.clone();

                                tauri::async_runtime::spawn(async move {
                                    if let Some(state) = handle_clone.try_state::<db::DbState>() {
                                        let pool = &state.0;

                                        // 1. Get Top Bans from DB
                                        if let Ok(mut top_champs) = db::coach_service::get_top_bans(pool, &elo_for_thread, &role_clone).await {
                                            // Filter out champions already hovered by my team
                                            let hovered_ids: Vec<i64> = my_team_data.iter().map(|m| m.champion_id).filter(|id| *id > 0).collect();
                                            let mut hovered_names = Vec::new();
                                            for hid in hovered_ids {
                                                if let Ok(Some(name)) = db::coach_service::get_champion_id_by_key(pool, hid as i32).await {
                                                    hovered_names.push(name);
                                                }
                                            }

                                            top_champs.retain(|c| !hovered_names.contains(&c.champion_id));
                                            println!("[Coach:BAN] Elo: {}, Role: {}, Top Meta: {:?}", elo_for_thread, role_clone, top_champs);

                                            let first_ban = top_champs.get(0);
                                            let second_ban = top_champs.get(1);

                                            let format_name = |n: &str| -> String {
                                                match n.to_lowercase().as_str() {
                                                    "missfortune" => "Miss Fortune".to_string(),
                                                    "masteryi" => "Master Yi".to_string(),
                                                    "aurelionsol" => "Aurelion Sol".to_string(),
                                                    "drmundo" => "Dr. Mundo".to_string(),
                                                    "tahmkench" => "Tahm Kench".to_string(),
                                                    "twistedfate" => "Twisted Fate".to_string(),
                                                    "xinzhao" => "Xin Zhao".to_string(),
                                                    other => {
                                                        let mut chars = other.chars();
                                                        match chars.next() {
                                                            None => String::new(),
                                                            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                                                        }
                                                    }
                                                }
                                            };

                                            let mut tip = "Bana Amumu ou Master Yi — dominantes neste ELO.".to_string();
                                            if let (Some(b1), Some(b2)) = (first_ban, second_ban) {
                                                let name1 = format_name(&b1.champion_id);
                                                let name2 = format_name(&b2.champion_id);
                                                let wr1 = b1.win_rate.map(|w| format!("{:.1}%", w * 100.0)).unwrap_or_default();
                                                let wr2 = b2.win_rate.map(|w| format!("{:.1}%", w * 100.0)).unwrap_or_default();
                                                tip = format!(
                                                    "Bana {} ({}) ou {} ({}) — dominantes neste ELO.",
                                                    name1, wr1, name2, wr2
                                                );
                                            } else if let Some(b1) = first_ban {
                                                let name1 = format_name(&b1.champion_id);
                                                let wr1 = b1.win_rate.map(|w| format!("{:.1}%", w * 100.0)).unwrap_or_default();
                                                tip = format!(
                                                    "Bana {} ({}) — campeão dominante neste ELO.",
                                                    name1, wr1
                                                );
                                            }

                                            let front_ban = match (first_ban, second_ban) {
                                                (Some(b1), Some(b2)) => format!("Sugestão: Banar {} ou {}", format_name(&b1.champion_id), format_name(&b2.champion_id)),
                                                (Some(b1), None) => format!("Sugestão: Banar {}", format_name(&b1.champion_id)),
                                                _ => "Sugestão de Banimento".to_string(),
                                            };

                                            let _ = handle_clone.emit("update-flashcard-content", json!({
                                                "title": "Coach de Seleção — Banimento",
                                                "frontText": front_ban,
                                                "backText": tip,
                                                "rarity": "epic"
                                            }));
                                        }
                                    }
                                });
                            } else if let game_flow::ChampSelectPhase::Escolhendo | game_flow::ChampSelectPhase::AguardandoPick = phase {
                                let handle_clone = handle.clone();
                                let role_clone = role.clone();
                                let my_team_data = my_team.clone();
                                let their_team_data = their_team.clone();
                                let pick_index_val = *pick_index;
                                let banned_ids = banned_champion_ids.clone();
                                let elo_for_thread = elo.clone();
                                let active_puuid_clone = s["puuid"].as_str().unwrap_or("").to_string();

                                tauri::async_runtime::spawn(async move {
                                    if let Some(state) = handle_clone.try_state::<db::DbState>() {
                                        let pool = &state.0;
                                        let elo = elo_for_thread;
                                        let puuid = active_puuid_clone;

                                        // Resolve banned names to exclude them
                                        let mut banned_names = Vec::new();
                                        for bid in banned_ids {
                                            if let Ok(Some(name)) = db::coach_service::get_champion_id_by_key(pool, bid as i32).await {
                                                banned_names.push(name);
                                            }
                                        }

                                        // Exclude the top 2 recommended bans from pick suggestions
                                        if let Ok(mut top_bans) = db::coach_service::get_top_bans(pool, &elo, &role_clone).await {
                                            let hovered_ids: Vec<i64> = my_team_data.iter().map(|m| m.champion_id).filter(|id| *id > 0).collect();
                                            let mut hovered_names = Vec::new();
                                            for hid in hovered_ids {
                                                if let Ok(Some(name)) = db::coach_service::get_champion_id_by_key(pool, hid as i32).await {
                                                    hovered_names.push(name);
                                                }
                                            }
                                            top_bans.retain(|c| !hovered_names.contains(&c.champion_id));

                                            for ban in top_bans.into_iter().take(2) {
                                                if !banned_names.contains(&ban.champion_id) {
                                                    banned_names.push(ban.champion_id);
                                                }
                                            }
                                        }

                                        // 1. Resolve champion names for teams
                                        let mut my_team_names = Vec::new();
                                        for m in my_team_data {
                                            if m.champion_id > 0 {
                                                if let Ok(Some(name)) = db::coach_service::get_champion_id_by_key(pool, m.champion_id as i32).await {
                                                    my_team_names.push(name);
                                                }
                                            }
                                        }
                                        let mut their_team_names = Vec::new();
                                        let mut opponent_name = None;
                                        for m in their_team_data {
                                            if m.champion_id > 0 {
                                                if let Ok(Some(name)) = db::coach_service::get_champion_id_by_key(pool, m.champion_id as i32).await {
                                                    if m.assigned_position == role_clone {
                                                        opponent_name = Some(name.clone());
                                                    }
                                                    their_team_names.push(name);
                                                }
                                            }
                                        }

                                        // 2. Fetch the active user's most played champions from local matches database
                                        let mut user_pool = Vec::new();
                                        if !puuid.is_empty() {
                                            user_pool = db::coach_service::get_most_played_champions(pool, &puuid, 10).await;
                                        }

                                        // Filter the user's pool to select champions viable for the active role that are NOT banned
                                        let mut viable_user_pool = Vec::new();
                                        for c in &user_pool {
                                            if db::coach_service::is_viable_for_role(c, &role_clone) && !banned_names.contains(c) {
                                                viable_user_pool.push(c.clone());
                                            }
                                        }

                                        let title;
                                        let front_text;
                                        let back_text;

                                        // Regras de escolha por ordem na seleção
                                        if pick_index_val <= 3 {
                                            // REGRA 1: Posições 1-3 — escolha cega (o inimigo ainda não revelou tudo)
                                            title = "Coach de Seleção — Escolha Cega".to_string();
                                            front_text = format!("{}ª Posição — Escolha seu Campeão de Conforto", pick_index_val);

                                            let mut meta_blinds = Vec::new();
                                            if let Ok(blinds) = db::coach_service::get_blind_picks(pool, &elo, &role_clone).await {
                                                meta_blinds = blinds.into_iter()
                                                    .filter(|c| !banned_names.contains(&c.champion_id))
                                                    .map(|c| c.champion_id)
                                                    .collect();
                                            }

                                            if !viable_user_pool.is_empty() {
                                                let c1 = &viable_user_pool[0];
                                                let name1 = helpers::format_champion_display_name(c1);
                                                if viable_user_pool.len() > 1 {
                                                    let c2 = &viable_user_pool[1];
                                                    let name2 = helpers::format_champion_display_name(c2);
                                                    back_text = format!(
                                                        "{} ou {} — seus mais jogados. Priorize conforto na escolha cega.",
                                                        name1, name2
                                                    );
                                                } else {
                                                    back_text = format!(
                                                        "{} — seu mais jogado nessa rota. Priorize conforto.",
                                                        name1
                                                    );
                                                }
                                            } else if !meta_blinds.is_empty() {
                                                let c1 = &meta_blinds[0];
                                                let name1 = helpers::format_champion_display_name(c1);
                                                if meta_blinds.len() > 1 {
                                                    let c2 = &meta_blinds[1];
                                                    let name2 = helpers::format_champion_display_name(c2);
                                                    back_text = format!(
                                                        "{} ou {} — alta taxa de vitória, seguros para escolha cega.",
                                                        name1, name2
                                                    );
                                                } else {
                                                    back_text = format!(
                                                        "{} — alta taxa de vitória, sem fraquezas óbvias.",
                                                        name1
                                                    );
                                                }
                                            } else {
                                                let default_champ = match role_clone.to_uppercase().as_str() {
                                                    "JUNGLE" => "Amumu",
                                                    "TOP" => "Garen",
                                                    "MID" => "Annie",
                                                    "ADC" => "Miss Fortune",
                                                    "SUPPORT" => "Lux",
                                                    _ => "Garen",
                                                };
                                                back_text = format!(
                                                    "{}: estável para escolha cega, sem fraquezas críticas.",
                                                    default_champ
                                                );
                                            }
                                        } else if pick_index_val == 4 {
                                            // REGRA 2: 4ª posição — você já vê parte do time inimigo
                                            title = "Coach de Seleção — 4ª Posição".to_string();
                                            front_text = "4ª Posição — Use a Informação Disponível".to_string();

                                            if let Some(ref opp) = opponent_name {
                                                let mut user_matchups = Vec::new();
                                                for champ in &viable_user_pool {
                                                    if let Ok(Some(m)) = db::coach_service::get_matchup(pool, champ, opp, Some(&elo)).await {
                                                        user_matchups.push((champ.clone(), m.win_rate.unwrap_or(0.5)));
                                                    }
                                                }
                                                user_matchups.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                                                if !user_matchups.is_empty() {
                                                    let (best_champ, wr) = &user_matchups[0];
                                                    let display_champ = helpers::format_champion_display_name(best_champ);
                                                    let display_opp = helpers::format_champion_display_name(opp);
                                                    back_text = format!(
                                                        "{} tem {:.1}% contra {} — conforto com vantagem estatística.",
                                                        display_champ, wr * 100.0, display_opp
                                                    );
                                                } else {
                                                    let mut meta_counters = Vec::new();
                                                    if let Ok(counters) = db::coach_service::get_counter_picks(pool, opp, &role_clone, Some(&elo)).await {
                                                        meta_counters = counters.into_iter()
                                                            .filter(|c| !banned_names.contains(&c.champion_id))
                                                            .collect();
                                                    }

                                                    if !meta_counters.is_empty() {
                                                        let best_meta = &meta_counters[0];
                                                        let display_champ = helpers::format_champion_display_name(&best_meta.champion_id);
                                                        let display_opp = helpers::format_champion_display_name(opp);
                                                        let wr = best_meta.win_rate.unwrap_or(0.5);
                                                        back_text = format!(
                                                            "{} tem {:.1}% contra {} — vantagem estatística confirmada.",
                                                            display_champ, wr * 100.0, display_opp
                                                        );
                                                    } else {
                                                        let display_opp = helpers::format_champion_display_name(opp);
                                                        let default_champ = match role_clone.to_uppercase().as_str() {
                                                            "JUNGLE" => "Amumu",
                                                            "TOP" => "Garen",
                                                            "MID" => "Annie",
                                                            "ADC" => "Miss Fortune",
                                                            "SUPPORT" => "Lux",
                                                            _ => "Garen",
                                                        };
                                                        back_text = format!(
                                                            "{} é escolha segura contra {} nessa rota.",
                                                            default_champ, display_opp
                                                        );
                                                    }
                                                }
                                            } else {
                                                if !viable_user_pool.is_empty() {
                                                    let best_champ = &viable_user_pool[0];
                                                    let display_champ = helpers::format_champion_display_name(best_champ);
                                                    back_text = format!(
                                                        "{}: mais jogado. Sem adversário confirmado, priorize conforto.",
                                                        display_champ
                                                    );
                                                } else {
                                                    let default_champ = match role_clone.to_uppercase().as_str() {
                                                        "JUNGLE" => "Amumu",
                                                        "TOP" => "Garen",
                                                        "MID" => "Annie",
                                                        "ADC" => "Miss Fortune",
                                                        "SUPPORT" => "Lux",
                                                        _ => "Garen",
                                                    };
                                                    back_text = format!(
                                                        "{}: escolha flexível para encaixar na composição.",
                                                        default_champ
                                                    );
                                                }
                                            }
                                        } else {
                                            // REGRA 3: Última posição (5ª) — máxima informação disponível
                                            title = "Coach de Seleção — Última Posição".to_string();
                                            front_text = "5ª Posição — Você Vê Tudo. Maximize o Counter.".to_string();

                                            if let Some(ref opp) = opponent_name {
                                                let mut user_matchups = Vec::new();
                                                for champ in &viable_user_pool {
                                                    if let Ok(Some(m)) = db::coach_service::get_matchup(pool, champ, opp, Some(&elo)).await {
                                                        user_matchups.push((champ.clone(), m.win_rate.unwrap_or(0.5)));
                                                    }
                                                }
                                                user_matchups.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                                                if !user_matchups.is_empty() && user_matchups[0].1 >= 0.50 {
                                                    let (best_champ, wr) = &user_matchups[0];
                                                    let display_champ = helpers::format_champion_display_name(best_champ);
                                                    let display_opp = helpers::format_champion_display_name(opp);
                                                    back_text = format!(
                                                        "{} tem {:.1}% contra {} — conforto e vantagem combinados.",
                                                        display_champ, wr * 100.0, display_opp
                                                    );
                                                } else {
                                                    let mut meta_counters = Vec::new();
                                                    if let Ok(counters) = db::coach_service::get_counter_picks(pool, opp, &role_clone, Some(&elo)).await {
                                                        meta_counters = counters.into_iter()
                                                            .filter(|c| !banned_names.contains(&c.champion_id))
                                                            .collect();
                                                    }

                                                    if !meta_counters.is_empty() {
                                                        let best_meta = &meta_counters[0];
                                                        let display_champ = helpers::format_champion_display_name(&best_meta.champion_id);
                                                        let display_opp = helpers::format_champion_display_name(opp);
                                                        let wr = best_meta.win_rate.unwrap_or(0.5);
                                                        back_text = format!(
                                                            "{} tem {:.1}% contra {} — use a vantagem de informação.",
                                                            display_champ, wr * 100.0, display_opp
                                                        );
                                                    } else {
                                                        let display_opp = helpers::format_champion_display_name(opp);
                                                        let default_champ = match role_clone.to_uppercase().as_str() {
                                                            "JUNGLE" => "Amumu",
                                                            "TOP" => "Garen",
                                                            "MID" => "Annie",
                                                            "ADC" => "Miss Fortune",
                                                            "SUPPORT" => "Lux",
                                                            _ => "Garen",
                                                        };
                                                        back_text = format!(
                                                            "{} contra {}: estabilidade de composição garantida.",
                                                            default_champ, display_opp
                                                        );
                                                    }
                                                }
                                            } else {
                                                if !viable_user_pool.is_empty() {
                                                    let best_champ = &viable_user_pool[0];
                                                    let display_champ = helpers::format_champion_display_name(best_champ);
                                                    back_text = format!(
                                                        "{}: maior conforto. Sem adversário, familiaridade supera estatística.",
                                                        display_champ
                                                    );
                                                } else {
                                                    let default_champ = match role_clone.to_uppercase().as_str() {
                                                        "JUNGLE" => "Amumu",
                                                        "TOP" => "Garen",
                                                        "MID" => "Annie",
                                                        "ADC" => "Miss Fortune",
                                                        "SUPPORT" => "Lux",
                                                        _ => "Garen",
                                                    };
                                                    back_text = format!(
                                                        "{}: escolha consistente para encerrar a composição.",
                                                        default_champ
                                                    );
                                                }
                                            }
                                        }

                                        // Verify log consistency: Console output prints exactly what the user sees
                                        println!("[Coach:PICK] Title: '{}' | Front: '{}' | Back: '{}'", title, front_text, back_text);

                                        // Emit the exact tip payload to the frontend
                                        let _ = handle_clone.emit("update-flashcard-content", json!({
                                            "title": title,
                                            "frontText": front_text,
                                            "backText": back_text,
                                            "rarity": "legendary"
                                        }));
                                    }
                                });
                            }
                            last_phase = Some(phase.clone());
                        }

                        if raw_id > 0 {
                            if let Some(state) = handle.try_state::<db::DbState>() {
                                let pool = &state.0;
                                match db::coach_service::get_champion_id_by_key(pool, raw_id as i32).await {
                                    Ok(Some(name)) => {
                                        resolved_name = Some(name);
                                        db_status = "Resolved".to_string();
                                    }
                                    Ok(None) => db_status = format!("ID {} not in DB", raw_id),
                                    Err(e) => db_status = format!("DB Err: {}", e),
                                }
                            }
                        } else {
                            db_status = "ID is 0".to_string();
                        }
                    } else {
                        last_phase = None;
                    }

                    let g = g_res.map(|v| serde_json::to_value(v).unwrap_or(json!("IDLE")))
                        .unwrap_or(json!("IDLE"));

                    let debug_info = json!({
                        "raw_id": raw_id,
                        "db_status": db_status
                    });

                    ("Connected".to_string(), s, g, resolved_name, debug_info, elo)
                }
                None => {
                    last_phase = None;
                    ("Disconnected".to_string(), json!({}), json!("IDLE"), None, json!({}), "GOLD".to_string())
                },
            };

            // 4. Prepare LCA data
            let mut lca_data = json!({});
            if let Ok(data) = lca_conn.get("allgamedata").await {
                lca_data = data;
            }

            // 5. Emit event to frontend
            let _ = handle.emit("lcu-update", json!({
                "status": status,
                "summoner": summoner,
                "state": game_state,
                "championName": champ_name,
                "gameData": lca_data,
                "elo": elo,
                "debug": debug_info
            }));

            // 6. When InGame: push rune overlay data from LCA active player + DB
            let game_state_str = game_state.as_str().unwrap_or("");
            let is_in_game = game_state_str == "InGame" || lca_data["activePlayer"].is_object();

            // Detecta início de nova partida (transição fora→dentro do jogo).
            // Reseta CoachState para que todas as one-shot tips (animation cancel,
            // recall timing, respawn warnings, jungle clear, etc.) disparem novamente.
            if is_in_game && !was_in_game {
                coach_state = crate::live_coach::CoachState::new();
                last_tip_emit_time = -30.0;
                let _ = handle.emit("game-started", serde_json::json!({}));
                println!("[Bridge] Nova partida detectada — CoachState resetado.");
            }

            // Fim de jogo: analisa e exibe relatório pós-partida
            if !is_in_game && was_in_game && !coach_state.db_champion_key.is_empty() {
                let h = handle.clone();
                let champ = coach_state.db_champion_key.clone();
                let role = coach_state.last_role.clone();
                let puuid_opt: Option<String> = summoner["puuid"].as_str().map(|s| s.to_string());
                tauri::async_runtime::spawn(async move {
                    // Aguarda 8s para a Riot API registrar a partida
                    tokio::time::sleep(std::time::Duration::from_secs(8)).await;
                    if let Some(db_state) = h.try_state::<crate::db::DbState>() {
                        let pool = &db_state.0;
                        if let Some(puuid) = puuid_opt {
                            match crate::riot_api::get_last_match(&h, &puuid).await {
                                Ok(match_data) => {
                                    crate::post_game::analyze_and_emit(&h, pool, &match_data, &champ, &role).await;
                                }
                                Err(e) => println!("[PostGame] Erro ao buscar última partida: {}", e),
                            }
                        }
                    }
                });
            }

            was_in_game = is_in_game;

            if is_in_game {
                in_game::handle_in_game_coaching(&handle, &lca_data, champ_name.as_deref(), &mut coach_state, &mut last_tip_emit_time, cached_profile.as_ref(), &elo).await;
            }

            tokio::time::sleep(Duration::from_secs(config::LCU_POLLING_INTERVAL_SECS)).await;
        }
    });
}
