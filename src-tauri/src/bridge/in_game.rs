use tauri::{Emitter, Manager};
use serde_json::json;
use crate::db;

pub(super) async fn handle_in_game_coaching(
    handle: &tauri::AppHandle,
    lca_data: &serde_json::Value,
    champ_name: Option<&str>,
    coach_state: &mut crate::live_coach::CoachState,
    last_tip_emit_time: &mut f64,
    cached_profile: Option<&crate::db::player_style_service::PlayerAggregateProfile>,
    elo: &str,
) {
    if let Some(state) = handle.try_state::<db::DbState>() {
        let pool = &state.0;

        // Resolve o nome do campeão ativo a partir dos dados do LCA.
        // activePlayer.championName pode estar vazio em patches recentes — usa allPlayers
        // com matching triplo: summonerName exato, base do Riot ID (sem #tag) e teamParticipantId.
        let active_summ_for_champ = lca_data["activePlayer"]["summonerName"].as_str().unwrap_or("");
        let active_base_for_champ = active_summ_for_champ.split('#').next().unwrap_or(active_summ_for_champ).to_lowercase();
        let active_team_id = lca_data["activePlayer"]["teamParticipantId"].as_i64().unwrap_or(-1);
        let lca_champ_name = lca_data["activePlayer"]["championName"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| {
                lca_data["allPlayers"].as_array()?.iter()
                    .find(|p| {
                        let p_name = p["summonerName"].as_str().unwrap_or("");
                        let p_base = p_name.split('#').next().unwrap_or(p_name).to_lowercase();
                        let p_team_id = p["teamParticipantId"].as_i64().unwrap_or(-2);
                        p_name == active_summ_for_champ
                            || (!active_base_for_champ.is_empty() && p_base == active_base_for_champ)
                            || (active_team_id >= 0 && p_team_id == active_team_id)
                    })
                    .and_then(|p| p["championName"].as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();

        // Also try champ_name from ChampSelect resolution
        let champion_for_runes = if !lca_champ_name.is_empty() {
            lca_champ_name.clone()
        } else if let Some(name) = champ_name {
            name.to_string()
        } else {
            String::new()
        };

        // Detect role from active player position
        let mut raw_pos = "MIDDLE".to_string();
        let mut team_side = "ORDER".to_string();
        let mut has_smite = false;
        let active_summ_name = lca_data["activePlayer"]["summonerName"].as_str().unwrap_or("");
        // Strip #TagLine for robust matching (Riot ID format varies between endpoints)
        let active_base = active_summ_name.split('#').next().unwrap_or(active_summ_name).to_lowercase();

        // Nome do campeão ativo — fallback de identificação quando summonerName não casa
        let active_champ_name = {
            let from_active = lca_data["activePlayer"]["championName"].as_str().unwrap_or("").to_lowercase();
            if !from_active.is_empty() { from_active }
            else { lca_champ_name.to_lowercase() }
        };

        if let Some(players) = lca_data["allPlayers"].as_array() {
            for p in players {
                let p_name = p["summonerName"].as_str().unwrap_or("");
                let p_base = p_name.split('#').next().unwrap_or(p_name).to_lowercase();
                let p_champ = p["championName"].as_str().unwrap_or("").to_lowercase();

                // Três critérios de match — o terceiro (por campeão) evita falha
                // quando Riot ID / formato de nome varia entre endpoints da LCA.
                let matches = p_name == active_summ_name
                    || (!active_base.is_empty() && p_base == active_base)
                    || (!active_champ_name.is_empty() && p_champ == active_champ_name);

                if matches {
                    raw_pos = p["position"].as_str()
                        .filter(|s| !s.is_empty())
                        .unwrap_or("MIDDLE")
                        .to_uppercase();
                    team_side = p["team"].as_str().unwrap_or("ORDER").to_uppercase();
                    has_smite = p["summonerSpells"]["summonerSpellOne"]["displayName"]
                        .as_str().map(|s| s.to_lowercase().contains("smite")).unwrap_or(false)
                        || p["summonerSpells"]["summonerSpellTwo"]["displayName"]
                        .as_str().map(|s| s.to_lowercase().contains("smite")).unwrap_or(false);
                    println!("[Bridge] Player encontrado: {} | pos={} | smite={}", p_champ, raw_pos, has_smite);
                    break;
                }
            }
        }
        let role_for_runes = match raw_pos.as_str() {
            "JUNGLE" => "JUNGLE",
            "TOP"    => "TOP",
            "BOTTOM" => "ADC",
            "UTILITY"=> "SUPPORT",
            // MIDDLE ou não detectado: usa Smite como desempate
            _ => if has_smite { "JUNGLE" } else { "MID" },
        };

        // ── Carga única de dados do banco por partida ──────────────────────────
        if !coach_state.db_loaded && !champion_for_runes.is_empty() {
            // Resolve o ID DDragon uma vez (ex: "Miss Fortune" → "MissFortune")
            // para evitar mismatch entre o nome da LCA e as chaves do banco de dados.
            coach_state.db_champion_key = db::coach_service::resolve_champion_db_id(
                pool, &champion_for_runes
            ).await;
            if coach_state.db_champion_key.is_empty() {
                coach_state.db_champion_key = champion_for_runes.clone();
            }
            println!("[DB] Champion resolvido: '{}' → '{}'", champion_for_runes, coach_state.db_champion_key);

            if let Some(meta) = db::coach_service::get_champion_meta_from_db(
                pool, &coach_state.db_champion_key, role_for_runes
            ).await {
                coach_state.db_skill_order    = meta.skill_order;
                coach_state.db_jungle_path    = meta.jungle_path;
                coach_state.db_first_item_time_ms = meta.first_item_time_ms;
                coach_state.db_starting_items = meta.starting_items;
                println!("[DB] Meta carregada: {} habilidades, jungle={:?}, item_time={:?}ms, {} itens iniciais",
                    coach_state.db_skill_order.len(),
                    coach_state.db_jungle_path,
                    coach_state.db_first_item_time_ms,
                    coach_state.db_starting_items.len());
            }

            // Detecta oponente de rota e carrega dificuldade do matchup
            if let Some(players) = lca_data["allPlayers"].as_array() {
                let my_team = players.iter()
                    .find(|p| p["summonerName"].as_str().unwrap_or("") == active_summ_name)
                    .and_then(|p| p["team"].as_str())
                    .unwrap_or("ORDER");
                let opp_champ = players.iter()
                    .find(|p| {
                        p["team"].as_str().unwrap_or("ORDER") != my_team
                            && !raw_pos.is_empty()
                            && p["position"].as_str()
                                .filter(|s| !s.is_empty())
                                .map(|s| s.to_uppercase() == raw_pos)
                                .unwrap_or(false)
                    })
                    .and_then(|p| p["championName"].as_str());
                if let Some(opp) = opp_champ {
                    // Resolve o nome DDragon do oponente (LCA retorna "Lee Sin", DB tem "LeeSin")
                    let opp_key = db::coach_service::resolve_champion_db_id(pool, opp).await;
                    let opp_resolved = if opp_key.is_empty() { opp.to_string() } else { opp_key };
                    coach_state.db_matchup_difficulty = db::coach_service::get_matchup_difficulty(
                        pool, &coach_state.db_champion_key, &opp_resolved
                    ).await;
                    if let Some(diff) = coach_state.db_matchup_difficulty {
                        println!("[DB] Matchup vs {}: dificuldade {}/10", opp_resolved, diff);
                    }
                }
            }

            coach_state.db_loaded = true;
        }

        // ── Telemetria de inimigos — roda em todo tick, independente de cooldown ──
        {
            let gt = lca_data["gameData"]["gameTime"].as_f64().unwrap_or(0.0);
            if gt > 0.0 {
                crate::live_coach::update_enemy_telemetry(coach_state, lca_data, &team_side, gt);
            }
        }

        // --- INTELLIGENT COACH IN-GAME ---
        if let Some(game_time) = lca_data["gameData"]["gameTime"].as_f64() {
            let level = lca_data["activePlayer"]["level"].as_i64().unwrap_or(1);

            let gold = lca_data["activePlayer"]["currentGold"].as_f64().unwrap_or(0.0);
            let hp_current = lca_data["activePlayer"]["championStats"]["currentHealth"].as_f64().unwrap_or(1000.0);
            let hp_max = lca_data["activePlayer"]["championStats"]["maxHealth"].as_f64().unwrap_or(1000.0);
            let health_perc = if hp_max > 0.0 { hp_current / hp_max } else { 1.0 };

            let mana_current = lca_data["activePlayer"]["championStats"]["resourceValue"].as_f64().unwrap_or(100.0);
            let mana_max = lca_data["activePlayer"]["championStats"]["resourceMax"].as_f64().unwrap_or(100.0);
            let mana_perc = if mana_max > 0.0 { mana_current / mana_max } else { 1.0 };

            // Habilidades do jogador — usadas para dismiss condition de skill level-up tips
            let ability_q = lca_data["activePlayer"]["abilities"]["Q"]["abilityLevel"].as_i64().unwrap_or(0);
            let ability_w = lca_data["activePlayer"]["abilities"]["W"]["abilityLevel"].as_i64().unwrap_or(0);
            let ability_e = lca_data["activePlayer"]["abilities"]["E"]["abilityLevel"].as_i64().unwrap_or(0);
            let ability_r = lca_data["activePlayer"]["abilities"]["R"]["abilityLevel"].as_i64().unwrap_or(0);

            // Dados do jogador na lista geral (CS e inventário) — usados no game-state-update
            let gs_player = lca_data["allPlayers"].as_array()
                .and_then(|players| players.iter().find(|p| {
                    let p_name = p["summonerName"].as_str().unwrap_or("");
                    let p_base = p_name.split('#').next().unwrap_or(p_name).to_lowercase();
                    p_name == active_summ_name || (!active_base.is_empty() && p_base == active_base)
                }));
            let gs_cs = gs_player
                .and_then(|p| p["scores"]["creepScore"].as_i64())
                .unwrap_or(0);
            let gs_inventory: Vec<i64> = gs_player
                .map(|p| p["items"].as_array()
                    .map(|items| items.iter()
                        .filter_map(|i| i["itemID"].as_i64())
                        .collect::<Vec<i64>>())
                    .unwrap_or_default())
                .unwrap_or_default();

            // Emitido a cada tick — frontend usa para verificar dismiss conditions das dicas ativas
            let _ = handle.emit("game-state-update", serde_json::json!({
                "game_time": game_time,
                "level": level,
                "gold": gold,
                "cs": gs_cs,
                "clear_step": coach_state.first_clear_step,
                "inventory": gs_inventory,
                "abilities": { "q": ability_q, "w": ability_w, "e": ability_e, "r": ability_r }
            }));

            // Guard por ciclo: garante no máximo 1 emit de update-flashcard-content por tick de 2s.
            // Sem isso, múltiplos blocos (level-up + dynamic, clear + sighting) disparam juntos.
            let mut emitted_this_cycle = false;

            // ── Skill priority — dispara UMA vez após DB carregar (5-15s de jogo) ──
            if !emitted_this_cycle && game_time >= 5.0 && game_time < 20.0
                && coach_state.db_loaded
                && !coach_state.db_skill_order.is_empty()
                && !coach_state.shown_categories.contains_key("SKILL_PRIORITY_SHOWN")
            {
                coach_state.shown_categories.insert("SKILL_PRIORITY_SHOWN".to_string(), 1);
                let order = coach_state.db_skill_order.iter().take(5).cloned().collect::<Vec<_>>();
                let order_str = order.join(" → ");
                // Identifica a habilidade a maximizar primeiro (mais frequente nos 9 primeiros níveis)
                let first_9 = &coach_state.db_skill_order[..coach_state.db_skill_order.len().min(9)];
                let q_count = first_9.iter().filter(|s| s.to_uppercase() == "Q").count();
                let w_count = first_9.iter().filter(|s| s.to_uppercase() == "W").count();
                let e_count = first_9.iter().filter(|s| s.to_uppercase() == "E").count();
                let max_skill = if q_count >= w_count && q_count >= e_count { "Q" }
                    else if w_count >= e_count { "W" } else { "E" };
                let _ = handle.emit("update-flashcard-content", serde_json::json!({
                    "title": format!("⬆️ Prioridade: Max {}", max_skill),
                    "frontText": format!("1→5: {}", order_str),
                    "backText": format!("Maximize {} primeiro. Ordem dos primeiros níveis: {}.", max_skill, order_str),
                    "rarity": "rare",
                    "dismiss": { "type": "fallback", "max_ms": 20000 }
                }));
                *last_tip_emit_time = game_time;
                emitted_this_cycle = true;
                println!("[DB] Skill priority emitida: max {} | {}", max_skill, order_str);
            }

            // ── Dica de itens iniciais (meta) — dispara uma vez nos primeiros 5s ──
            // Guard: só dispara se não houve outra dica recentemente (evita burst na entrada)
            let bypass_cooldown_ok = (game_time - *last_tip_emit_time) >= 5.0;
            if !emitted_this_cycle && game_time < 5.0 && !coach_state.db_starting_items_suggested
                && !coach_state.db_starting_items.is_empty() && bypass_cooldown_ok
            {
                coach_state.db_starting_items_suggested = true;
                // Resolve names from items table (not champion table)
                let starting_clone = coach_state.db_starting_items.clone();
                let names: Vec<String> = {
                    let mut v = Vec::new();
                    for id in &starting_clone {
                        let name_opt: Option<String> = sqlx::query_scalar(
                            "SELECT name FROM items WHERE id = ?"
                        ).bind(id.to_string()).fetch_optional(pool).await.unwrap_or(None);
                        v.push(name_opt.unwrap_or_else(|| format!("Item {}", id)));
                    }
                    v
                };
                if !names.is_empty() {
                    let _ = handle.emit("update-flashcard-content", serde_json::json!({
                        "title": "🛒 Setup Inicial do Meta",
                        "frontText": format!("Compre: {}", names.join(", ")),
                        "backText": format!("Itens iniciais recomendados pelo meta: {}.", names.join(", ")),
                        "rarity": "common",
                        "dismiss": { "type": "inventory_change" }
                    }));
                    *last_tip_emit_time = game_time;
                    emitted_this_cycle = true;
                }
            }

            // ── Alerta de timing do 1º item (meta) ───────────────────────────
            if !emitted_this_cycle {
            if let Some(first_ms) = coach_state.db_first_item_time_ms {
                let first_secs = first_ms as f64 / 1000.0;
                // Alerta 30s antes do tempo médio de recall
                // Guard: só dispara se não houve outra dica nos últimos 10s
                let item_bypass_ok = (game_time - *last_tip_emit_time) >= 10.0;
                if game_time >= first_secs - 30.0 && game_time < first_secs
                    && !coach_state.db_first_item_alerted && item_bypass_ok
                {
                    coach_state.db_first_item_alerted = true;
                    let mins = (first_secs / 60.0) as i32;
                    let secs = (first_secs % 60.0) as i32;
                    let _ = handle.emit("update-flashcard-content", serde_json::json!({
                        "title": "🔄 Recall — 1º Item em 30s",
                        "frontText": format!("Tempo médio: {}:{:02}", mins, secs),
                        "backText": format!("Prepare-se para voltar à base. Tempo médio do 1º item: {}:{:02}.", mins, secs),
                        "rarity": "rare",
                        "dismiss": { "type": "game_time_gte", "target": first_secs }
                    }));
                    *last_tip_emit_time = game_time;
                    emitted_this_cycle = true;
                }
            }
            } // !emitted_this_cycle

            // ── Alerta de matchup difícil — dispara uma vez no início ─────────
            // Guard: aguarda 10s após a última dica antes de disparar
            if !emitted_this_cycle {
            if let Some(diff) = coach_state.db_matchup_difficulty {
                let matchup_bypass_ok = (game_time - *last_tip_emit_time) >= 10.0;
                if diff >= 7 && game_time >= 10.0 && game_time < 30.0 && matchup_bypass_ok {
                    let shown = coach_state.shown_categories.entry("DB_MATCHUP_WARN".to_string()).or_insert(0);
                    if *shown == 0 {
                        *shown += 1;
                        let msg = if diff >= 9 {
                            "Counter direto — evite trocas, jogue pelo farm.".to_string()
                        } else {
                            "Matchup desfavorável — jogue curto e espere ganks.".to_string()
                        };
                        let _ = handle.emit("update-flashcard-content", serde_json::json!({
                            "title": format!("⚠️ Matchup Difícil ({}/10)", diff),
                            "frontText": "Desvantagem no confronto",
                            "backText": msg,
                            "rarity": "epic",
                            "dismiss": { "type": "fallback", "max_ms": 30000 }
                        }));
                        *last_tip_emit_time = game_time;
                        emitted_this_cycle = true;
                    }
                }
            }
            } // !emitted_this_cycle

            // Guia de limpeza da selva — roda fora do cooldown global (cooldown próprio de 8s).
            // Cada etapa do clear usa vida% e mana% reais para coaching contextual.
            let clear_tip_emitted = if !emitted_this_cycle {
                if let Some((clear_title, clear_text)) = crate::live_coach::get_jungle_clear_tip(coach_state, lca_data, game_time, role_for_runes) {
                    println!("[LiveCoach] Clear guide -> {}", clear_title);
                    // Step 1 = rota inicial (fixo — fica até o próximo step a substituir)
                    // Step 2+ = campo específico (fecha quando o campo é morto: clear_step > N)
                    let step_now = coach_state.first_clear_step;
                    let jungle_dismiss = if step_now <= 1 {
                        serde_json::json!({ "type": "fixed" })
                    } else {
                        serde_json::json!({ "type": "clear_step_gt", "target": step_now })
                    };
                    let _ = handle.emit("update-flashcard-content", serde_json::json!({
                        "title": clear_title,
                        "frontText": format!("Selva: {}", role_for_runes),
                        "backText": clear_text,
                        "rarity": "epic",
                        "dismiss": jungle_dismiss,
                        "tipCategory": "jungle_clear"
                    }));
                    emitted_this_cycle = true;
                    true
                } else {
                    false
                }
            } else {
                false
            };

            // Enquanto o jungle não recebeu a rota (first_clear_step == 0), nenhuma
            // outra tip pode disparar — o clear é a prioridade absoluta do jogo inteiro.
            let jungle_clear_pending = role_for_runes == "JUNGLE" && coach_state.first_clear_step == 0;

            // Level-up tip: roda fora do cooldown global — é time-sensitive.
            // Bloqueada para jungle até a rota ser comunicada.
            if !emitted_this_cycle && !clear_tip_emitted && !jungle_clear_pending {
            if let Some((lu_title, lu_text)) = crate::live_coach::get_skill_levelup_tip(coach_state, lca_data, game_time) {
                println!("[LiveCoach] Skill level-up -> {}", lu_title);
                let _ = handle.emit("update-flashcard-content", serde_json::json!({
                    "title": lu_title,
                    "frontText": format!("Skill Up: {}", role_for_runes),
                    "backText": lu_text,
                    "rarity": "legendary",
                    "dismiss": {
                        "type": "skill_leveled",
                        "snapshot": { "q": ability_q, "w": ability_w, "e": ability_e, "r": ability_r }
                    }
                }));
                emitted_this_cycle = true;
                // Não atualiza last_tip_emit_time intencionalmente — level-up é time-sensitive
                // e não deve bloquear a próxima dica de coaching
            }
            }

            // ── Alerta de inimigo avistado (bypass do cooldown global — time-sensitive) ──
            if !emitted_this_cycle && !jungle_clear_pending {
                if let Some((sighting_champ, sighting_role, fog_secs)) = coach_state.recent_enemy_sighting.take() {
                    if game_time > 90.0 && (game_time - coach_state.last_ward_sighting_alert_time) >= 20.0 {
                        coach_state.last_ward_sighting_alert_time = game_time;
                        let role_label = match sighting_role.as_str() {
                            "JUNGLE" => "caçador",
                            "MID" => "meio",
                            "ADC" => "atirador",
                            "SUPPORT" => "suporte",
                            "TOP" => "topo",
                            _ => "inimigo",
                        };
                        let _ = handle.emit("update-flashcard-content", serde_json::json!({
                            "title": "👁️ Inimigo Revelado",
                            "frontText": format!("{} ({}) visto no mapa", sighting_champ, role_label),
                            "backText": format!("{} sumiu {:.0}s — reavalie antes de avançar.", sighting_champ, fog_secs),
                            "rarity": "epic",
                            "dismiss": { "type": "game_time_gte", "target": game_time + 18.0 }
                        }));
                        emitted_this_cycle = true;
                        println!("[Sighting] {} ({}) após {:.0}s de névoa", sighting_champ, sighting_role, fog_secs);
                    }
                }
            }

            // Global cooldown: max one coaching tip per 25 seconds to avoid flooding voice + flashcard queue.
            // Danger alerts (gank/missing) bypass this via high priority in the frontend.
            let tip_cooldown_ok = (game_time - *last_tip_emit_time) >= 25.0;

            let mut triggered_tip = false;
            if !emitted_this_cycle && tip_cooldown_ok && !jungle_clear_pending {
                if let Some((tip_title, tip_text)) = crate::live_coach::get_dynamic_coaching_tip(coach_state, lca_data, role_for_runes, cached_profile) {
                    println!("[LiveCoach] New Strategy Phase! -> {}", tip_title);
                    let _ = handle.emit("update-flashcard-content", serde_json::json!({
                        "title": tip_title,
                        "frontText": format!("Macro/Micro: {}", role_for_runes),
                        "backText": tip_text,
                        "rarity": "mythic",
                        "dismiss": { "type": "fallback", "max_ms": 30000 }
                    }));
                    *last_tip_emit_time = game_time;
                    emitted_this_cycle = true;
                    triggered_tip = true;
                }

                // Micro coaching por role (usa dados reais da API)
                if !triggered_tip {
                    if let Some((micro_title, micro_text)) = crate::live_coach::get_role_micro_tip(coach_state, lca_data, role_for_runes, game_time, cached_profile) {
                        println!("[LiveCoach] Micro tip -> {}", micro_title);
                        let _ = handle.emit("update-flashcard-content", serde_json::json!({
                            "title": micro_title,
                            "frontText": format!("Micro: {}", role_for_runes),
                            "backText": micro_text,
                            "rarity": "epic",
                            "dismiss": { "type": "fallback", "max_ms": 30000 }
                        }));
                        *last_tip_emit_time = game_time;
                        emitted_this_cycle = true;
                        triggered_tip = true;
                    }
                }
            }

            // Sistema de Itens & Receitas (Coach de Compras)
            if !emitted_this_cycle && !triggered_tip && (game_time - coach_state.last_purchase_alert_time) >= 30.0 {
                let mut inventory_items: Vec<i64> = Vec::new();
                if let Some(players) = lca_data["allPlayers"].as_array() {
                    for p in players {
                        if p["summonerName"].as_str().unwrap_or("") == active_summ_name {
                            if let Some(items) = p["items"].as_array() {
                                for item in items {
                                    if let Some(item_id) = item["itemID"].as_i64() {
                                        inventory_items.push(item_id);
                                    }
                                }
                            }
                            break;
                        }
                    }
                }

                if let Ok(Some(item_alert)) = crate::db::coach_service::get_next_item_purchase_alert(pool, &champion_for_runes, role_for_runes, gold, inventory_items).await {
                    let _ = handle.emit("update-flashcard-content", serde_json::json!({
                        "title": "\u{1f6d2} Compra Disponível",
                        "frontText": format!("Ouro Atual: {}g", gold as i32),
                        "backText": item_alert,
                        "rarity": "legendary",
                        "dismiss": { "type": "fallback", "max_ms": 25000 }
                    }));
                    coach_state.last_purchase_alert_time = game_time;
                    coach_state.active_recommendation = Some(crate::live_coach::ActiveRec::RecallGold);
                }
            }
        }
        // ── Ward Map & Alertas Pré-Objetivo ──────────────────────────────────
        if let Some(game_time) = lca_data["gameData"]["gameTime"].as_f64() {
            let phase = if game_time < 600.0 { "early" }
                        else if game_time < 1200.0 { "mid" }
                        else { "late" };
            let my_team_side = if lca_data["allPlayers"].as_array()
                .and_then(|arr| arr.iter().find(|p| {
                    p["summonerName"].as_str().unwrap_or("") == active_summ_name
                }))
                .and_then(|p| p["team"].as_str())
                .unwrap_or("ORDER") == "ORDER" { "blue" } else { "red" };

            // ── 1. Atualiza spawn times a partir dos eventos do LCA ───────────
            if let Some(events) = lca_data["events"]["Events"].as_array() {
                for event in events {
                    let name = event["EventName"].as_str().unwrap_or("");
                    let time = event["EventTime"].as_f64().unwrap_or(0.0);
                    match name {
                        "DragonKill" => {
                            let next = time + 300.0; // 5 min respawn
                            if next > coach_state.dragon_next_spawn {
                                coach_state.dragon_next_spawn = next;
                                println!("[Objective] Dragão morto em {:.0}s → próximo em {:.0}s", time, next);
                            }
                        }
                        "BaronKill" => {
                            let next = time + 360.0; // 6 min respawn
                            if next > coach_state.baron_next_spawn {
                                coach_state.baron_next_spawn = next;
                                println!("[Objective] Barão morto em {:.0}s → próximo em {:.0}s", time, next);
                            }
                        }
                        "HeraldKill" => {
                            coach_state.herald_done = true;
                        }
                        _ => {}
                    }
                }
            }
            // Herald desaparece definitivamente após 13:45 (825s)
            if game_time > 825.0 { coach_state.herald_done = true; }

            // ── 2. Alerta de wards 40s antes de cada objetivo ────────────────
            // Bounding boxes (coordenadas de jogo 0–15000):
            //   Dragon pit   ≈ (9800, 4400) → box (7000–13000, 2000–7000)
            //   Baron/Herald ≈ (4900, 10400)→ box (2000–8000, 8000–14000)
            //   Scuttle bot  ≈ (11000, 5000)→ box (8500–14000, 3000–7000)
            //   Scuttle top  ≈ (4200, 8000) → box (2000–7000, 6000–10500)
            struct ObjAlert {
                key: &'static str,
                emoji: &'static str,
                name_pt: &'static str,
                spawn: f64,
                x_min: i64, x_max: i64,
                y_min: i64, y_max: i64,
            }

            // Scuttle: fixed windows (3:30 + cada 2:30 até 15min)
            // Como não temos evento de kill do aronguejo, usamos tempos fixos aproximados
            let scuttle_windows: &[f64] = &[210.0, 360.0, 510.0, 660.0, 810.0];
            let scuttle_spawn = scuttle_windows.iter()
                .find(|&&t| t > game_time + 5.0)
                .copied()
                .unwrap_or(f64::MAX);

            let objectives = [
                ObjAlert { key: "dragon", emoji: "🐉", name_pt: "Dragão",
                    spawn: coach_state.dragon_next_spawn,
                    x_min: 7000, x_max: 13000, y_min: 2000, y_max: 7000 },
                ObjAlert { key: "baron", emoji: "💜", name_pt: "Barão",
                    spawn: coach_state.baron_next_spawn,
                    x_min: 2000, x_max: 8000, y_min: 8000, y_max: 14000 },
                ObjAlert { key: "herald", emoji: "🔮", name_pt: "Arauto",
                    spawn: if coach_state.herald_done { f64::MAX } else { 840.0 }, // 14:00 — Season 15
                    x_min: 2000, x_max: 8000, y_min: 8000, y_max: 14000 },
                ObjAlert { key: "scuttle", emoji: "🦀", name_pt: "Aronguejo",
                    spawn: scuttle_spawn,
                    x_min: 7000, x_max: 14000, y_min: 2000, y_max: 8000 },
            ];

            // Usa o ID DDragon resolvido para queries (fallback para nome LCA se não resolvido)
            let db_key = if !coach_state.db_champion_key.is_empty() {
                &coach_state.db_champion_key
            } else {
                &champion_for_runes
            };

            for obj in &objectives {
                if obj.spawn == f64::MAX || db_key.is_empty() { continue; }
                let secs_left = obj.spawn - game_time;
                if secs_left > 40.0 || secs_left < 0.0 { continue; }

                let alert_key = format!("{}:{:.0}", obj.key, obj.spawn);
                if coach_state.objective_ward_alerts.contains(&alert_key) { continue; }
                coach_state.objective_ward_alerts.insert(alert_key);

                let wards = db::coach_service::get_ward_suggestions_for_objective(
                    pool, db_key, role_for_runes,
                    obj.spawn, obj.x_min, obj.x_max, obj.y_min, obj.y_max,
                ).await;

                if !wards.is_empty() {
                    coach_state.last_ward_map_time = game_time;
                    let _ = handle.emit("update-ward-map", serde_json::json!({
                        "champion": champion_for_runes,
                        "role": role_for_runes,
                        "game_time": game_time,
                        "phase": phase,
                        "team_side": my_team_side,
                        "wards": wards,
                        "objective": obj.name_pt,
                        "objective_emoji": obj.emoji,
                        "seconds_to_spawn": secs_left.round() as i64,
                    }));
                    println!("[WardMap:Obj] {} em {:.0}s — {} pontos emitidos",
                        obj.name_pt, secs_left, wards.len());
                }
            }

            // ── 3. Ward map genérico a cada 3 minutos (sem contexto de objetivo) ──
            let ward_cooldown = 180.0;
            let ward_due = game_time >= 60.0
                && (game_time - coach_state.last_ward_map_time) >= ward_cooldown
                && !db_key.is_empty();

            if ward_due {
                let wards = db::coach_service::get_ward_suggestions(
                    pool, db_key, role_for_runes, game_time
                ).await;

                if !wards.is_empty() {
                    coach_state.last_ward_map_time = game_time;
                    let _ = handle.emit("update-ward-map", serde_json::json!({
                        "champion": champion_for_runes,
                        "role": role_for_runes,
                        "game_time": game_time,
                        "phase": phase,
                        "team_side": my_team_side,
                        "wards": wards,
                    }));
                    println!("[WardMap] {} pontos emitidos para {}/{} em {:.0}s",
                        wards.len(), champion_for_runes, role_for_runes, game_time);
                }
            }
        }
        // ---------------------------------

        if !champion_for_runes.is_empty() {
            let rune_result = db::coach_service::get_rune_overlay_data_command_internal(
                pool,
                champion_for_runes,
                Some(role_for_runes.to_string()),
                None,
            ).await;

            if let Ok(rune_data) = rune_result {
                let _ = handle.emit("update-rune-overlay-content", json!({
                    "champion_name": rune_data.champion_name,
                    "primary_tree_id": rune_data.primary_tree_id,
                    "primary_tree_name": rune_data.primary_tree_name,
                    "secondary_tree_id": rune_data.secondary_tree_id,
                    "secondary_tree_name": rune_data.secondary_tree_name,
                    "keystone_id": rune_data.keystone_id,
                    "keystone_name": rune_data.keystone_name,
                    "runes": rune_data.runes,
                    "shards": rune_data.shards,
                    "summoner_spells": rune_data.summoner_spells
                }));
            }
        }
    }
}
