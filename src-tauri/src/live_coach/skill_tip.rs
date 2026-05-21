use super::state::CoachState;

fn get_champion_level_up_advice(
    champion: &str,
    level: i64,
    q_lv: i32,
    w_lv: i32,
    e_lv: i32,
    r_lv: i32,
) -> Option<(String, String)> {
    let sum_levels = q_lv + w_lv + e_lv + r_lv;
    if sum_levels >= level as i32 {
        return None;
    }

    // Determine the champion's ideal start and max priorities
    let (start_1, start_2, start_3, priority) = match champion.to_lowercase().as_str() {
        "amumu" => ("W", "E", "Q", vec!["E", "Q", "W"]),
        "lillia" => ("Q", "W", "E", vec!["Q", "W", "E"]),
        "karthus" => ("Q", "E", "W", vec!["Q", "W", "E"]),
        "fiddlesticks" => ("W", "E", "Q", vec!["W", "Q", "E"]),
        "diana" => ("W", "Q", "E", vec!["Q", "W", "E"]),
        "evelynn" => ("Q", "W", "E", vec!["Q", "E", "W"]),
        "nidalee" => ("W", "Q", "E", vec!["Q", "E", "W"]),
        "gragas" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "ivern" => ("E", "Q", "W", vec!["E", "Q", "W"]),
        "elise" => ("W", "Q", "E", vec!["Q", "W", "E"]),
        "ekko" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "taliyah" => ("Q", "W", "E", vec!["Q", "E", "W"]),
        "zac" => ("W", "E", "Q", vec!["E", "W", "Q"]),
        "nunu" => ("Q", "W", "E", vec!["Q", "E", "W"]),
        "sejuani" => ("E", "W", "Q", vec!["W", "Q", "E"]),
        "rammus" => ("W", "Q", "E", vec!["Q", "W", "E"]),
        "shaco" => ("W", "Q", "E", vec!["E", "Q", "W"]),
        "viego" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "briar" => ("W", "Q", "E", vec!["W", "Q", "E"]),
        "olaf" => ("Q", "W", "E", vec!["Q", "E", "W"]),
        "xin zhao" | "xinzhao" => ("E", "Q", "W", vec!["W", "E", "Q"]),
        "lee sin" | "leesin" => ("W", "Q", "E", vec!["Q", "W", "E"]),
        "graves" => ("E", "Q", "W", vec!["Q", "E", "W"]),
        "nocturne" => ("Q", "W", "E", vec!["Q", "W", "E"]),
        "master yi" | "masteryi" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "warwick" => ("Q", "W", "E", vec!["W", "Q", "E"]),
        "hecarim" => ("Q", "W", "E", vec!["Q", "W", "E"]),
        "rengar" => ("Q", "W", "E", vec!["Q", "E", "W"]),
        "riven" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "yasuo" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "caitlyn" => ("Q", "W", "E", vec!["Q", "E", "W"]),
        "ashe" => ("W", "Q", "E", vec!["W", "Q", "E"]),
        "ezreal" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "vayne" => ("Q", "W", "E", vec!["Q", "W", "E"]),
        "renekton" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "aatrox" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "lux" => ("E", "Q", "W", vec!["E", "Q", "W"]),
        "ahri" => ("Q", "W", "E", vec!["Q", "W", "E"]),
        "zed" => ("Q", "W", "E", vec!["Q", "E", "W"]),
        "katarina" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "garen" => ("Q", "E", "W", vec!["E", "Q", "W"]),
        "darius" => ("Q", "W", "E", vec!["Q", "E", "W"]),
        "fiora" => ("Q", "W", "E", vec!["Q", "E", "W"]),
        "kayn" => ("Q", "W", "E", vec!["Q", "W", "E"]),
        "twitch" => ("E", "Q", "W", vec!["E", "Q", "W"]),
        "kaisa" | "kai'sa" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "lucian" => ("Q", "E", "W", vec!["Q", "E", "W"]),
        "jhin" => ("Q", "W", "E", vec!["Q", "W", "E"]),
        "thresh" => ("E", "Q", "W", vec!["E", "W", "Q"]),
        "lulu" => ("E", "Q", "W", vec!["E", "W", "Q"]),
        "nami" => ("W", "Q", "E", vec!["W", "E", "Q"]),
        "janna" => ("W", "Q", "E", vec!["W", "E", "Q"]),
        "soraka" => ("Q", "W", "E", vec!["W", "Q", "E"]),
        "morgana" => ("W", "Q", "E", vec!["Q", "E", "W"]),
        "blitzcrank" => ("Q", "W", "E", vec!["Q", "W", "E"]),
        _ => ("", "", "", Vec::new()),
    };

    if priority.is_empty() {
        return Some((
            format!("⬆️ Nível {} — Skill", level),
            "Ponto de skill disponível — escolha pela build atual.".to_string()
        ));
    }

    if level == 6 {
        return Some((
            "🌟 R — Level 6".to_string(),
            "Evolua o R agora — maior power spike do early game.".to_string()
        ));
    }
    if level == 11 {
        return Some((
            "🌟 R — Level 11".to_string(),
            "Evolua o R para nível 2 — impacto em teamfight dobra.".to_string()
        ));
    }
    if level == 16 {
        return Some((
            "🌟 R — Level 16".to_string(),
            "R nível máximo — use sempre que o timing permitir.".to_string()
        ));
    }

    if level == 1 {
        return Some((
            format!("⬆️ Nível 1 — [{}]", start_1),
            format!("[{}] primeiro — define o padrão de farm e primeiro trade.", start_1)
        ));
    }
    if level == 2 {
        return Some((
            format!("⬆️ Nível 2 — [{}]", start_2),
            format!("[{}] completa o combo. Janela de all-in se ficou 2 primeiro.", start_2)
        ));
    }
    if level == 3 {
        return Some((
            format!("⬆️ Nível 3 — [{}]", start_3),
            format!("Kit completo. Max: {} > {} > {}.", priority[0], priority[1], priority[2])
        ));
    }

    for skill in &priority {
        let current_lv = match *skill {
            "Q" => q_lv,
            "W" => w_lv,
            "E" => e_lv,
            _ => 0,
        };
        let max_at_level = std::cmp::min(5, (level as i32 - r_lv - 1) / 2 + 1);
        if current_lv < max_at_level {
            return Some((
                format!("⬆️ Nível {} — [{}]", level, skill),
                format!("Evolua [{}] — prioridade {} > {} > {}.", skill, priority[0], priority[1], priority[2])
            ));
        }
    }

    Some((
        format!("⬆️ Nível {} — [{}]", level, priority[0]),
        format!("[{}] é a prioridade de max agora.", priority[0])
    ))
}


