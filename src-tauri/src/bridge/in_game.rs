use tauri::{Emitter, Manager};
use serde_json::json;
use crate::db;

// Posições estáticas de ward de alto nível para quando o DB não tem dados do campeão/role.
// Coordenadas no espaço de jogo LoL (0–15000). Base azul = canto inferior-esquerdo.
fn static_wards_for_objective(objective_key: &str, team_side: &str) -> Vec<crate::db::coach_service::WardPoint> {
    use crate::db::coach_service::WardPoint;
    let is_red = team_side == "red";
    // 2 wards de sentinela + 1 pink (posição de controle dentro do pit).
    // Mesmas coordenadas absolutas para ambos os lados (objetivos ficam no mesmo ponto do mapa),
    // mas a ORDEM de prioridade muda: cada time prioriza a entrada do seu lado primeiro.
    let coords: Vec<(i64, i64, &str)> = match objective_key {
        "dragon" => {
            if !is_red {
                vec![
                    (10200, 5800, "ward"),  // entrada aliada (norte/azul) — abate de gank
                    (11000, 3900, "ward"),  // entrada inimiga (sul/vermelho)
                    (9800,  4400, "pink"),  // dentro do pit
                ]
            } else {
                vec![
                    (11000, 3900, "ward"),  // entrada aliada (sul/vermelho) — prioridade red
                    (10200, 5800, "ward"),  // entrada inimiga (norte/azul) — nega visão
                    (9800,  4400, "pink"),  // dentro do pit
                ]
            }
        }
        "baron" | "herald" => {
            if !is_red {
                vec![
                    (4200, 11800, "ward"),  // tri-bush entrada aliada (azul)
                    (5800, 10600, "ward"),  // entrada inimiga (vermelho/leste)
                    (4951, 10440, "pink"),  // dentro do pit
                ]
            } else {
                vec![
                    (5800, 10600, "ward"),  // entrada aliada (vermelho/leste) — prioridade red
                    (4200, 11800, "ward"),  // tri-bush entrada inimiga (azul)
                    (4951, 10440, "pink"),  // dentro do pit
                ]
            }
        }
        "scuttle" => vec![
            // Aronguejo: acesso simétrico para ambos os lados
            (9200, 5600, "ward"),
            (6400, 9300, "ward"),
            (8000, 6000, "pink"),
        ],
        _ => vec![],
    };
    coords.into_iter().enumerate()
        .map(|(i, (x, y, t))| WardPoint { x, y, priority: i as i32 + 1, ward_type: t.to_string() })
        .collect()
}

fn static_wards_generic(role: &str, team_side: &str) -> Vec<crate::db::coach_service::WardPoint> {
    use crate::db::coach_service::WardPoint;
    // 2 wards + 1 pink por role (suporte tem 2 pinks — cobre mais pontos de visão).
    let coords: Vec<(i64, i64, &str)> = match role {
        "TOP" => {
            if team_side == "blue" {
                vec![(2600, 12500, "ward"), (3800, 11200, "ward"), (1800, 13200, "pink")]
            } else {
                vec![(12400, 2500, "ward"), (11200, 3800, "ward"), (13200, 1800, "pink")]
            }
        }
        "JUNGLE" => {
            if team_side == "blue" {
                vec![(7500, 11400, "ward"), (9200, 5800, "ward"), (4951, 10440, "pink")]
            } else {
                vec![(7500, 3600, "ward"), (5800, 9200, "ward"), (9866, 4414, "pink")]
            }
        }
        "MID" => {
            if team_side == "blue" {
                vec![(8800, 6200, "ward"), (6200, 8800, "ward"), (9500, 4800, "pink")]
            } else {
                vec![(6200, 8800, "ward"), (8800, 6200, "ward"), (5500, 10200, "pink")]
            }
        }
        "ADC" => {
            if team_side == "blue" {
                vec![(10200, 5200, "ward"), (11500, 1800, "ward"), (9800, 4400, "pink")]
            } else {
                vec![(4800, 9800, "ward"), (3500, 13200, "ward"), (5200, 10600, "pink")]
            }
        }
        "SUPPORT" => {
            if team_side == "blue" {
                // 2 wards de rota + 2 pinks: pit do dragão e pixel bush do bot
                vec![(10200, 4200, "ward"), (11800, 1500, "ward"), (9800, 4000, "pink"), (12000, 2800, "pink")]
            } else {
                // Lado vermelho: baron area + pixel bush do top bot
                vec![(4800, 10800, "ward"), (3200, 13500, "ward"), (5200, 11000, "pink"), (3000, 12200, "pink")]
            }
        }
        _ => vec![(7500, 9000, "ward"), (8500, 6000, "ward"), (8000, 7500, "pink")],
    };
    coords.into_iter().enumerate()
        .map(|(i, (x, y, t))| WardPoint { x, y, priority: i as i32 + 1, ward_type: t.to_string() })
        .collect()
}

