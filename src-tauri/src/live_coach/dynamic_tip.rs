use super::state::{CoachState, ActiveRec};
use super::micro_tip::{can_fire, var_idx};
use crate::db::player_style_service::PlayerAggregateProfile;

pub fn get_dynamic_coaching_tip(
    state: &mut CoachState,
    lca_data: &serde_json::Value,
    role: &str,
    profile_opt: Option<&PlayerAggregateProfile>,
) -> Option<(String, String)> {
    let game_time = lca_data["gameData"]["gameTime"].as_f64().unwrap_or(0.0);
    let active_summ_name = lca_data["activePlayer"]["summonerName"].as_str().unwrap_or("");
    let gold = lca_data["activePlayer"]["currentGold"].as_f64().unwrap_or(0.0);

    let hp_current = lca_data["activePlayer"]["championStats"]["currentHealth"].as_f64().unwrap_or(1000.0);
    let hp_max = lca_data["activePlayer"]["championStats"]["maxHealth"].as_f64().unwrap_or(1000.0);
    let health_perc = if hp_max > 0.0 { hp_current / hp_max } else { 1.0 };

    let mana_current = lca_data["activePlayer"]["championStats"]["resourceValue"].as_f64().unwrap_or(100.0);
    let mana_max = lca_data["activePlayer"]["championStats"]["resourceMax"].as_f64().unwrap_or(100.0);
    let mana_perc = if mana_max > 0.0 { mana_current / mana_max } else { 1.0 };

    let active_player_champ = {
        let raw = lca_data["activePlayer"]["championName"].as_str().unwrap_or("");
        if !raw.is_empty() {
            raw.to_string()
        } else {
            let base = active_summ_name.split('#').next().unwrap_or(active_summ_name).to_lowercase();
            lca_data["allPlayers"].as_array()
                .and_then(|arr| arr.iter().find(|p| {
                    let p_name = p["summonerName"].as_str().unwrap_or("");
                    let p_base = p_name.split('#').next().unwrap_or(p_name).to_lowercase();
                    p_name == active_summ_name || (!base.is_empty() && p_base == base)
                }))
                .and_then(|p| p["championName"].as_str())
                .unwrap_or("")
                .to_string()
        }
    };
    let level = lca_data["activePlayer"]["level"].as_i64().unwrap_or(1);
    let _ = level; // usado nas branches abaixo

    // --- ALERTAS DE RENASCIMENTO DE CAMPOS DA SELVA (JUNGLE RESPAWN WARNINGS) ---
    if role.to_uppercase() == "JUNGLE" {
        // Alerta Aronguejo (Scuttle): 3:20 → nasce 3:30 (210s)
        if game_time >= 200.0 && game_time < 215.0 && !state.respawn_warnings_sent.contains("scuttle_1") {
            state.respawn_warnings_sent.insert("scuttle_1".to_string());
            return Some((
                "🦀 Aronguejo — 10s!".to_string(),
                "Aronguejo em 10s — posicione no rio agora.".to_string()
            ));
        }
        // Segundo Aronguejo: ~5:50 (3:30 inicial + 2:30 respawn = 360s)
        if game_time >= 350.0 && game_time < 365.0 && !state.respawn_warnings_sent.contains("scuttle_2") {
            state.respawn_warnings_sent.insert("scuttle_2".to_string());
            return Some((
                "🦀 Segundo Aronguejo — 10s".to_string(),
                "Segundo Aronguejo nascendo — avalie se vale contestar.".to_string()
            ));
        }
        // Alerta Buffs: 6:25 (nasce 6:40)
        if game_time >= 385.0 && game_time < 395.0 && !state.respawn_warnings_sent.contains("buffs_1") {
            state.respawn_warnings_sent.insert("buffs_1".to_string());
            return Some((
                "🛡️ Bufos em 15s".to_string(),
                "Campo de bônus renascendo — garanta ou rastreie o caçador inimigo.".to_string()
            ));
        }
        // Pre-posicionamento Dragon: alerta ~60-70s antes do 2º Dragon (aprox 10:00 = 600s)
        // O JG deve estabelecer visão no Dragon pit antes de iniciar, não 5s antes
        if game_time >= 540.0 && game_time < 570.0 && !state.dragon_setup_warned {
            state.dragon_setup_warned = true;
            return Some((
                "🐉 Preparação Dragão".to_string(),
                "Dragão em breve — ward no rio e prioridade de rota.".to_string()
            ));
        }
        // Voidgrubs last chance: despawnam às 14:00 (840s), alerta em 13:20 (800s)
        if game_time >= 800.0 && game_time < 840.0 && !state.grubs_last_chance_warned {
            state.grubs_last_chance_warned = true;
            return Some((
                "🐛 Vastilarvas — Última Chance".to_string(),
                "Vastilarvas somem às 14:00 — priorize os acúmulos restantes agora.".to_string()
            ));
        }
        // Aviso de Baron Nashusor: nasce às 20:00 (1200s), alerta aos 19:00 (1140s)
        // Pesquisa: "sentinelas 60-90 segundos ANTES de objetivo, nunca 5s antes"
        if game_time >= 1140.0 && game_time < 1160.0 && !state.baron_warned {
            state.baron_warned = true;
            return Some((
                "💜 Barão — 1 Minuto".to_string(),
                "Barão em 60s — agrupe no rio, não morra antes.".to_string()
            ));
        }
    }

    // --- COGNITIVE PRAISE LOOP (ELOGIO DE AÇÕES CONCLUÍDAS) ---
    if let Some(rec) = state.active_recommendation.clone() {
        match rec {
            ActiveRec::RecallGold => {
                if gold < 300.0 {
                    state.active_recommendation = None;
                    return Some((
                        "✅ Item Comprado".to_string(),
                        "Item comprado — retorne com pico de poder.".to_string()
                    ));
                }
            },
            ActiveRec::RecallHp => {
                if health_perc >= 0.90 {
                    state.active_recommendation = None;
                    return Some((
                        "✅ Vida Restaurada".to_string(),
                        "Vida cheia — retorne e retome a pressão na rota.".to_string()
                    ));
                }
            },
            ActiveRec::PlaySafe { ref enemy_champ, start_time, initial_deaths } => {
                let mut current_deaths = 0;
                if let Some(players) = lca_data["allPlayers"].as_array() {
                    for p in players {
                        if p["summonerName"].as_str().unwrap_or("") == active_summ_name {
                            current_deaths = p["scores"]["deaths"].as_i64().unwrap_or(0);
                            break;
                        }
                    }
                }

                if game_time - start_time >= 45.0 {
                    state.active_recommendation = None;
                    if current_deaths <= initial_deaths {
                        return Some((
                            "✅ Defesa Sólida".to_string(),
                            format!("Segurou {} sem morrer — agora reconquiste a rota.", enemy_champ)
                        ));
                    }
                }
            }
        }
    }

    // Identificação de time e oponentes
    let mut team_side = "ORDER".to_string();
    let mut active_player_kills = 0;
    let mut active_player_cs = 0;
    let mut active_player_deaths = 0;
    let mut active_player_champ = String::new();
    if let Some(players) = lca_data["allPlayers"].as_array() {
        for p in players {
            if p["summonerName"].as_str().unwrap_or("") == active_summ_name {
                team_side = p["team"].as_str().unwrap_or("ORDER").to_uppercase();
                active_player_kills = p["scores"]["kills"].as_i64().unwrap_or(0);
                active_player_cs = p["scores"]["creepScore"].as_i64().unwrap_or(0);
                active_player_deaths = p["scores"]["deaths"].as_i64().unwrap_or(0);
                active_player_champ = p["championName"].as_str().unwrap_or("").to_string();
                break;
            }
        }
    }

    // --- ANIMATION CANCEL / MECANICA ESPECIFICA POR CAMPEÃO ---
    if game_time >= 100.0 && !state.animation_cancel_suggested && !active_player_champ.is_empty() {
        state.animation_cancel_suggested = true;
        let (title, cancel_tip) = match active_player_champ.to_lowercase().as_str() {
            "riven" => (
                "⚡ Riven: Fast Q Cancel".to_string(),
                "Auto → Q no frame do projétil → movimento → próximo auto.".to_string()
            ),
            "yasuo" => (
                "⚡ Yasuo: E-Q Cancel".to_string(),
                "E no minion → Q no impacto do dash para combo rápido.".to_string()
            ),
            "caitlyn" => (
                "⚡ Caitlyn: E-Q-W".to_string(),
                "E para trás → Q imediato → W onde o inimigo vai pousar.".to_string()
            ),
            "renekton" => (
                "⚡ Renekton: W Cancel".to_string(),
                "W fortalecido → Q ou E imediatamente após o stun.".to_string()
            ),
            "leesin" | "lee sin" => (
                "⚡ Lee Sin: Insec".to_string(),
                "Q → ward atrás do alvo durante o voo → W → R.".to_string()
            ),
            "vayne" => (
                "⚡ Vayne: Wall Tumble".to_string(),
                "Auto → Q rente à parede → auto imediato — reset de ataque.".to_string()
            ),
            "rengar" => (
                "⚡ Rengar: Burst".to_string(),
                "Salte com 4 de fúria → Q no ar → W e E antes de pousar.".to_string()
            ),
            "aatrox" => (
                "⚡ Aatrox: Q-E Redirect".to_string(),
                "Q → E durante a animação para acertar a ponta da espada.".to_string()
            ),
            "lucian" => (
                "⚡ Lucian: Passive Reset".to_string(),
                "Ability → double shot → E no frame do segundo projétil.".to_string()
            ),
            "alistar" => (
                "⚡ Alistar: W-Q".to_string(),
                "W no alvo → Q exatamente antes do impacto do headbutt.".to_string()
            ),
            "ezreal" => (
                "⚡ Ezreal: R-E Cancel".to_string(),
                "R → E imediato para reposicionar com o projétil em voo.".to_string()
            ),
            _ => (
                "⚡ Caitar — Cancel de Auto".to_string(),
                "Auto → movimento após o projétil sair → próximo auto.".to_string()
            )
        };
        return Some((title, cancel_tip));
    }

    // --- DICA DE VOLTA À BASE PERFEITA PARA CAMPEÕES DE ROTA ---
    let is_lane_role = ["TOP", "MID", "MIDDLE", "ADC", "BOTTOM", "SUPPORT", "UTILITY"].contains(&role.to_uppercase().as_str());
    if is_lane_role && game_time >= 210.0 && game_time < 300.0 && !state.recall_timing_suggested {
        state.recall_timing_suggested = true;
        return Some((
            "🔄 Recall Perfeito".to_string(),
            "Empurre a onda de canhão antes de voltar à base.".to_string()
        ));
    }

    // --- ROTAÇÃO E OBJETIVOS PARA MID ---
    let is_mid_role = ["MID", "MIDDLE"].contains(&role.to_uppercase().as_str());
    if is_mid_role && game_time >= 360.0 && game_time < 480.0 && !state.mid_roaming_suggested {
        state.mid_roaming_suggested = true;
        return Some((
            "🗺️ Meio — Rotacione".to_string(),
            "Avance a onda sob a torre antes de rotacionar ao objetivo.".to_string()
        ));
    }

    // --- SUPORTE MACRO: CONTROLE DE ARBUSTO E VISÃO ---
    let is_sup_role = ["SUPPORT", "UTILITY"].contains(&role.to_uppercase().as_str());
    if is_sup_role {
        if game_time >= 120.0 && game_time < 200.0 && !state.sup_bush_dominance_suggested {
            state.sup_bush_dominance_suggested = true;
            return Some((
                "🌿 Arbusto Lateral".to_string(),
                "Domine o arbusto — pressão de zona e visão de emboscada.".to_string()
            ));
        }

        if game_time >= 240.0 && game_time < 330.0 && !state.sup_river_vision_suggested {
            state.sup_river_vision_suggested = true;
            return Some((
                "👁️ Controle do Rio".to_string(),
                "Onda controlada — jogue pelo rio e colete informação.".to_string()
            ));
        }
    }

    // --- TOPO MACRO: CONTROLE DE ONDA, VASTILARVAS, ARAUTO, TELEPORTE ---
    let is_top_role = ["TOP"].contains(&role.to_uppercase().as_str());
    if is_top_role {
        if game_time >= 150.0 && game_time < 230.0 && !state.top_wave_control_suggested {
            state.top_wave_control_suggested = true;
            return Some((
                "❄️ Congele a Onda".to_string(),
                "Congele perto da torre — force exposição e crie emboscadas.".to_string()
            ));
        }

        if game_time >= 270.0 && game_time < 305.0 && !state.top_grubs_priority_suggested {
            state.top_grubs_priority_suggested = true;
            return Some((
                "🐛 Vastilarvas — 30s".to_string(),
                "Vastilarvas em 30s — avance, dê prioridade ao caçador.".to_string()
            ));
        }

        // Arauto do Vale nasce às 14:00 no jogo atual (Season 14/15 com Voidgrubs no slot de 5:00)
        if game_time >= 825.0 && game_time < 860.0 && !state.top_herald_suggested {
            state.top_herald_suggested = true;
            return Some((
                "🔮 Arauto — 15s".to_string(),
                "Arauto em 15s — agrupe com o caçador agora.".to_string()
            ));
        }

        if game_time >= 870.0 && game_time < 960.0 && !state.top_tp_management_suggested {
            state.top_tp_management_suggested = true;
            return Some((
                "⚡ Gestão de Recurso".to_string(),
                "Empurre a onda antes de sair. Preserve recurso global para objetivos.".to_string()
            ));
        }
    }

    // 0. Gatilhos de Perigo Imediato (Vida, Mana, Ouro)
    if health_perc > 0.0 && health_perc < 0.25 && (game_time - state.last_danger_alert_time >= 40.0) {
        state.last_danger_alert_time = game_time;
        state.active_recommendation = Some(ActiveRec::RecallHp);
        return Some((
            "🚨 Vida Crítica".to_string(),
            "Vida abaixo de 25% — saia da rota imediatamente.".to_string()
        ));
    }
    if gold >= 1400.0 && (game_time - state.last_danger_alert_time >= 50.0) {
        state.last_danger_alert_time = game_time;
        state.active_recommendation = Some(ActiveRec::RecallGold);
        return Some((
            "💰 Base — Converta o Ouro".to_string(),
            format!("{:.0} de ouro — volte à base e compre o item agora.", gold)
        ));
    }
    if mana_perc > 0.0 && mana_perc < 0.15 && (game_time - state.last_danger_alert_time >= 60.0) {
        state.last_danger_alert_time = game_time;
        return Some((
            "🔋 Mana Crítica".to_string(),
            "Mana abaixo de 15% — jogue defensivo ou recue à base.".to_string()
        ));
    }

    // --- ALERTA DE EMBOSCADA VIA TELEMETRIA ---
    if game_time - state.last_gank_alert_time >= 60.0 && game_time > 120.0 {
        let is_lane = ["TOP", "MID", "ADC", "SUPPORT"].contains(&role.to_uppercase().as_str());
        if is_lane {
            for tel in &state.enemy_telemetries {
                let is_ganker = tel.role == "JUNGLE" || (tel.role == "MID" && role.to_uppercase() != "MID");
                if is_ganker && !tel.is_visible && tel.fog_duration >= 10.0 && tel.fog_duration <= 25.0 {
                    state.last_gank_alert_time = game_time;
                    let role_display = if tel.role == "JUNGLE" { "Caçador" } else { "Meio" };
                    return Some((
                        "🚨 Alerta de Emboscada".to_string(),
                        format!("{} ({}) sumiu há {:.0}s — fique perto da torre.", role_display, tel.champion_name, tel.fog_duration)
                    ));
                }
            }
        }
    }

    // --- PERIODIC MINIMAP CHECK REMINDERS (a cada 120s, máx 4 por partida) ---
    if game_time - state.last_minimap_time >= 120.0 && game_time > 90.0 {
        if let Some(n) = can_fire(state, "MINIMAP", 7) {
            state.last_minimap_time = game_time;
            let msgs = [
                ("🗺️ Rastreie os Inimigos", "Onde estão os 5 inimigos? Sumiu = recue e jogue conservador."),
                ("👁️ Informação", "Confirme a posição dos inimigos perigosos antes de avançar."),
                ("🗺️ Objetivos", "Verifique o próximo objetivo — posicione 60s antes."),
                ("🗺️ Pressão de Mapa", "Qual rota tem onda avançada? Pressione dois pontos simultâneos."),
                ("👁️ Rastreie o Caçador", "Caçador inimigo pelo lado da onda atrasada — avance com segurança."),
                ("📖 Controle de Onda", "Congele, acumule devagar ou empurre rápido — três estados fundamentais."),
                ("📖 Pressão de Objetivo", "Onda avançada na rota oposta ao objetivo divide a defesa inimiga."),
            ];
            let idx = var_idx(n, game_time, msgs.len());
            let (t, m) = msgs[idx];
            return Some((t.to_string(), m.to_string()));
        }
    }

    let my_team = team_side;
    let mut enemy_players = Vec::new();
    let mut ally_players = Vec::new();

    if let Some(players) = lca_data["allPlayers"].as_array() {
        for p in players {
            let p_team = p["team"].as_str().unwrap_or("ORDER").to_uppercase();
            if p_team != my_team {
                enemy_players.push(p);
            } else if p["summonerName"].as_str().unwrap_or("") != active_summ_name {
                ally_players.push(p);
            }
        }
    }

    // --- CÁLCULO DE ABERTURA DE GANK (GANK OPPORTUNITIES - JG) ---
    if role.to_uppercase() == "JUNGLE" && game_time > 120.0 && (game_time - state.last_gank_alert_time >= 50.0) {
        for enemy in &enemy_players {
            let is_dead = enemy["isDead"].as_bool().unwrap_or(false);
            if !is_dead {
                let hp = enemy["championStats"]["currentHealth"].as_f64().unwrap_or(1000.0);
                let hp_max = enemy["championStats"]["maxHealth"].as_f64().unwrap_or(1000.0);
                let enemy_hp_perc = if hp_max > 0.0 { hp / hp_max } else { 1.0 };
                let enemy_pos = enemy["position"].as_str().unwrap_or("").to_uppercase();

                if enemy_hp_perc > 0.0 && enemy_hp_perc < 0.40 {
                    let lane_display = match enemy_pos.as_str() {
                        "TOP" => "Top",
                        "MIDDLE" => "Mid",
                        "BOTTOM" => "Bot",
                        _ => "",
                    };
                    if !lane_display.is_empty() {
                        let enemy_name = enemy["championName"].as_str().unwrap_or("Inimigo");
                        state.last_gank_alert_time = game_time;
                        return Some((
                            "🎯 Gank Window — JG".to_string(),
                            format!(
                                "{} ({}) a {:.0}% HP — entre pelo flanco da selva, não pelo corredor da rota. Coordene com o laner para CC primeiro.",
                                enemy_name, lane_display, enemy_hp_perc * 100.0
                            )
                        ));
                    }
                }
            }
        }
    }

    // 1. ANÁLISE DE MATCHUP DIRETO NO TAB
    let mut enemy_laner_champ = String::new();
    let mut enemy_laner_kills = 0;
    let mut enemy_laner_cs = 0;

    let normalized_role = match role.to_uppercase().as_str() {
        "MID" | "MIDDLE" => "MIDDLE",
        "TOP" => "TOP",
        "JUNGLE" => "JUNGLE",
        "ADC" | "BOTTOM" => "BOTTOM",
        "SUPPORT" | "UTILITY" => "UTILITY",
        _ => "",
    };

    for enemy in &enemy_players {
        let enemy_pos = enemy["position"].as_str().unwrap_or("").to_uppercase();
        if !normalized_role.is_empty() && enemy_pos == normalized_role {
            enemy_laner_champ = enemy["championName"].as_str().unwrap_or("").to_string();
            enemy_laner_kills = enemy["scores"]["kills"].as_i64().unwrap_or(0);
            enemy_laner_cs = enemy["scores"]["creepScore"].as_i64().unwrap_or(0);
            break;
        }
    }

    let mut current_phase_id = String::new();
    if game_time > 0.0 && game_time < 300.0 {
        current_phase_id = "EARLY_1".to_string();
    } else if game_time >= 300.0 && game_time < 840.0 {
        current_phase_id = "EARLY_2".to_string();
    } else if game_time >= 840.0 && game_time < 1500.0 {
        current_phase_id = "MID_1".to_string();
    } else if game_time >= 1500.0 {
        current_phase_id = "LATE_1".to_string();
    }

    if !current_phase_id.is_empty() && state.last_phase != current_phase_id {
        state.last_phase = current_phase_id.clone();
        state.last_alert_time = game_time;
    } else if game_time - state.last_alert_time >= 90.0 && can_fire(state, "ADV_CONTEXT", 5).is_some() {
        state.last_alert_time = game_time;

        // --- ADVERSARIAL MATCHUP FEEDBACK (baseado em dados reais da tab) ---
        if !enemy_laner_champ.is_empty() && game_time > 300.0 {
            if enemy_laner_kills >= active_player_kills + 2 {
                state.active_recommendation = Some(ActiveRec::PlaySafe {
                    enemy_champ: enemy_laner_champ.clone(),
                    start_time: game_time,
                    initial_deaths: active_player_deaths,
                });
                return Some((
                    "⚔️ Oponente com Vantagem".to_string(),
                    format!("{} com {} kills de frente — não tente x1 sem poke prévio ou engage do JG. Jogue safe na torre e aguarde equalização.", enemy_laner_champ, enemy_laner_kills)
                ));
            } else if active_player_kills >= enemy_laner_kills + 2 {
                return Some((
                    "🔥 Vantagem na Rota".to_string(),
                    format!("{} kills de frente — converta em pressão de torre ou roam. Ouro de kills sem pressão de objetivos não vence o jogo.", active_player_kills)
                ));
            } else if enemy_laner_cs > active_player_cs + 20 {
                return Some((
                    "🌾 CS Gap Crítico".to_string(),
                    format!("{} minions de diferença — cada 15 CS equivale a uma kill em ouro. Priorize last hit em vez de trades sem conversão.", enemy_laner_cs - active_player_cs)
                ));
            }
        }

        // --- FED ENEMY DETECTION ---
        let mut fed_enemy_champ = String::new();
        let mut fed_enemy_kills = 0;
        for enemy in &enemy_players {
            let ekills = enemy["scores"]["kills"].as_i64().unwrap_or(0);
            let edeaths = enemy["scores"]["deaths"].as_i64().unwrap_or(0);
            if ekills >= 4 && ekills > edeaths && ekills > fed_enemy_kills {
                fed_enemy_kills = ekills;
                fed_enemy_champ = enemy["championName"].as_str().unwrap_or("").to_string();
            }
        }

        if !fed_enemy_champ.is_empty() && game_time > 300.0 {
            state.active_recommendation = Some(ActiveRec::PlaySafe {
                enemy_champ: fed_enemy_champ.clone(),
                start_time: game_time,
                initial_deaths: active_player_deaths,
            });
            return Some((
                "⚠️ Inimigo Escalado".to_string(),
                format!("{} com {} kills — nunca fique isolado sem visão na rota dele. Jogue em grupo ou aguarde JG inimigo estar visível antes de avançar.", fed_enemy_champ, fed_enemy_kills)
            ));
        }

        let alert = get_dynamic_context_alert(game_time, role, &enemy_players, &ally_players);
        return Some(alert);
    } else {
        return None;
    }

    // Geração de Dica de Fase (Estratégia Pura Micro e Macro)
    let (title, tip) = match role.to_uppercase().as_str() {
        "TOP" => {
            let top_early2_tip = if game_time < 480.0 {
                "Vastilarvas nascem às 5:00 no seu lado! Puxe a wave agora e ajude seu JG a garantir o objetivo de cerco."
            } else {
                "Arauto do Vale ativo na fenda! Puxe a wave, ganhe prioridade e ajude seu JG a garantir o Arauto."
            };
            match current_phase_id.as_str() {
                "EARLY_1" => ("Top Laning".to_string(), "Top: pega Lvl 2 na primeira wave, domina o arbusto de cima e warda o rio aos 2:45.".to_string()),
                "EARLY_2" => ("🎯 Objetivo Top!".to_string(), top_early2_tip.to_string()),
                "MID_1" => ("Split Push".to_string(), "Split ativo. Se tiver TP pronto, empurre a side oposta ao Baron ou Dragão.".to_string()),
                _ => ("Late Game Top".to_string(), "Late game: decida entre flanquear com TP na backline ou fazer peeling pro carry.".to_string())
            }
        },
        "JUNGLE" => {
            let obj_tip = if game_time < 480.0 {
                // 5:00-8:00: Vastilarvas e Dragão disponíveis
                "Vastilarvas (top) e Dragão (bot) estão no mapa — decida com o time qual pegar primeiro!"
            } else {
                // 8:00+: Arauto do Vale disponível
                "Arauto do Vale ativo! Garanta visão na fenda e trackeia o JG inimigo antes de iniciar."
            };
            match current_phase_id.as_str() {
                "EARLY_1" => ("📍 Trackeia o JG".to_string(), "Observe qual rota inimiga chegou atrasada — isso revela por onde o JG deles começou. Trackeia o pathing!".to_string()),
                "EARLY_2" => ("🎯 Objetivos!".to_string(), obj_tip.to_string()),
                "MID_1" => ("👁️ Controle Rio".to_string(), "Sweeper ativado. Limpe visão no rio, crie armadilhas e jogue com quem está carregando.".to_string()),
                _ => ("⚡ Objetivo Decisivo".to_string(), "Não morra agora — sua presença vale Barão ou Dragão Ancião. Entre no objetivo só com posição e vida suficientes.".to_string())
            }
        },
        "MID" | "MIDDLE" => {
            match current_phase_id.as_str() {
                "EARLY_1" => ("Prioridade Mid".to_string(), "Mid: pegue Lvl 2 rápido para zonear e garanta o controle do rio.".to_string()),
                "EARLY_2" => ("Roaming Mid".to_string(), "Empurre a wave rápido na torre deles e procure roam ou warda as galinhas.".to_string()),
                "MID_1" => ("Side Farm".to_string(), "ADC foi Mid? Vá para a side lane farmar e obter recursos com segurança.".to_string()),
                _ => ("Picks na Névoa".to_string(), "Fique na névoa de guerra e procure pick-offs rápidos antes das lutas.".to_string())
            }
        },
        "ADC" | "BOTTOM" => {
            match current_phase_id.as_str() {
                "EARLY_1" => ("Atirador Early".to_string(), "ADC: pegue Lvl 2 na wave 1 + 3 minions magos. Pune o recuo deles.".to_string()),
                "EARLY_2" => ("Farm Seguro".to_string(), "Foque em pegar barricadas seguras e só avance com visão profunda no rio.".to_string()),
                "MID_1" => ("Mid Rotação".to_string(), "Rotacione para o Mid! É mais seguro coletar ouro sob a escolta do suporte.".to_string()),
                _ => ("Fights Atirador".to_string(), "Fique atrás da linha de frente. Bata no alvo mais próximo e preserve seus recursos defensivos para a ameaça real.".to_string())
            }
        },
        "SUPPORT" | "UTILITY" => {
            match current_phase_id.as_str() {
                "EARLY_1" => ("Suporte Arbusto".to_string(), "Sup: domine o arbusto da rota. Pressione o ADC inimigo no last hit dele.".to_string()),
                "EARLY_2" => ("Roam Suporte".to_string(), "ADC recuou ou congelou wave? Faça um roam rápido no Mid ou rio.".to_string()),
                "MID_1" => ("Barreira de Informação".to_string(), "Prepare informação antes dos objetivos: acompanhe seu time pelo rio e evite entrar sozinho na selva inimiga.".to_string()),
                _ => ("Proteção Lutas".to_string(), "Peeling total. Use stuns e exaustão para salvar quem tá batendo nas lutas.".to_string())
            }
        },
        _ => {
            ("Dica Geral".to_string(), "Foque em farmar, evite lutas desvantajosas e warda o mapa sempre.".to_string())
        }
    };

    let mut personalized_tip = tip;
    if let Some(profile) = profile_opt {
        if current_phase_id == "EARLY_2" || current_phase_id == "MID_1" {
            if profile.avg_vision_score_per_min < 0.5 {
                personalized_tip = "🧠 HISTÓRICO: sua informação de mapa costuma ser baixa. Jogue mais perto do rio com aliados e confirme inimigos antes de avançar.".to_string();
            } else if profile.avg_cs_per_min < 6.0 {
                personalized_tip = "🧠 HISTÓRICO: Seu farm médio está baixo. Foca no last hit e pare de brigar à toa.".to_string();
            }
        } else if current_phase_id == "LATE_1" {
            if profile.avg_deaths > 6.5 {
                personalized_tip = "🧠 HISTÓRICO: Você costuma morrer muito no late game. Fique recuado e jogue safe!".to_string();
            }
        }
    }

    Some((title, personalized_tip))
}

