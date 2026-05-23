use super::state::CoachState;

/// Verifica se uma categoria pode disparar nesta partida (abaixo do limite por jogo).
pub(super) fn can_fire(state: &mut CoachState, cat: &str, max_per_game: u32) -> Option<u32> {
    let entry = state.shown_categories.entry(cat.to_string()).or_insert(0);
    if *entry < max_per_game {
        let count = *entry;
        *entry += 1;
        Some(count)
    } else {
        None
    }
}

/// Seleciona qual variante de uma dica com base na contagem e no tempo.
pub(super) fn var_idx(fire_count: u32, game_time: f64, total: usize) -> usize {
    let time_bucket = (game_time as usize) / 120;
    (fire_count as usize + time_bucket) % total
}

pub fn get_role_micro_tip(
    state: &mut CoachState,
    lca_data: &serde_json::Value,
    role: &str,
    game_time: f64,
    profile_opt: Option<&crate::db::player_style_service::PlayerAggregateProfile>,
) -> Option<(String, String)> {
    let active_summ_name = lca_data["activePlayer"]["summonerName"].as_str().unwrap_or("");
    let my_cs = lca_data["allPlayers"].as_array().and_then(|arr| {
        arr.iter().find(|p| p["summonerName"].as_str().unwrap_or("") == active_summ_name)
           .map(|p| p["scores"]["creepScore"].as_i64().unwrap_or(0))
    }).unwrap_or(0);
    let my_kills: i64 = lca_data["allPlayers"].as_array().and_then(|arr| {
        arr.iter().find(|p| p["summonerName"].as_str().unwrap_or("") == active_summ_name)
           .map(|p| p["scores"]["kills"].as_i64().unwrap_or(0))
    }).unwrap_or(0);
    let my_deaths: i64 = lca_data["allPlayers"].as_array().and_then(|arr| {
        arr.iter().find(|p| p["summonerName"].as_str().unwrap_or("") == active_summ_name)
           .map(|p| p["scores"]["deaths"].as_i64().unwrap_or(0))
    }).unwrap_or(0);
    let gold   = lca_data["activePlayer"]["currentGold"].as_f64().unwrap_or(0.0);
    let hp_pct = {
        let hp  = lca_data["activePlayer"]["championStats"]["currentHealth"].as_f64().unwrap_or(1000.0);
        let max = lca_data["activePlayer"]["championStats"]["maxHealth"].as_f64().unwrap_or(1000.0);
        if max > 0.0 { hp / max } else { 1.0 }
    };
    let cs_per_min = if game_time > 60.0 { my_cs as f64 / (game_time / 60.0) } else { 0.0 };

    let hist_cs_per_min = profile_opt.map(|p| p.avg_cs_per_min).unwrap_or(0.0);
    let hist_deaths     = profile_opt.map(|p| p.avg_deaths).unwrap_or(99.0);
    let hist_vision     = profile_opt.map(|p| p.avg_vision_score_per_min).unwrap_or(0.0);

    let new_death = my_deaths > state.prev_deaths;
    if new_death { state.prev_deaths = my_deaths; }

    let cs_gap = if state.last_cs_snapshot_time > 0.0 && (game_time - state.last_cs_snapshot_time) >= 60.0 {
        let delta_cs  = my_cs - state.last_cs_snapshot;
        let delta_min = (game_time - state.last_cs_snapshot_time) / 60.0;
        state.last_cs_snapshot = my_cs;
        state.last_cs_snapshot_time = game_time;
        delta_cs as f64 / delta_min
    } else {
        if state.last_cs_snapshot_time == 0.0 {
            state.last_cs_snapshot = my_cs;
            state.last_cs_snapshot_time = game_time;
        }
        999.0
    };

    let mut lowest_enemy_hp_pct = 1.0_f64;
    let mut lowest_enemy_name   = String::new();
    let mut lowest_enemy_lane   = String::new();
    if let Some(players) = lca_data["allPlayers"].as_array() {
        let my_team = players.iter()
            .find(|p| p["summonerName"].as_str().unwrap_or("") == active_summ_name)
            .map(|p| p["team"].as_str().unwrap_or("ORDER").to_uppercase())
            .unwrap_or_else(|| "ORDER".to_string());
        for p in players {
            if p["team"].as_str().unwrap_or("ORDER").to_uppercase() == my_team { continue; }
            if p["isDead"].as_bool().unwrap_or(false) { continue; }
            let hp  = p["championStats"]["currentHealth"].as_f64().unwrap_or(9999.0);
            let max = p["championStats"]["maxHealth"].as_f64().unwrap_or(9999.0);
            let pct = if max > 0.0 { hp / max } else { 1.0 };
            if pct < lowest_enemy_hp_pct {
                lowest_enemy_hp_pct = pct;
                lowest_enemy_name   = p["championName"].as_str().unwrap_or("Inimigo").to_string();
                lowest_enemy_lane   = p["position"].as_str().unwrap_or("").to_uppercase();
            }
        }
    }

    // =====================================================================
    // DICAS DE ABERTURA — disparam uma vez entre 10-90s
    // =====================================================================
    if game_time >= 10.0 && game_time < 90.0 {
        let role_up = role.to_uppercase();

        let opening = match role_up.as_str() {
            "SUPPORT" | "UTILITY" if !state.sup_bush_dominance_suggested => {
                state.sup_bush_dominance_suggested = true;
                Some((
                    "🌿 Arbusto Lateral".to_string(),
                    "Domine o arbusto — força avanço sem visão e cria ganks.".to_string(),
                ))
            }
            "TOP" => {
                let entry = state.shown_categories.entry("TOP_OPEN".to_string()).or_insert(0);
                if *entry == 0 {
                    *entry += 1;
                    Some((
                        "🗺️ Topo — Início".to_string(),
                        "Congele perto da torre e negue o farm inimigo.".to_string(),
                    ))
                } else { None }
            }
            "MIDDLE" | "MID" => {
                let entry = state.shown_categories.entry("MID_OPEN".to_string()).or_insert(0);
                if *entry == 0 {
                    *entry += 1;
                    Some((
                        "🗺️ Meio — Início".to_string(),
                        "Cubra os arbustos centrais, dois flancos expostos.".to_string(),
                    ))
                } else { None }
            }
            "BOTTOM" | "ADC" => {
                let entry = state.shown_categories.entry("ADC_OPEN".to_string()).or_insert(0);
                if *entry == 0 {
                    *entry += 1;
                    Some((
                        "🎯 Bot — Início".to_string(),
                        "Farm primeiro. Posicione atrás do suporte nas trocas.".to_string(),
                    ))
                } else { None }
            }
            _ => None,
        };

        if let Some((title, text)) = opening {
            state.last_micro_time = game_time;
            return Some((title, text));
        }
    }

    // Cooldown entre micro tips — 45s para manter ritmo sem spam
    if game_time - state.last_micro_time < 45.0 || game_time < 20.0 {
        return None;
    }

    macro_rules! fire {
        ($cat:expr, $max:expr, $last_cat:expr, $title:expr, $text:expr) => {{
            if state.last_tip_category != $last_cat {
                if let Some(n) = can_fire(state, $cat, $max) {
                    let _ = n;
                    state.last_micro_time = game_time;
                    state.last_tip_category = $last_cat.to_string();
                    return Some(($title, $text));
                }
            }
        }};
        ($cat:expr, $max:expr, $last_cat:expr, $variants:expr) => {{
            if state.last_tip_category != $last_cat {
                if let Some(n) = can_fire(state, $cat, $max) {
                    let idx = var_idx(n, game_time, $variants.len());
                    let (t, m): (&str, String) = $variants[idx].clone();
                    state.last_micro_time = game_time;
                    state.last_tip_category = $last_cat.to_string();
                    return Some((t.to_string(), m));
                }
            }
        }};
    }

    // =====================================================================
    // JUNGLE MICRO
    // =====================================================================
    if role.to_uppercase() == "JUNGLE" && state.first_clear_step >= 8 {
        let level = lca_data["activePlayer"]["level"].as_i64().unwrap_or(1);

        if level >= 5 && !state.jg_level5_suggested {
            state.jg_level5_suggested = true;
            fire!("JG_SPIKE", 1, "JG_SPIKE",
                "⚡ Nível 5 — Emboscada".to_string(),
                "Nível 5 — janela de emboscada aberta. Priorize rota com onda avançando.".to_string()
            );
        }

        if game_time >= 270.0 && game_time < 305.0 && !state.jg_dragon_suggested {
            state.jg_dragon_suggested = true;
            fire!("JG_OBJECTIVE", 1, "JG_OBJECTIVE",
                "⚔️ 5:00 — Dois Objetivos".to_string(),
                "Vastilarvas e Dragão simultâneos — escolha um, decida pela composição.".to_string()
            );
        }

        if new_death && my_deaths >= 3 && hist_deaths >= 4.0 {
            fire!("JG_DYING", 2, "JG_DYING", vec![
                ("💀 Jogue Passivo",
                 format!("{} mortes — farme passivo e invada o lado oposto do caçador.", my_deaths)),
                ("💀 Reduza o Risco",
                 format!("{} mortes — limpe a selva sem arriscar ganks por agora.", my_deaths)),
            ]);
        }

        if lowest_enemy_hp_pct < 0.35 && !lowest_enemy_name.is_empty() {
            let lane = match lowest_enemy_lane.as_str() {
                "TOP" => "Top", "MIDDLE" => "Meio", "BOTTOM" => "Bot", _ => ""
            };
            if !lane.is_empty() {
                fire!("JG_GANK", 2, "JG_GANK", vec![
                    ("🎯 Emboscada",
                     format!("{} ({}) a {:.0}% — entre pelo flanco da selva.", lowest_enemy_name, lane, lowest_enemy_hp_pct * 100.0)),
                    ("🎯 Abate Disponível",
                     format!("{} em {} a {:.0}%. Sinalize o aliado antes de entrar.", lane, lowest_enemy_name, lowest_enemy_hp_pct * 100.0)),
                ]);
            }
        }

        if let Some(enemy_jg) = state.enemy_telemetries.iter().find(|t| t.role == "JUNGLE") {
            let fog = enemy_jg.fog_duration;
            let ename = enemy_jg.champion_name.clone();
            if !enemy_jg.is_visible && fog >= 8.0 && fog <= 20.0 && my_deaths == 0 {
                fire!("JG_INVADE", 1, "JG_INVADE", vec![
                    ("🗡️ Invasão",
                     format!("{} sumiu há {:.0}s — campo oposto livre, invada.", ename, fog)),
                ]);
            }
        }

        if cs_per_min < 4.0 && game_time > 300.0 && hist_cs_per_min < 6.5 {
            fire!("JG_FARM", 1, "JG_FARM", vec![
                ("🌿 Farm Baixo",
                 format!("{:.1} CS/min — meta do caçador é 6+. Inclua campos no trajeto.", cs_per_min)),
            ]);
        }

        if gold > 1200.0 {
            fire!("JG_GOLD", 2, "JG_GOLD", vec![
                ("💰 Converta o Ouro",
                 format!("{:.0} de ouro — volte à base entre ondas ou após objetivo.", gold)),
                ("💰 Base",
                 format!("{:.0} de ouro acumulado. Converta em item agora.", gold)),
            ]);
        }

        if cs_per_min >= 6.0 && my_deaths == 0 && my_kills >= 1 {
            fire!("JG_PRAISE", 1, "JG_PRAISE",
                "🔥 Selva Dominante".to_string(),
                format!("{:.1} CS/min, {} abate(s), 0 mortes — converta em objetivo.", cs_per_min, my_kills)
            );
        }
    }

    // =====================================================================
    // TOP LANE MICRO
    // =====================================================================
    if role.to_uppercase() == "TOP" {
        let level = lca_data["activePlayer"]["level"].as_i64().unwrap_or(1);

        if new_death && my_deaths >= 3 && hist_deaths >= 4.0 {
            fire!("TOP_DYING", 2, "TOP_DYING", vec![
                ("☠️ Congele a Onda",
                 format!("{} mortes — congele a onda na torre e farme seguro.", my_deaths)),
                ("☠️ Jogue Defensivo",
                 format!("{} mortes — saia da linha de frente e evite trocas.", my_deaths)),
            ]);
        }

        if lowest_enemy_hp_pct < 0.3 && lowest_enemy_lane == "TOP" {
            fire!("TOP_KILL", 2, "TOP_KILL", vec![
                ("🔪 Janela de Abate",
                 format!("{} a {:.0}% — espere o controle de grupo sair.", lowest_enemy_name, lowest_enemy_hp_pct * 100.0)),
                ("🔪 Troque Agora",
                 format!("{} vulnerável a {:.0}% — inicie quando ele usar escape.", lowest_enemy_name, lowest_enemy_hp_pct * 100.0)),
            ]);
        }

        if let Some(enemy_top) = state.enemy_telemetries.iter().find(|t| t.role == "TOP") {
            let fog = enemy_top.fog_duration;
            let ename = enemy_top.champion_name.clone();
            if !enemy_top.is_visible && fog >= 8.0 && fog <= 30.0 {
                fire!("TOP_MISSING", 2, "TOP_MISSING", vec![
                    ("⚠️ Topo Sumiu",
                     format!("{} fora de visão há {:.0}s — recue para zona segura.", ename, fog)),
                    ("⚠️ Cuidado",
                     format!("{} sumiu há {:.0}s. Fique sob a torre.", ename, fog)),
                ]);
            }
        }

        // Tip proativa de wave control (150-200s) — cobre safe-play gap
        if game_time >= 150.0 && game_time < 200.0 {
            fire!("TOP_WAVE_EARLY", 1, "TOP_WAVE_EARLY",
                "🌊 Controle de Onda".to_string(),
                "Empurre a onda antes do Aronguejo (3:30) e warda o rio.".to_string()
            );
        }

        if level == 6 && my_kills >= my_deaths {
            fire!("TOP_SPIKE", 1, "TOP_SPIKE",
                "🌟 Nível 6".to_string(),
                "R disponível — janela de investida total agora.".to_string()
            );
        }

        if my_kills >= 2 && my_deaths == 0 && hp_pct > 0.65 {
            fire!("TOP_SNOWBALL", 1, "TOP_SNOWBALL", vec![
                ("⚡ Converta",
                 "Vantagem — acumule onda e converta em pressão de torre.".to_string()),
            ]);
        }

        if cs_gap < 4.0 && cs_gap < 999.0 && hist_cs_per_min < 7.0 {
            fire!("TOP_FARM", 1, "TOP_FARM", vec![
                ("🌾 Farm Baixo",
                 format!("{:.0} CS/min abaixo do ideal — priorize o último acerto.", cs_gap)),
            ]);
        }

        if hp_pct < 0.35 && hp_pct > 0.0 && game_time < 900.0 {
            fire!("TOP_HP", 2, "TOP_HP", vec![
                ("❤️ Vida Crítica",
                 "Vida < 35% — saia da linha de frente ou vá à base.".to_string()),
                ("❤️ Recue",
                 "Vida crítica — posicione fora do alcance inimigo.".to_string()),
            ]);
        }

        if gold > 1400.0 && hp_pct > 0.7 {
            fire!("TOP_RECALL", 2, "TOP_RECALL", vec![
                ("💰 Base",
                 format!("{:.0} de ouro — avance sob a torre deles e vá à base.", gold)),
                ("💰 Pico de Item",
                 format!("{:.0} de ouro — acumule onda, empurre e volte à base.", gold)),
            ]);
        }
    }

    // =====================================================================
    // MID LANE MICRO
    // =====================================================================
    if role.to_uppercase() == "MID" || role.to_uppercase() == "MIDDLE" {
        let level = lca_data["activePlayer"]["level"].as_i64().unwrap_or(1);

        if new_death && my_deaths >= 2 && hist_deaths >= 3.5 {
            fire!("MID_DYING", 2, "MID_DYING", vec![
                ("☠️ Jogue Curto",
                 format!("{} mortes — ward nas entradas e jogue pelo lado seguro.", my_deaths)),
                ("☠️ Farme Defensivo",
                 format!("{} mortes — abates sob a torre, recupere pressão.", my_deaths)),
            ]);
        }

        if let Some(enemy_mid) = state.enemy_telemetries.iter().find(|t| t.role == "MID") {
            let fog = enemy_mid.fog_duration;
            let ename = enemy_mid.champion_name.clone();
            if !enemy_mid.is_visible && fog >= 6.0 && fog <= 22.0 {
                fire!("MID_MISSING", 2, "MID_MISSING", vec![
                    ("⚠️ Meio Sumiu",
                     format!("{} sumiu há {:.0}s — avance e puna com pressão de torre.", ename, fog)),
                    ("⚠️ Puna a Rotação",
                     format!("{} rotacionando há {:.0}s. Avance imediatamente.", ename, fog)),
                ]);
            }
        }

        // Tip proativa de roam/rio (120-160s) — cobre safe-play gap
        if game_time >= 120.0 && game_time < 160.0 {
            fire!("MID_RIVER_EARLY", 1, "MID_RIVER_EARLY",
                "🗺️ Visão do Rio".to_string(),
                "Empurre e warda os lados — controle as entradas de gank.".to_string()
            );
        }

        if level == 6 && my_kills >= my_deaths {
            fire!("MID_SPIKE", 1, "MID_SPIKE",
                "🌟 Nível 6 — Pressione".to_string(),
                "R disponível — avance, entre na selva ou rotacione.".to_string()
            );
        }

        if lowest_enemy_hp_pct < 0.35 && (lowest_enemy_lane == "TOP" || lowest_enemy_lane == "BOTTOM") {
            let lane = if lowest_enemy_lane == "TOP" { "Topo" } else { "Bot" };
            fire!("MID_ROAM", 2, "MID_ROAM", vec![
                ("🗺️ Rotacione",
                 format!("{} ({}) a {:.0}% — avance rápido e rotacione.", lowest_enemy_name, lane, lowest_enemy_hp_pct * 100.0)),
                ("🗺️ Impacto Global",
                 format!("Abate disponível no {} — pressão de mapa vale mais.", lane)),
            ]);
        }

        if cs_gap < 5.0 && cs_gap < 999.0 && hist_cs_per_min < 7.5 {
            fire!("MID_FARM", 1, "MID_FARM", vec![
                ("🌾 Farm Baixo",
                 format!("{:.0} CS/min — meta do meio é 7-8. Priorize o último acerto.", cs_gap)),
            ]);
        }

        if my_kills >= 2 && hp_pct > 0.55 {
            fire!("MID_SNOWBALL", 1, "MID_SNOWBALL", vec![
                ("🗺️ Converta",
                 "Dominando — rotacione ou pressione Dragão com o caçador.".to_string()),
            ]);
        }

        if cs_gap >= 7.0 && cs_gap < 999.0 && my_deaths == 0 {
            fire!("MID_PRAISE", 1, "MID_PRAISE",
                "🔥 Farm Impecável".to_string(),
                format!("{:.0} CS/min — vantagem confirmada. Pressione Dragão ou Arauto.", cs_gap)
            );
        }

        if hp_pct < 0.35 && hp_pct > 0.0 {
            fire!("MID_HP", 2, "MID_HP", vec![
                ("❤️ Vida Baixa",
                 "Vida < 35% no meio — recue para a torre imediatamente.".to_string()),
                ("❤️ Posicione na Torre",
                 "Vida crítica no meio — não avance na onda.".to_string()),
            ]);
        }
    }

    // =====================================================================
    // ADC MICRO
    // =====================================================================
    if role.to_uppercase() == "ADC" || role.to_uppercase() == "BOTTOM" {
        let level = lca_data["activePlayer"]["level"].as_i64().unwrap_or(1);

        if new_death && my_deaths >= 2 && hist_deaths >= 3.5 {
            fire!("ADC_DYING", 2, "ADC_DYING", vec![
                ("☠️ Posicionamento",
                 format!("{} mortes — posicione atrás do suporte, fora do alcance.", my_deaths)),
                ("☠️ Alcance Máximo",
                 format!("{} mortes — mantenha distância máxima de ataque.", my_deaths)),
            ]);
        }

        if lowest_enemy_hp_pct < 0.25 && lowest_enemy_lane == "BOTTOM" {
            fire!("ADC_KILL", 2, "ADC_KILL", vec![
                ("🔪 Abate",
                 format!("{} a {:.0}% — ataque agressivo quando suporte travar CC.", lowest_enemy_name, lowest_enemy_hp_pct * 100.0)),
                ("🔪 Confirme",
                 format!("{} a {:.0}% — CC do suporte, você finaliza.", lowest_enemy_name, lowest_enemy_hp_pct * 100.0)),
            ]);
        }

        // Tip proativa de posicionamento (90-120s) — antes do Nível 2 all-in
        if game_time >= 90.0 && game_time < 120.0 {
            fire!("ADC_POS_EARLY", 1, "ADC_POS_EARLY",
                "🛡️ Posicionamento Early".to_string(),
                "Fique atrás do suporte nas trocas — Nível 2 é a janela de all-in.".to_string()
            );
        }

        if cs_gap < 5.0 && cs_gap < 999.0 && hist_cs_per_min < 8.0 {
            fire!("ADC_FARM", 1, "ADC_FARM", vec![
                ("🌾 Farm Baixo",
                 format!("{:.0} CS/min — meta do atirador é 8+. Priorize o último acerto.", cs_gap)),
            ]);
        }

        if my_kills >= 2 && game_time < 600.0 {
            fire!("ADC_SNOWBALL", 1, "ADC_SNOWBALL", vec![
                ("💥 Destrua a Torre",
                 "Dominando — destrua a torre e colete pratos de ouro.".to_string()),
            ]);
        }

        if hp_pct < 0.4 && hp_pct > 0.0 && game_time < 900.0 {
            fire!("ADC_HP", 2, "ADC_HP", vec![
                ("❤️ Vida Baixa",
                 "Vida < 40% — o suporte absorve, você mantém alcance máximo.".to_string()),
                ("❤️ Posicione Atrás",
                 "Vida crítica — use o suporte como escudo nas trocas.".to_string()),
            ]);
        }

        if game_time > 120.0 && game_time < 500.0 && my_deaths == 0 && cs_per_min > 5.0 {
            fire!("ADC_KITE", 1, "ADC_KITE",
                "⚡ Orbwalk".to_string(),
                "Ataque básico → mova imediatamente → próximo ataque sem parar.".to_string()
            );
        }

        if level == 6 && my_deaths <= 1 {
            fire!("ADC_SPIKE", 1, "ADC_SPIKE",
                "🌟 Nível 6".to_string(),
                "R disponível — prepare investida total com o suporte.".to_string()
            );
        }

        if game_time > 900.0 && my_deaths >= 2 && hist_deaths >= 4.0 {
            fire!("ADC_LATE", 1, "ADC_LATE", vec![
                ("⚡ Sobreviva em Luta",
                 "Nunca fique parado em teamfight — ataque e mova constantemente.".to_string()),
            ]);
        }
    }

    // =====================================================================
    // SUPPORT MICRO
    // =====================================================================
    if role.to_uppercase() == "SUPPORT" || role.to_uppercase() == "UTILITY" {
        let level = lca_data["activePlayer"]["level"].as_i64().unwrap_or(1);

        if level == 2 && game_time < 130.0 {
            fire!("SUP_LV2", 1, "SUP_LV2",
                "⚡ Nível 2 — Investida".to_string(),
                "Nível 2 antes deles — janela de investida total agora.".to_string()
            );
        }

        if new_death && my_deaths >= 3 && hist_deaths >= 4.0 {
            fire!("SUP_DYING", 2, "SUP_DYING", vec![
                ("☠️ Você é o Escudo",
                 format!("{} mortes — posicione para absorver dano, não iniciar.", my_deaths)),
                ("☠️ Proteja o Atirador",
                 format!("{} mortes — atirador exposto sem você. Recue para proteger.", my_deaths)),
            ]);
        }

        if lowest_enemy_lane == "BOTTOM" && lowest_enemy_hp_pct > 0.55 && hp_pct > 0.5 && game_time < 600.0 {
            fire!("SUP_POKE", 2, "SUP_POKE", vec![
                ("💢 Poke Antes do All-in",
                 format!("{} a {:.0}% — poke primeiro, all-in depois.", lowest_enemy_name, lowest_enemy_hp_pct * 100.0)),
                ("💢 Desgaste",
                 format!("{} a {:.0}%. Pressão constante force volta à base.", lowest_enemy_name, lowest_enemy_hp_pct * 100.0)),
            ]);
        }

        if game_time > 90.0 && game_time < 480.0 && hp_pct > 0.55 && gold > 350.0 {
            fire!("SUP_WARD_ADV", 1, "SUP_WARD_ADV", vec![
                ("👁️ Ward Antes de Avançar",
                 "Ward no arbusto triângulo inimigo antes de cruzar a meia rota.".to_string()),
            ]);
        }

        if lowest_enemy_hp_pct < 0.3 && lowest_enemy_lane == "BOTTOM" {
            fire!("SUP_ENGAGE", 2, "SUP_ENGAGE", vec![
                ("🎯 Inicie Agora",
                 format!("{} a {:.0}% — CC completo e sinalize o atirador.", lowest_enemy_name, lowest_enemy_hp_pct * 100.0)),
                ("🎯 Janela Aberta",
                 format!("{} a {:.0}% — CC abre, atirador fecha o abate.", lowest_enemy_name, lowest_enemy_hp_pct * 100.0)),
            ]);
        }

        {
            let mut adc_hp_pct = 1.0_f64;
            if let Some(players) = lca_data["allPlayers"].as_array() {
                let my_team_val = players.iter()
                    .find(|p| p["summonerName"].as_str().unwrap_or("") == active_summ_name)
                    .map(|p| p["team"].as_str().unwrap_or("ORDER").to_uppercase())
                    .unwrap_or_else(|| "ORDER".to_string());
                for p in players {
                    if p["team"].as_str().unwrap_or("ORDER").to_uppercase() == my_team_val
                        && p["position"].as_str().unwrap_or("").to_uppercase() == "BOTTOM" {
                        let hp = p["championStats"]["currentHealth"].as_f64().unwrap_or(1000.0);
                        let max = p["championStats"]["maxHealth"].as_f64().unwrap_or(1000.0);
                        if max > 0.0 { adc_hp_pct = hp / max; }
                    }
                }
            }
            if adc_hp_pct < 0.3 && adc_hp_pct > 0.0 {
                fire!("SUP_PEEL", 2, "SUP_PEEL", vec![
                    ("🛡️ Proteja o Atirador",
                     "Atirador crítico — CC no atacante, não inicie agora.".to_string()),
                    ("🛡️ Escudo Agora",
                     "Atirador com vida crítica — interponha-se e use CC defensivo.".to_string()),
                ]);
            }
        }

        {
            let mut enemy_bot_dead = false;
            if let Some(players) = lca_data["allPlayers"].as_array() {
                let my_team_val = players.iter()
                    .find(|p| p["summonerName"].as_str().unwrap_or("") == active_summ_name)
                    .map(|p| p["team"].as_str().unwrap_or("ORDER").to_uppercase())
                    .unwrap_or_else(|| "ORDER".to_string());
                for p in players {
                    let dead = p["isDead"].as_bool().unwrap_or(false);
                    let pos  = p["position"].as_str().unwrap_or("").to_uppercase();
                    if p["team"].as_str().unwrap_or("ORDER").to_uppercase() != my_team_val
                        && dead && (pos == "BOTTOM" || pos == "UTILITY") {
                        enemy_bot_dead = true;
                    }
                }
            }
            if enemy_bot_dead && game_time > 180.0 && hp_pct > 0.5 {
                fire!("SUP_ROAM", 2, "SUP_ROAM", vec![
                    ("🗺️ Rotacione",
                     "Bot inimigo morto — pressione o meio ou ward na selva deles.".to_string()),
                    ("🗺️ Espaço Livre",
                     "Inimigo do bot morto. Ward no arbusto triplo inimigo agora.".to_string()),
                ]);
            }
        }

        if gold > 650.0 && game_time > 120.0 && hist_vision < 0.65 {
            fire!("SUP_VISION", 2, "SUP_VISION", vec![
                ("👁️ Visão",
                 format!("{:.0} de ouro — ward 60s antes do objetivo vale muito.", gold)),
                ("👁️ Sentinela",
                 format!("{:.0} de ouro. Invista em visão antes de avançar.", gold)),
            ]);
        }

        if my_kills >= 3 && game_time < 600.0 {
            fire!("SUP_KILLS", 1, "SUP_KILLS",
                "⚠️ Deixe o Atirador Escalar".to_string(),
                "Suporte acumulando abates — deixe o atirador pegar os finais.".to_string()
            );
        }

        if game_time > 200.0 && game_time < 450.0 && my_deaths == 0 && hp_pct > 0.6 && lowest_enemy_hp_pct > 0.5 {
            fire!("SUP_MAP", 1, "SUP_MAP", vec![
                ("🗺️ Pressão de Mapa",
                 "Situação estável no bot — pressione o meio ou ward na selva.".to_string()),
            ]);
        }
    }

    None
}
