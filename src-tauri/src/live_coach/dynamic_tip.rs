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

    // --- ALERTAS DE RENASCIMENTO DE CAMPOS DA SELVA (JUNGLE RESPAWN WARNINGS) ---
    if role.to_uppercase() == "JUNGLE" {
        // Alerta Aronguejo (Scuttle): 3:20 → nasce 3:30 (210s)
        if game_time >= 200.0 && game_time < 215.0 && !state.respawn_warnings_sent.contains("scuttle_1") {
            state.respawn_warnings_sent.insert("scuttle_1".to_string());
            return Some((
                "🦀 Aronguejo — 10s".to_string(),
                "Aronguejo em 10s. Posicione no lado de baixo do rio agora — o escudo de visão vale muito no early.".to_string()
            ));
        }
        // Segundo Aronguejo: ~5:50 (3:30 inicial + 2:30 respawn = 360s)
        if game_time >= 350.0 && game_time < 365.0 && !state.respawn_warnings_sent.contains("scuttle_2") {
            state.respawn_warnings_sent.insert("scuttle_2".to_string());
            return Some((
                "🦀 Segundo Aronguejo — 10s".to_string(),
                "Segundo Aronguejo em 10s. Pegue se o Dragão estiver vivo — controle de visão dobrado no Rio.".to_string()
            ));
        }
        // Alerta Buffs: 6:25 (nasce 6:40)
        if game_time >= 385.0 && game_time < 395.0 && !state.respawn_warnings_sent.contains("buffs_1") {
            state.respawn_warnings_sent.insert("buffs_1".to_string());
            return Some((
                "🛡️ Campos de Bônus — 15s".to_string(),
                "Red/Blue renascendo em 15s. Rastreie o caçador inimigo — se ele sumiu, o campo dele está vulnerável para invasão.".to_string()
            ));
        }
        // Pre-posicionamento Dragon: alerta ~60-70s antes do 2º Dragon (aprox 10:00 = 600s)
        if game_time >= 540.0 && game_time < 570.0 && !state.dragon_setup_warned {
            state.dragon_setup_warned = true;
            return Some((
                "🐉 Setup Dragão — 1min".to_string(),
                "Dragão em ~1min. Ward no arbusto do rio agora. Não inicie sem visão — rasteje o Arauto/JG inimigo antes.".to_string()
            ));
        }
        // Voidgrubs last chance: despawnam às 14:00 (840s), alerta em 13:20 (800s)
        if game_time >= 800.0 && game_time < 840.0 && !state.grubs_last_chance_warned {
            state.grubs_last_chance_warned = true;
            return Some((
                "🐛 Vastilarvas — Última Chance".to_string(),
                "Vastilarvas somem às 14:00. Se faltam acúmulos, abandone a rota agora — cada stack vale pressão de estrutura no late.".to_string()
            ));
        }
        // Aviso de Baron Nashusor: nasce às 20:00 (1200s), alerta aos 19:00 (1140s)
        if game_time >= 1140.0 && game_time < 1160.0 && !state.baron_warned {
            state.baron_warned = true;
            return Some((
                "💜 Barão em 60s — Agrupe".to_string(),
                "Barão em 60s. Termine a onda, empurre para dentro e agrupe no Rio AGORA — quem chega morto não conta.".to_string()
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
    if let Some(players) = lca_data["allPlayers"].as_array() {
        for p in players {
            if p["summonerName"].as_str().unwrap_or("") == active_summ_name {
                team_side = p["team"].as_str().unwrap_or("ORDER").to_uppercase();
                active_player_kills = p["scores"]["kills"].as_i64().unwrap_or(0);
                active_player_cs = p["scores"]["creepScore"].as_i64().unwrap_or(0);
                active_player_deaths = p["scores"]["deaths"].as_i64().unwrap_or(0);
                break;
            }
        }
    }


    // --- DICA DE MACRO — 1 por role, no momento certo, uma vez por partida ---

    // MID: roaming e o conceito mais impactante que mid baixo ELO ignora
    let is_mid_role = ["MID", "MIDDLE"].contains(&role.to_uppercase().as_str());
    if is_mid_role && game_time >= 360.0 && game_time < 480.0 && !state.mid_roaming_suggested {
        state.mid_roaming_suggested = true;
        return Some((
            "\u{1F5FA} Meio — Hora de Rotar".to_string(),
            "Avance a onda até a torre deles e rotacione — top para objetivo ou bot para Dragão.".to_string()
        ));
    }

    // SUPPORT: dominar o arbusto é o principal diferencial de suportes iniciantes
    let is_sup_role = ["SUPPORT", "UTILITY"].contains(&role.to_uppercase().as_str());
    if is_sup_role && game_time >= 120.0 && game_time < 200.0 && !state.sup_bush_dominance_suggested {
        state.sup_bush_dominance_suggested = true;
        return Some((
            "\u{1F33F} Domine o Arbusto".to_string(),
            "Entre no arbusto lateral — força o ADC inimigo a farmar sem visão e cria ameaça de emboscada.".to_string()
        ));
    }

    // TOP: Vastilarvas são o objetivo mais ignorado por tops iniciantes
    let is_top_role = role.to_uppercase() == "TOP";
    if is_top_role && game_time >= 270.0 && game_time < 305.0 && !state.top_grubs_priority_suggested {
        state.top_grubs_priority_suggested = true;
        return Some((
            "\u{1F41B} Avance Antes das Vastilarvas".to_string(),
            "Vastilarvas em 30s — empurre a onda agora e vá com o caçador.".to_string()
        ));
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
                    // Dica urgente com ação concreta baseada no tempo de névoa
                    let action = if tel.fog_duration < 15.0 {
                        "Recue IMEDIATAMENTE para a torre."
                    } else {
                        "Não avance na wave — posição segura sob torre."
                    };
                    return Some((
                        format!("🚨 {} Sumiu — Perigo!", role_display),
                        format!("{} ({}) fora de visão há {:.0}s. {} Não vale a kill se você morrer no processo.", role_display, tel.champion_name, tel.fog_duration, action)
                    ));
                }
            }
        }
    }

    // --- PERIODIC MINIMAP CHECK REMINDERS (a cada 3 min, máx 2 por partida) ---
    if game_time - state.last_minimap_time >= 180.0 && game_time > 90.0 {
        if let Some(n) = can_fire(state, "MINIMAP", 2) {
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
                            "🎯 Janela de Gank — JG".to_string(),
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
    } else if game_time - state.last_alert_time >= 120.0 && can_fire(state, "ADV_CONTEXT", 4).is_some() {
        state.last_alert_time = game_time;

        // --- ADVERSARIAL MATCHUP FEEDBACK (baseado em dados reais da tab) ---
        if !enemy_laner_champ.is_empty() && game_time > 120.0 {
            if enemy_laner_kills >= active_player_kills + 2 {
                state.active_recommendation = Some(ActiveRec::PlaySafe {
                    enemy_champ: enemy_laner_champ.clone(),
                    start_time: game_time,
                    initial_deaths: active_player_deaths,
                });
                let gold_gap = (enemy_laner_kills - active_player_kills) * 300;
                return Some((
                    format!("⚔️ {} em Vantagem — Jogue Seguro", enemy_laner_champ),
                    format!("{} kills à frente (~{}g de vantagem). Fique sob a torre, farm first — nunca inicie sem JG visível. Equalize antes de arriscar.", enemy_laner_kills, gold_gap)
                ));
            } else if active_player_kills >= enemy_laner_kills + 2 {
                return Some((
                    "🔥 Vantagem — Converta Agora".to_string(),
                    format!("{} kills de frente. NÃO banque — empurre para Tier 1, chame o JG e force o próximo objetivo. Kill gold sem estrutura não fecha o jogo.", active_player_kills)
                ));
            } else if enemy_laner_cs > active_player_cs + 20 {
                return Some((
                    "🌾 CS Gap — Recupere o Farm".to_string(),
                    format!("{} minions de diferença (~{}g a menos). Last hit em vez de trades — cada wave perdida é 200g que o inimigo converte em item.", enemy_laner_cs - active_player_cs, (enemy_laner_cs - active_player_cs) / 15 * 300)
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

        if !fed_enemy_champ.is_empty() && game_time > 180.0 {
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
                "Vastilarvas nascem às 5:00 — EMPURRE A WAVE agora, entre na selva com o JG e leve os stacks. Peça priority 30s antes."
            } else {
                "Arauto do Vale ativo — empurre para dentro, teleporte para o Arauto e quebre a Tier 1 inimiga. É a melhor troca de recursos do mapa."
            };
            match current_phase_id.as_str() {
                "EARLY_1" => ("🗺️ Topo — Early".to_string(), "Pegue Nível 2 na 3ª minion maga. Domine o arbusto de cima para pressão de zona. Ward no rio aos 2:45 — custo: 75g, valor: vida.".to_string()),
                "EARLY_2" => ("🎯 Objetivo no Topo!".to_string(), top_early2_tip.to_string()),
                "MID_1" => ("🔀 Split Push Ativo".to_string(), "Empurre a rota OPOSTA ao Barão/Dragão. Força 2 inimigos a defender, criando 3x4 para o time. TP pronto = janela de teleporte para lutas.".to_string()),
                _ => ("⚔️ Late — Flanqueie ou Peele".to_string(), "Escolha agora: flanquear pela retaguarda com TP e executar o carry inimigo, OU absorver dano e proteger o seu. Ambos os caminhos fecham o jogo.".to_string())
            }
        },
        "JUNGLE" => {
            let obj_tip = if game_time < 480.0 {
                "Vastilarvas (top) e Dragão (bot) disponíveis. Vença a moeda: stacks de Voidgrub dão pressão de estrutura no late, Dragão dá escalamento. Decida baseado na composição."
            } else {
                "Arauto do Vale ativo. Garanta visão na Fenda dos Ventos com Sweeper ANTES de iniciar — nunca inicie sem ver o JG inimigo no minimapa."
            };
            match current_phase_id.as_str() {
                "EARLY_1" => ("📍 Trackeia o Pathing".to_string(), "Qual rota inimiga chegou atrasada na wave? Isso revela por onde o JG deles começou. Mirre o camp oposto e preveja o próximo gank.".to_string()),
                "EARLY_2" => ("🎯 Objetivos Disponíveis".to_string(), obj_tip.to_string()),
                "MID_1" => ("👁️ Controle Rio — Sweeper".to_string(), "Ative o Sweeper no rio agora. Cada sentinela inimiga removida é informação que você retira do adversário. Jogue com quem estiver carregando.".to_string()),
                _ => ("⚡ Não Morra Antes do Objetivo".to_string(), "Barão/Ancião são decisivos. Se você morrer antes, o time perde o objetivo E o fight. Entre APENAS com vida >50% e o inimigo sem summ chave.".to_string())
            }
        },
        "MID" | "MIDDLE" => {
            match current_phase_id.as_str() {
                "EARLY_1" => ("🗺️ Meio — Controle do Rio".to_string(), "Pegue Nível 2 primeiro (3ª minion maga). Ward nas entradas do rio nos 2 lados — você é o pivot do mapa, informação é obrigação.".to_string()),
                "EARLY_2" => ("🗺️ Shove e Roame".to_string(), "Empurre a wave ATÉ a torre deles e IMEDIATAMENTE rotacione — top para pressão de objetivo, bot para Dragon. Não fique parado esperando.".to_string()),
                "MID_1" => ("🌾 Rota Lateral Livre".to_string(), "ADC foi pro Meio? Farm a rota bot com segurança — são 150+ minions acumulados = 3000g para você. Visão antes de avançar.".to_string()),
                _ => ("🎯 Pick Comp — Névoa".to_string(), "Jogue na névoa de guerra. Um flanco bem executado com R + CC abre teamfight sem troca. Não avance com 5 inimigos visíveis.".to_string())
            }
        },
        "ADC" | "BOTTOM" => {
            match current_phase_id.as_str() {
                "EARLY_1" => ("🎯 Atirador — Nível 2 Primeiro".to_string(), "Pegue Nível 2 na wave 1 + 3 magos. Se chegar antes: all-in imediato. Se chegar depois: fique safe sob tower e farme.".to_string()),
                "EARLY_2" => ("🌾 Farm — Barricadas Seguras".to_string(), "Pegue plates de torre com o suporte por perto. Cada plate = 160g. Não cruce a meia rota sem ward no tri-bush — JG pode flanquear.".to_string()),
                "MID_1" => ("🗺️ Rotacione pro Meio".to_string(), "Vá pro Mid com o suporte — é a rota mais segura para farm em grupo. Bot vazia por muito tempo = gank gratuito. Mova com visão.".to_string()),
                _ => ("⚔️ Teamfight — Posicionamento".to_string(), "NUNCA fique parado em teamfight. Ataque o alvo mais próximo enquanto se move para trás da linha de frente. Sua vida vale mais que a kill.".to_string())
            }
        },
        "SUPPORT" | "UTILITY" => {
            match current_phase_id.as_str() {
                "EARLY_1" => ("🌿 Suporte — Arbusto Lateral".to_string(), "Domine o arbusto lateral: force o ADC inimigo a last hit sob pressão ou zoneie completamente. Cada minion perdido é 20g que o ADC não tem.".to_string()),
                "EARLY_2" => ("🗺️ Roam ou Ward".to_string(), "ADC está seguro e com wave congelada? Roame pro Mid ou plante ward na entrada da selva inimiga. Informação vale mais que presença passiva no bot.".to_string()),
                "MID_1" => ("👁️ Visão Antes do Objetivo".to_string(), "Comece a plantar wards 90s ANTES do próximo objetivo. Tri-bush, fenda do Barão/Dragão e entrada da selva — se você chegou 5s antes deles, venceu o objetivo.".to_string()),
                _ => ("🛡️ Peeling Total".to_string(), "Sua função agora: salvar o carry. CC no assassino que pula, Exhaust no fed inimigo, escudo/heal no último segundo. Não inicie — REAJA.".to_string())
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
        let names_str = dead_enemy_names[..dead_enemy_count.min(3)].join(", ");
        return (
            format!("🔥 {}x0 — Inicie Objetivo AGORA", dead_enemy_count),
            format!("{} mortos. Vá ao Baron/Dragon IMEDIATAMENTE — cada segundo que espera eles voltam com vida cheia e a janela fecha.", names_str)
        );
    }

    // JG morto = selva free para invadir
    if enemy_jg_dead {
        return (
            "⚔️ JG Inimigo Morto — Invada".to_string(),
            "Caçador inimigo morto — selva dele está LIVRE. Entre pelo lado oposto ao seu JG, roube Red/Blue/campos. 120-170g por campo sem risco de combate.".to_string()
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
            "🐉 Dragon — Inicie Agora".to_string(),
            "ADC e Suporte inimigos mortos — Dragon sem contestação real. Chame o JG, confirme visão no arbusto e inicie imediatamente. Não espere vida perfeita.".to_string()
        );
    }
    if enemy_bot_dead && game_time > 180.0 {
        return (
            "🐉 Dragon — Janela Aberta".to_string(),
            "ADC inimigo morto — Dragon com contestação reduzida. Ward no arbusto do rio antes de iniciar; Suporte deles ainda vivo pode tentar roubar.".to_string()
        );
    }

    // Mid morto = pressione a torre mid
    if enemy_mid_dead {
        return (
            "🎯 Mid Morto — Vá À Torre".to_string(),
            "Mid inimigo morto — shove imediato da wave e ataque a torre. Cada tiro de torre que você pega é ouro que vai para o seu time. Não volte à base agora.".to_string()
        );
    }

    // Se algum inimigo morreu mas não é uma janela de objetivo clara
    if !dead_enemy_names.is_empty() && game_time > 300.0 {
        return (
            format!("🔥 {} Morto — Converta", dead_enemy_names[0]),
            format!("{} morto por ~20s. Escolha: torre mais próxima, campo inimigo livre, ou wave lateral adiantada. Não fique parado esperando.", dead_enemy_names[0])
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
                ("🗺️ Farme a Rota Lateral".to_string(), "Shove a wave do meio e farme a rota lateral mais próxima. Ouro flutuante no mapa com visão segura.".to_string())
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