/// Pontos de ward para o momento de push após matar o inimigo de rota.
/// Foco em cobrir as entradas de gank enquanto o jogador avança na rota.
fn push_wards_for_role(role: &str, team_side: &str) -> Vec<crate::db::coach_service::WardPoint> {
    use crate::db::coach_service::WardPoint;
    let coords: Vec<(i64, i64, &str)> = match role {
        "TOP" => {
            if team_side == "blue" {
                // Empurrando para torre inimiga do topo (lado azul): cobre entrada do JG vindo do topo
                vec![(3000, 12800, "ward"), (2300, 11000, "ward"), (1800, 13000, "pink")]
            } else {
                vec![(12000, 2200, "ward"), (13000, 4000, "ward"), (13200, 1800, "pink")]
            }
        }
        "MID" => {
            if team_side == "blue" {
                // Empurrando mid: cobre JG entrando pelo bot e top
                vec![(8800, 6200, "ward"), (6200, 8800, "ward"), (9500, 4800, "pink")]
            } else {
                vec![(6200, 8800, "ward"), (8800, 6200, "ward"), (5500, 10200, "pink")]
            }
        }
        "ADC" => {
            if team_side == "blue" {
                vec![(10200, 5200, "ward"), (11800, 1500, "ward"), (9800, 4000, "pink")]
            } else {
                vec![(4800, 9800, "ward"), (3200, 13500, "ward"), (5200, 11000, "pink")]
            }
        }
        "SUPPORT" => {
            if team_side == "blue" {
                // Empurrando bot: cobre dragon + pixel bush — suporte planta 2 pinks
                vec![(10200, 5200, "ward"), (11800, 1500, "ward"), (9800, 4000, "pink"), (11200, 3200, "pink")]
            } else {
                vec![(4800, 9800, "ward"), (3200, 13500, "ward"), (5200, 11000, "pink"), (3800, 11800, "pink")]
            }
        }
        "JUNGLE" => {
            if team_side == "blue" {
                vec![(9200, 5800, "ward"), (5800, 9200, "ward"), (4951, 10440, "pink")]
            } else {
                vec![(5800, 9200, "ward"), (9200, 5800, "ward"), (9866, 4414, "pink")]
            }
        }
        _ => vec![(7500, 9000, "ward"), (8500, 6000, "ward"), (8000, 7500, "pink")],
    };
    coords.into_iter().enumerate()
        .map(|(i, (x, y, t))| WardPoint { x, y, priority: i as i32 + 1, ward_type: t.to_string() })
        .collect()
}

