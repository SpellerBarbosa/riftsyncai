use super::state::{CoachState, EnemyTelemetry};

pub fn update_enemy_telemetry(
    state: &mut CoachState,
    lca_data: &serde_json::Value,
    my_team: &str,
    game_time: f64,
) {
    if !state.initialized_telemetry {
        // Initialize enemy telemetries if not yet initialized
        if let Some(players) = lca_data["allPlayers"].as_array() {
            let mut telemetries = Vec::new();
            for p in players {
                let p_team = p["team"].as_str().unwrap_or("ORDER").to_uppercase();
                if p_team != my_team.to_uppercase() {
                    let champ_name = p["championName"].as_str().unwrap_or("").to_string();
                    let raw_role = p["position"].as_str().unwrap_or("MIDDLE").to_uppercase();
                    let role = match raw_role.as_str() {
                        "MIDDLE" => "MID",
                        "BOTTOM" => "ADC",
                        "UTILITY" => "SUPPORT",
                        "JUNGLE" => "JUNGLE",
                        "TOP" => "TOP",
                        _ => "MID",
                    }.to_string();

                    // Fountain location for the enemy team:
                    // If my team is ORDER (Blue), enemy is CHAOS (Red, base at 14500, 14500)
                    // If my team is CHAOS (Red), enemy is ORDER (Blue, base at 500, 500)
                    let is_enemy_chaos = my_team.to_uppercase() == "ORDER";
                    let (x, y) = if is_enemy_chaos {
                        (14500.0, 14500.0)
                    } else {
                        (500.0, 500.0)
                    };

                    telemetries.push(EnemyTelemetry {
                        champion_name: champ_name,
                        role,
                        is_visible: true,
                        last_visible_time: game_time,
                        x,
                        y,
                        fog_duration: 0.0,
                        last_health: p["championStats"]["currentHealth"].as_f64().unwrap_or(1000.0),
                        last_cs: p["scores"]["creepScore"].as_i64().unwrap_or(0),
                        last_level: p["level"].as_i64().unwrap_or(1),
                    });
                }
            }
            if !telemetries.is_empty() {
                state.enemy_telemetries = telemetries;
                state.initialized_telemetry = true;
            }
        }
    } else {
        // Update existing telemetries based on real-time LCA statistical changes
        let is_enemy_chaos = my_team.to_uppercase() == "ORDER";
        let enemy_fountain = if is_enemy_chaos { (14500.0, 14500.0) } else { (500.0, 500.0) };

        if let Some(players) = lca_data["allPlayers"].as_array() {
            for p in players {
                let p_team = p["team"].as_str().unwrap_or("ORDER").to_uppercase();
                if p_team != my_team.to_uppercase() {
                    let champ_name = p["championName"].as_str().unwrap_or("");
                    if let Some(tel) = state.enemy_telemetries.iter_mut().find(|t| t.champion_name == champ_name) {
                        let is_dead = p["isDead"].as_bool().unwrap_or(false);
                        let curr_health = p["championStats"]["currentHealth"].as_f64().unwrap_or(0.0);
                        let curr_cs = p["scores"]["creepScore"].as_i64().unwrap_or(0);
                        let curr_level = p["level"].as_i64().unwrap_or(1);

                        if is_dead {
                            tel.is_visible = true;
                            tel.last_visible_time = game_time;
                            tel.x = enemy_fountain.0;
                            tel.y = enemy_fountain.1;
                            tel.fog_duration = 0.0;
                        } else {
                            // Check if stats changed, which indicates visibility on map
                            let health_changed = (curr_health - tel.last_health).abs() > 0.01;
                            let cs_changed = curr_cs != tel.last_cs;
                            let level_changed = curr_level != tel.last_level;

                            if health_changed || cs_changed || level_changed {
                                let previous_fog_duration = tel.fog_duration;

                                // Limiar de névoa por role: junglers se movem rápido;
                                // laners precisam de ausência longa para indicar rotação real.
                                let fog_threshold = match tel.role.as_str() {
                                    "JUNGLE"           => 10.0, // JG sai/entra rápido — 10s já é saída de rota
                                    "TOP"              => 40.0, // Top demora muito para rotacionar
                                    _                  => 30.0, // Mid, ADC, Support
                                };

                                // CS mudou sozinho (sem HP nem level) = farmando na rota = sem ameaça.
                                // HP mudando junto indica combate/dano → pode ser movimento real.
                                let only_farming = cs_changed && !health_changed && !level_changed;

                                let was_hidden = !tel.is_visible
                                    && tel.fog_duration >= fog_threshold
                                    && !only_farming;

                                if was_hidden {
                                    state.recent_enemy_sighting = Some((
                                        tel.champion_name.clone(),
                                        tel.role.clone(),
                                        previous_fog_duration,
                                    ));
                                }

                                tel.is_visible = true;
                                tel.last_visible_time = game_time;
                                tel.fog_duration = 0.0;

                                // Estimate coordinates along their assigned lane or last known location
                                let (lane_x, lane_y) = match tel.role.as_str() {
                                    "TOP" => {
                                        if game_time < 840.0 {
                                            (1500.0, 13500.0) // Top lane
                                        } else {
                                            (7500.0, 7500.0) // Grouped Mid
                                        }
                                    },
                                    "MID" => {
                                        (7500.0, 7500.0) // Mid lane
                                    },
                                    "ADC" | "SUPPORT" => {
                                        if game_time < 840.0 {
                                            (13500.0, 1500.0) // Bot lane
                                        } else {
                                            (7500.0, 7500.0) // Grouped Mid
                                        }
                                    },
                                    "JUNGLE" => {
                                        (6000.0, 6000.0) // River area
                                    },
                                    _ => (7500.0, 7500.0)
                                };
                                tel.x = lane_x;
                                tel.y = lane_y;
                            } else {
                                // If stats are stagnant, they might be in Fog of War (not visible)
                                let elapsed = game_time - tel.last_visible_time;
                                if elapsed > 4.0 {
                                    tel.is_visible = false;
                                    tel.fog_duration = elapsed;
                                }
                            }
                        }

                        // Save current values for comparison
                        tel.last_health = curr_health;
                        tel.last_cs = curr_cs;
                        tel.last_level = curr_level;
                    }
                }
            }
        }
    }
}