/// Verifica se há ponto de skill não gasto e retorna a dica de qual evoluir.
/// Esta função tem cooldown próprio (5s) e deve ser chamada INDEPENDENTE do cooldown global
/// de coaching, pois level-ups são time-sensitive — o jogador precisa saber imediatamente.
pub fn get_skill_levelup_tip(
    state: &mut CoachState,
    lca_data: &serde_json::Value,
    game_time: f64,
) -> Option<(String, String)> {
    // Cooldown próprio de 5s para não re-disparar se o jogador já upou a skill
    if game_time - state.last_levelup_emit_time < 5.0 {
        return None;
    }

    if game_time < 2.0 { return None; }

    let active_summ = lca_data["activePlayer"]["summonerName"].as_str().unwrap_or("");
    let active_base = active_summ.split('#').next().unwrap_or(active_summ).to_lowercase();
    let champ_owned = lca_data["activePlayer"]["championName"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            lca_data["allPlayers"].as_array()?.iter()
                .find(|p| {
                    let p_name = p["summonerName"].as_str().unwrap_or("");
                    let p_base = p_name.split('#').next().unwrap_or(p_name).to_lowercase();
                    p_name == active_summ || (!active_base.is_empty() && p_base == active_base)
                })
                .and_then(|p| p["championName"].as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();
    let champ = champ_owned.as_str();
    if champ.is_empty() { return None; }

    let level = lca_data["activePlayer"]["level"].as_i64().unwrap_or(1);
    let abilities = &lca_data["activePlayer"]["abilities"];
    let q_lv = abilities["Q"]["abilityLevel"].as_i64().unwrap_or(0) as i32;
    let w_lv = abilities["W"]["abilityLevel"].as_i64().unwrap_or(0) as i32;
    let e_lv = abilities["E"]["abilityLevel"].as_i64().unwrap_or(0) as i32;
    let r_lv = abilities["R"]["abilityLevel"].as_i64().unwrap_or(0) as i32;
    let sum_levels = q_lv + w_lv + e_lv + r_lv;

    // Só dispara se há ponto de skill pendente E ainda não sugerimos para este nível
    if sum_levels >= level as i32 || (level as i32) <= state.last_suggested_level_up {
        return None;
    }

    // ── Usa skill order do banco se disponível (prioridade sobre hardcoded) ──
    if !state.db_skill_order.is_empty() {
        // db_skill_order[level-1] = qual skill evoluir neste nível (0-indexed)
        let idx = (level as usize).saturating_sub(1);
        if let Some(skill) = state.db_skill_order.get(idx) {
            let skill = skill.to_uppercase();
            // R nos níveis 6/11/16 → sempre correto
            let title = if level == 6 || level == 11 || level == 16 {
                format!("🌟 R — Nível {}", level)
            } else {
                format!("⬆️ Nível {} — [{}]", level, skill)
            };
            let text = if level == 6 || level == 11 || level == 16 {
                "Evolua o R agora — maior power spike do jogo.".to_string()
            } else {
                format!("Evolua [{}] — ordem recomendada pelo meta.", skill)
            };
            state.last_suggested_level_up = level as i32;
            state.last_levelup_emit_time = game_time;
            return Some((title, text));
        }
    }

    if let Some((title, text)) = get_champion_level_up_advice(champ, level, q_lv, w_lv, e_lv, r_lv) {
        state.last_suggested_level_up = level as i32;
        state.last_levelup_emit_time = game_time;
        return Some((title, text));
    }

    None
}