pub fn get_dynamic_context_alert(
    game_time: f64,
    role: &str,
    enemy_players: &[&serde_json::Value],
    ally_players: &[&serde_json::Value]
) -> (String, String) {
    let mut enemy_bot_dead = false;
    let mut enemy_jg_dead = false;
    let mut enemy_mid_dead = false;
    for enemy in enemy_players {
        let pos = enemy["position"].as_str().unwrap_or("").to_uppercase();
        let is_dead = enemy["isDead"].as_bool().unwrap_or(false);
        if is_dead {
            match pos.as_str() {
                "BOTTOM" | "UTILITY" => enemy_bot_dead = true,
                "JUNGLE" => enemy_jg_dead = true,
                "MIDDLE" => enemy_mid_dead = true,
                _ => {}
            }
        }
    }

    let mut ally_adc_dead = false;
    for ally in ally_players {
        let pos = ally["position"].as_str().unwrap_or("").to_uppercase();
        let is_dead = ally["isDead"].as_bool().unwrap_or(false);
        if is_dead && pos == "BOTTOM" {
            ally_adc_dead = true;
        }
    }

    // Conta quantos inimigos estao mortos
    let dead_enemy_count = enemy_players.iter().filter(|e| e["isDead"].as_bool().unwrap_or(false)).count();
    let dead_enemy_names: Vec<&str> = enemy_players.iter()
        .filter(|e| e["isDead"].as_bool().unwrap_or(false))
        .map(|e| e["championName"].as_str().unwrap_or("Inimigo"))
        .collect();

    // 3+ inimigos mortos = janela de objetivo imediata
    if dead_enemy_count >= 3 && game_time > 400.0 {
        return (
            "🔥 Vantagem Numerica".to_string(),
            format!("{}+ inimigos mortos — janela de Baron/Dragon agora. Cada segundo que passa e HP que eles regeneram reduz a vantagem.", dead_enemy_count)
        );
    }

    // JG morto = selva free para invadir
    if enemy_jg_dead {
        return (
            "⚔️ JG Inimigo Morto".to_string(),
            "Selva inimiga esta free — invada e roube campos do lado oposto. Cada campo equivale a 120-170g sem risco de confronto.".to_string()
        );
    }

    // Bot duplo morto = Dragon gratis
    let enemy_support_dead = enemy_players.iter().any(|e| {
        let pos = e["position"].as_str().unwrap_or("").to_uppercase();
        let dead = e["isDead"].as_bool().unwrap_or(false);
        dead && pos == "UTILITY"
    });
    if enemy_bot_dead && enemy_support_dead && game_time > 180.0 {
        return (
            "🐉 Dragon — Gratis".to_string(),
            "ADC e Suporte inimigos mortos — Dragão com baixa contestação. Inicie com o caçador vivo e aliados próximos; não faça sozinho se a vida estiver baixa.".to_string()
        );
    }
    if enemy_bot_dead && game_time > 180.0 {
        return (
            "🐉 Dragon Prioritario".to_string(),
            "ADC inimigo morto — boa janela para Dragon. Garanta visao no rio inferior antes de iniciar; suporte deles pode ainda contestar.".to_string()
        );
    }

    // Mid morto = pressione a torre mid
    if enemy_mid_dead {
        return (
            "🎯 Mid Morto — Torre".to_string(),
            "Mid inimigo morto — shove rapido a wave mid e pressione a torre. Cada segundo de respawn e ouro de torre que voce pode coletar.".to_string()
        );
    }

    // Se algum inimigo morreu mas nao e uma janela de objetivo clara
    if !dead_enemy_names.is_empty() && game_time > 300.0 {
        return (
            "🔥 Janela Aberta".to_string(),
            format!("{} morto — pressione o objetivo mais proximo ou farm side lane livre. Cada morte inimiga e espaco de mapa que voce deve converter.", dead_enemy_names[0])
        );
    }

    // Fallback: dica de macro baseada na role
    match role.to_uppercase().as_str() {
        "JUNGLE" => {
            if enemy_jg_dead {
                ("⚔️ Invasao Livre".to_string(), "Selva do inimigo esta free. Invada, roube campos e estabeleca controle de visao na selva dele.".to_string())
            } else {
                ("📍 Trackeia o Pathing".to_string(), "Identifique o lado onde a wave inimiga chegou atrasada — isso revela por onde o JG deles comecou. Antecipe o proximo gank.".to_string())
            }
        },
        "MID" | "MIDDLE" => {
            if enemy_mid_dead {
                ("🔥 Puxe e Pressione".to_string(), "Mid deles morreu — clear rapido e pressione a torre. Nao deixe a wave bater de graca.".to_string())
            } else {
                ("🗺️ Side Lane Farming".to_string(), "Shove a wave mid e farme a side lane mais proxima. Ouro flutuante no mapa com visao segura e EV positivo.".to_string())
            }
        },
        "ADC" | "BOTTOM" => {
            if enemy_jg_dead {
                ("🏹 Abuse o Bot".to_string(), "JG deles morreu — jogue agressivo na rota. Sem cobertura de gank, os trade win conditions do bot sao maximizados.".to_string())
            } else {
                ("🛡️ Jogue Controlado".to_string(), "JG inimigo nao esta trackado — jogue dentro do range da torre. Farm seguro > trade arriscado sem visao.".to_string())
            }
        },
        "SUPPORT" | "UTILITY" => {
            if ally_adc_dead {
                ("🗺️ Roam Oportunista".to_string(), "ADC aliado morto — roam no mid ou warde a selva inimiga. Esse espaco livre deve ser aproveitado para visao global.".to_string())
            } else {
                ("👁️ Setup de Visao".to_string(), "Com wave segura para o ADC, invista tempo em visao proativa: tri-bush, entrada da selva deles, rio. Informacao e win condition.".to_string())
            }
        },
        "TOP" => {
            if enemy_jg_dead {
                ("🛡️ Pressione o Top".to_string(), "JG deles morreu — pressione a rota sem medo de gank. Shove para prioridade e potencial de dive.".to_string())
            } else {
                ("🗺️ TP Awareness".to_string(), "Monitore o minimapa para oportunidade de TP em bot. Um TP bem executado em fight de 5x4 pode virar o jogo.".to_string())
            }
        },
        _ => {
            ("🗺️ Controle de Mapa".to_string(), "Identifique qual regiao do mapa esta mais fraca e aplique pressao. Vantagem de mapa e convertida em objetivos.".to_string())
        }
    }
}
