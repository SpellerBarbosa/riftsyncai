use super::state::CoachState;

/// Verifica se há ponto de skill não gasto e retorna a dica de qual evoluir.
/// Usa exclusivamente a ordem do banco (db_skill_order). Se não houver dados,
/// avisa genericamente que há um ponto disponível.
pub fn get_skill_levelup_tip(
    state: &mut CoachState,
    lca_data: &serde_json::Value,
    game_time: f64,
) -> Option<(String, String)> {
    if game_time - state.last_levelup_emit_time < 1.0 { return None; }
    if game_time < 2.0 { return None; }

    let level = lca_data["activePlayer"]["level"].as_i64().unwrap_or(1);
    let abilities = &lca_data["activePlayer"]["abilities"];
    let q_lv = abilities["Q"]["abilityLevel"].as_i64().unwrap_or(0) as i32;
    let w_lv = abilities["W"]["abilityLevel"].as_i64().unwrap_or(0) as i32;
    let e_lv = abilities["E"]["abilityLevel"].as_i64().unwrap_or(0) as i32;
    let r_lv = abilities["R"]["abilityLevel"].as_i64().unwrap_or(0) as i32;
    let sum_levels = q_lv + w_lv + e_lv + r_lv;

    // Só dispara se há ponto pendente e ainda não sugerimos para este nível
    if sum_levels >= level as i32 || (level as i32) <= state.last_suggested_level_up {
        return None;
    }

    state.last_suggested_level_up = level as i32;
    state.last_levelup_emit_time = game_time;

    // ── Usa a ordem do banco ──────────────────────────────────────────────────
    if !state.db_skill_order.is_empty() {
        let idx = (level as usize).saturating_sub(1);
        let skill = state.db_skill_order
            .get(idx)
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| "R".to_string());

        let title = if skill == "R" {
            "\u{2B06}\u{FE0F} Upe o R \u{1F31F}".to_string()
        } else {
            format!("\u{2B06}\u{FE0F} Upe o {}", skill)
        };

        // Resumo da ordem de max: skill mais frequente nos 9 primeiros níveis
        let first_9 = &state.db_skill_order[..state.db_skill_order.len().min(9)];
        let q = first_9.iter().filter(|s| s.to_uppercase() == "Q").count();
        let w = first_9.iter().filter(|s| s.to_uppercase() == "W").count();
        let e = first_9.iter().filter(|s| s.to_uppercase() == "E").count();
        let mut prio = vec![("Q", q), ("W", w), ("E", e)];
        prio.sort_by(|a, b| b.1.cmp(&a.1));
        let back = format!("Max: {} > {} > {}", prio[0].0, prio[1].0, prio[2].0);

        return Some((title, back));
    }

    // ── Sem dados do banco — aviso genérico ──────────────────────────────────
    Some((
        "\u{2B06}\u{FE0F} Ponto de Skill".to_string(),
        "Você tem um ponto disponível — consulte a build.".to_string(),
    ))
}