pub(super) async fn handle_in_game_coaching(
    handle: &tauri::AppHandle,
    lca_data: &serde_json::Value,
    champ_name: Option<&str>,
    coach_state: &mut crate::live_coach::CoachState,
    last_tip_emit_time: &mut f64,
    cached_profile: Option<&crate::db::player_style_service::PlayerAggregateProfile>,
    _elo: &str,
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
            coach_state.last_role = role_for_runes.to_string();
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
            let _health_perc = if hp_max > 0.0 { hp_current / hp_max } else { 1.0 };

            let mana_current = lca_data["activePlayer"]["championStats"]["resourceValue"].as_f64().unwrap_or(100.0);
            let mana_max = lca_data["activePlayer"]["championStats"]["resourceMax"].as_f64().unwrap_or(100.0);
            let _mana_perc = if mana_max > 0.0 { mana_current / mana_max } else { 1.0 };

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
                    "category": "skill",
                    "dismiss": { "type": "fallback", "max_ms": 20000 }
                }));
                *last_tip_emit_time = game_time;
                emitted_this_cycle = true;
                println!("[DB] Skill priority emitida: max {} | {}", max_skill, order_str);
            }

            // ── Groq: trigger de início de partida (15–20s) ──────────────────────
            // Dispara uma vez por partida para que o Groq forneça contexto de early game.
            if !coach_state.groq_game_start_triggered && game_time >= 15.0 && game_time < 30.0 {
                coach_state.groq_game_start_triggered = true;
                coach_state.last_groq_trigger_time = game_time;
                let _ = handle.emit("request-groq-tip", serde_json::json!({}));
                println!("[Groq] Trigger de início de partida em {:.0}s", game_time);
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
                        // Ignora itens sem nome no banco — evita falar o ID numérico no TTS
                        if let Some(name) = name_opt {
                            v.push(name);
                        }
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
                    "frontText": format!("Nível {}", level),
                    "backText": lu_text,
                    "rarity": "common",
                    "category": "skill",
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

            // ── Tip de evento (morte/kill/ace) — gerada na tick anterior pelo event loop ──
            // Tem prioridade sobre sighting: morte e ace são mais urgentes.
            if !emitted_this_cycle && !jungle_clear_pending {
                if let Some((ev_title, ev_text)) = coach_state.pending_event_tip.take() {
                    let _ = handle.emit("update-flashcard-content", serde_json::json!({
                        "title": ev_title,
                        "frontText": ev_text,
                        "backText": ev_text,
                        "rarity": "epic",
                        "dismiss": { "type": "fallback", "max_ms": 15000 }
                    }));
                    emitted_this_cycle = true;
                }
            }

            // ── Alerta de inimigo avistado (bypass do cooldown global — time-sensitive) ──
            if !emitted_this_cycle && !jungle_clear_pending {
                if let Some((sighting_champ, sighting_role, fog_secs)) = coach_state.recent_enemy_sighting.take() {
                    if game_time > 90.0 && (game_time - coach_state.last_ward_sighting_alert_time) >= 20.0 {
                        coach_state.last_ward_sighting_alert_time = game_time;

                        // Detecta zona real via análise de pixels do minimapa (~1-3ms em thread bloqueante).
                        // Fallback para zona derivada do role se a captura falhar ou não achar clusters.
                        let role_for_capture = sighting_role.clone();
                        let detected_zone = tokio::task::spawn_blocking(move || {
                            crate::minimap::find_enemy_zone(&role_for_capture)
                        }).await.ok().flatten();

                        let location = detected_zone.as_deref().unwrap_or_else(|| {
                            match sighting_role.as_str() {
                                "JUNGLE"           => "no rio",
                                "TOP"              => "no top",
                                "MID"              => "no meio",
                                "ADC" | "SUPPORT"  => "no bot",
                                _                  => "no mapa",
                            }
                        });

                        // Filtra alertas de inimigos voltando para a própria rota:
                        // Se um laner aparece na zona esperada da role dele, provavelmente
                        // foi apenas recall/visão de minions — não é uma rotação relevante.
                        // Junglers (sem zona fixa) e inimigos em zonas inesperadas sempre alertam.
                        let enemy_expected_zone = match sighting_role.as_str() {
                            "TOP"             => "no top",
                            "MID"             => "no meio",
                            "ADC" | "SUPPORT" => "no bot",
                            _                 => "", // JUNGLE não tem zona fixa
                        };
                        let returning_to_own_lane = !enemy_expected_zone.is_empty()
                            && location == enemy_expected_zone;

                        if returning_to_own_lane {
                            let src = if detected_zone.is_some() { "minimapa" } else { "role" };
                            println!("[Sighting] {} voltou à própria rota ({}) via {} — ignorado", sighting_champ, location, src);
                        } else {
                            let msg = format!("{} visto {}", sighting_champ, location);
                            let _ = handle.emit("update-flashcard-content", serde_json::json!({
                                "title": "👁️ Inimigo Avistado",
                                "frontText": msg,
                                "backText": msg,
                                "rarity": "epic",
                                "dismiss": { "type": "game_time_gte", "target": game_time + 18.0 }
                            }));
                            emitted_this_cycle = true;
                            let src = if detected_zone.is_some() { "minimapa" } else { "role" };
                            println!("[Sighting] {} visto {} (via {}, névoa {:.0}s)", sighting_champ, location, src, fog_secs);
                        }
                    }
                }
            }

            // ── Oportunidade de objetivo (2+ inimigos mortos) ────────────────────
            // Usa isDead do LCA (disponível por tick) — sem OCR necessário.
            // Prioridade: Barão > Dragão > Arauto. Só alerta se o objetivo estiver vivo.
            if !emitted_this_cycle && !jungle_clear_pending {
                let dead_count = lca_data["allPlayers"].as_array()
                    .map(|players| {
                        players.iter()
                            .filter(|p| {
                                let p_team = p["team"].as_str().unwrap_or("ORDER").to_uppercase();
                                p_team != team_side && p["isDead"].as_bool().unwrap_or(false)
                            })
                            .count()
                    })
                    .unwrap_or(0);

                if dead_count >= 2
                    && (game_time - coach_state.last_objective_opportunity_time) >= 60.0
                {
                    let dragon_alive = game_time >= coach_state.dragon_next_spawn;
                    let baron_alive  = game_time >= coach_state.baron_next_spawn;
                    let herald_alive = !coach_state.herald_done
                        && game_time >= 480.0  // 8:00 — primeiro spawn do Arauto
                        && game_time < 840.0;  // 14:00 — despawn

                    let (obj_name, obj_emoji) = if baron_alive {
                        ("Barão", "💜")
                    } else if dragon_alive {
                        ("Dragão", "🐉")
                    } else if herald_alive {
                        ("Arauto", "🔮")
                    } else {
                        ("", "")
                    };

                    if !obj_name.is_empty() {
                        coach_state.last_objective_opportunity_time = game_time;
                        let front_msg = format!("{} inimigos mortos — {} disponível!", dead_count, obj_name);
                        let back_msg = format!(
                            "{} inimigos caíram. Aproveite para garantir o {}.",
                            dead_count, obj_name
                        );
                        let _ = handle.emit("update-flashcard-content", serde_json::json!({
                            "title": format!("{} Oportunidade!", obj_emoji),
                            "frontText": front_msg,
                            "backText": back_msg,
                            "rarity": "legendary",
                            "dismiss": { "type": "fallback", "max_ms": 20000 }
                        }));
                        emitted_this_cycle = true;
                        println!("[Objective] {} inimigos mortos → {}", dead_count, obj_name);
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
            // Reutiliza team_side já computado com matching triplo (nome/base/campeão) — muito mais
            // robusto que o exact match que existia aqui antes (defaultava para blue incorretamente).
            let my_team_side = if team_side == "ORDER" { "blue" } else { "red" };

            // Inventário do jogador — necessário para filtrar pink ward
            let gs_inventory: Vec<i64> = lca_data["allPlayers"].as_array()
                .and_then(|players| players.iter().find(|p| {
                    let p_name = p["summonerName"].as_str().unwrap_or("");
                    let p_base = p_name.split('#').next().unwrap_or(p_name).to_lowercase();
                    p_name == active_summ_name || (!active_base.is_empty() && p_base == active_base)
                }))
                .map(|p| p["items"].as_array()
                    .map(|items| items.iter()
                        .filter_map(|i| i["itemID"].as_i64())
                        .collect::<Vec<i64>>())
                    .unwrap_or_default())
                .unwrap_or_default();

            // ── 1. Atualiza spawn times + detecta kill/death/ace para coaching ──
            if let Some(events) = lca_data["events"]["Events"].as_array() {
                // Passagem 1: spawn times (idempotente — pode re-processar sem problema)
                for event in events.iter() {
                    let name = event["EventName"].as_str().unwrap_or("");
                    let time = event["EventTime"].as_f64().unwrap_or(0.0);
                    match name {
                        "DragonKill" => {
                            let next = time + 300.0;
                            if next > coach_state.dragon_next_spawn {
                                coach_state.dragon_next_spawn = next;
                                println!("[Objective] Dragão morto em {:.0}s → próximo em {:.0}s", time, next);
                            }
                        }
                        "BaronKill" => {
                            let next = time + 360.0;
                            if next > coach_state.baron_next_spawn {
                                coach_state.baron_next_spawn = next;
                                println!("[Objective] Barão morto em {:.0}s → próximo em {:.0}s", time, next);
                            }
                        }
                        "HeraldKill" => { coach_state.herald_done = true; }
                        _ => {}
                    }
                }

                // Passagem 2: novos eventos de kill/death/ace (processados uma única vez)
                let total_events = events.len();
                if total_events > coach_state.last_processed_event_count {
                    for event in events[coach_state.last_processed_event_count..].iter() {
                        let name = event["EventName"].as_str().unwrap_or("");
                        let event_time = event["EventTime"].as_f64().unwrap_or(0.0);
                        match name {
                            "ChampionKill" if event_time > 60.0 => {
                                let victim = event["VictimName"].as_str().unwrap_or("");
                                let killer = event["KillerName"].as_str().unwrap_or("");
                                let vbase  = victim.split('#').next().unwrap_or(victim).to_lowercase();
                                let kbase  = killer.split('#').next().unwrap_or(killer).to_lowercase();

                                let player_died = victim == active_summ_name
                                    || (!active_base.is_empty() && vbase == active_base)
                                    || (!active_champ_name.is_empty() && victim.to_lowercase() == active_champ_name);
                                let player_killed = killer == active_summ_name
                                    || (!active_base.is_empty() && kbase == active_base);

                                if player_died {
                                    let tip = match role_for_runes {
                                        "TOP"     => "Congele perto da sua torre e minimize risco. Se foi gank, ward o rio antes de avançar.",
                                        "MID"     => "Jogue curto e ward as entradas do rio. Não avance sem visão do JG.",
                                        "ADC"     => "Fique atrás do suporte nas próximas trocas. Sobreviver vale mais que o abate.",
                                        "SUPPORT" => "Proteja o atirador — posicione entre ele e o perigo.",
                                        "JUNGLE"  => "Limpe o lado oposto ao JG inimigo. Evite re-mortes forçando ganks.",
                                        _         => "Ajuste o posicionamento e evite o mesmo erro.",
                                    };
                                    // Morte tem prioridade: sobrescreve kill tip anterior
                                    coach_state.pending_event_tip = Some((
                                        "💀 Depois da Morte".to_string(),
                                        tip.to_string(),
                                    ));
                                    // Groq: solicita dica contextual ao morrer (cooldown 2 min)
                                    let groq_ok = (game_time - coach_state.last_groq_trigger_time) >= 120.0;
                                    if groq_ok {
                                        coach_state.last_groq_trigger_time = game_time;
                                        let _ = handle.emit("request-groq-tip", serde_json::json!({}));
                                        println!("[Groq] Trigger por morte em {:.0}s", game_time);
                                    }
                                    println!("[Event] Morte detectada — tip de ajuste de posicionamento");
                                } else if player_killed && coach_state.pending_event_tip.is_none() {
                                    // Kill só seta se não há morte pendente
                                    coach_state.pending_event_tip = Some((
                                        "🔥 Abate — Converta Agora".to_string(),
                                        "Empurre a onda e converta em torre ou objetivo próximo antes que eles respawnem.".to_string(),
                                    ));
                                    println!("[Event] Kill confirmado — tip de conversão");

                                    // Push ward map — emite posições de ward para janela de push
                                    let push_cooldown_ok = (game_time - coach_state.last_push_ward_time) >= 60.0;
                                    if push_cooldown_ok && game_time < 1200.0 {
                                        let pink_count = gs_inventory.iter().filter(|&&id| id == 2055).count();
                                        let mut push_wards = push_wards_for_role(role_for_runes, my_team_side);
                                        let mut pinks_seen = 0usize;
                                        push_wards.retain(|w| {
                                            if w.ward_type == "pink" { pinks_seen += 1; pinks_seen <= pink_count }
                                            else { true }
                                        });
                                        if !push_wards.is_empty() {
                                            coach_state.last_push_ward_time = game_time;
                                            let pw_count = push_wards.len();
                                            let _ = handle.emit("update-ward-map", serde_json::json!({
                                                "champion": champion_for_runes,
                                                "role": role_for_runes,
                                                "game_time": game_time,
                                                "phase": phase,
                                                "team_side": my_team_side,
                                                "wards": push_wards,
                                                "display_secs": 10,
                                            }));
                                            println!("[WardMap:Push] {} pontos para janela de push após kill", pw_count);
                                        }
                                    }
                                }
                            }
                            "Ace" => {
                                let acing_team = event["AcingTeam"].as_str().unwrap_or("");
                                if acing_team == team_side {
                                    let obj = if game_time >= coach_state.baron_next_spawn { "Barão" }
                                        else if game_time >= coach_state.dragon_next_spawn { "Dragão" }
                                        else { "uma torre" };
                                    // Ace sempre sobrescreve — é a maior oportunidade do jogo
                                    coach_state.pending_event_tip = Some((
                                        "⚔️ ACE — Vá ao Objetivo!".to_string(),
                                        format!("5x0! Vá imediatamente ao {} — a janela fecha em segundos.", obj),
                                    ));
                                    println!("[Event] ACE pelo time do jogador → {}", obj);
                                }
                            }
                            _ => {}
                        }
                    }
                    coach_state.last_processed_event_count = total_events;
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
                if obj.spawn == f64::MAX { continue; }
                let secs_left = obj.spawn - game_time;
                // Janela ampliada para 60s — dá tempo de caminhar até o ponto de ward
                if secs_left > 60.0 || secs_left < 0.0 { continue; }

                let alert_key = format!("{}:{:.0}", obj.key, obj.spawn);
                if coach_state.objective_ward_alerts.contains(&alert_key) { continue; }
                coach_state.objective_ward_alerts.insert(alert_key);

                // Tenta dados do DB; fallback para posições estáticas se não houver dados
                let mut wards = if !db_key.is_empty() {
                    let db_wards = db::coach_service::get_ward_suggestions_for_objective(
                        pool, db_key, role_for_runes,
                        obj.spawn, obj.x_min, obj.x_max, obj.y_min, obj.y_max,
                    ).await;
                    if db_wards.is_empty() {
                        static_wards_for_objective(obj.key, my_team_side)
                    } else {
                        // Dados do DB são coordenadas absolutas de jogos reais (sem filtro de lado).
                        // Para objetivos, ambos os times wardam a mesma área do pit — coords válidas para os dois.
                        db_wards
                    }
                } else {
                    static_wards_for_objective(obj.key, my_team_side)
                };
                // Mantém só tantas pinks quanto o jogador tem no inventário
                let pink_count = gs_inventory.iter().filter(|&&id| id == 2055).count();
                let mut pinks_seen = 0usize;
                wards.retain(|w| {
                    if w.ward_type == "pink" { pinks_seen += 1; pinks_seen <= pink_count }
                    else { true }
                });

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

            // ── 3. Ward map genérico a cada 2 minutos — não sobrepõe tip imediata ──
            // Só bloqueia se uma tip disparou nos últimos 5s (não 30s, que travava sempre).
            // db_key vazio NÃO bloqueia mais — usa fallback estático por role.
            let ward_cooldown = 120.0;
            let tip_recently = (game_time - *last_tip_emit_time) < 5.0;
            let ward_due = game_time >= 120.0
                && (game_time - coach_state.last_ward_map_time) >= ward_cooldown
                && !tip_recently;

            if ward_due {
                // Tenta DB primeiro; fallback estático por role se vazio
                let mut wards = if !db_key.is_empty() {
                    let db_wards = db::coach_service::get_ward_suggestions(
                        pool, db_key, role_for_runes, game_time
                    ).await;
                    if db_wards.is_empty() {
                        println!("[WardMap] DB vazio para {}/{} — usando fallback estático", db_key, role_for_runes);
                        static_wards_generic(role_for_runes, my_team_side)
                    } else {
                        // Dados do DB são coordenadas absolutas sem filtro de lado (blue+red misturados).
                        // Para red side, espelha as coordenadas (15000-x, 15000-y) para que os pontos
                        // apareçam no lado correto do minimapa.
                        if my_team_side == "red" {
                            db_wards.into_iter().map(|mut w| { w.x = 15000 - w.x; w.y = 15000 - w.y; w }).collect()
                        } else {
                            db_wards
                        }
                    }
                } else {
                    println!("[WardMap] db_key vazio — usando fallback estático para role={} side={}", role_for_runes, my_team_side);
                    static_wards_generic(role_for_runes, my_team_side)
                };
                // Mantém só tantas pinks quanto o jogador tem no inventário
                let pink_count = gs_inventory.iter().filter(|&&id| id == 2055).count();
                let mut pinks_seen = 0usize;
                wards.retain(|w| {
                    if w.ward_type == "pink" { pinks_seen += 1; pinks_seen <= pink_count }
                    else { true }
                });

                if !wards.is_empty() {
                    coach_state.last_ward_map_time = game_time;
                    let w_count = wards.len();
                    let _ = handle.emit("update-ward-map", serde_json::json!({
                        "champion": champion_for_runes,
                        "role": role_for_runes,
                        "game_time": game_time,
                        "phase": phase,
                        "team_side": my_team_side,
                        "wards": wards,
                        "display_secs": 8,
                    }));
                    println!("[WardMap] {} pontos emitidos para {}/{} em {:.0}s",
                        w_count, champion_for_runes, role_for_runes, game_time);
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
