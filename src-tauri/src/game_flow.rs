use serde::{Serialize, Deserialize};
use crate::lcu::LcuConnection;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TeamMember {
    pub champion_id: i64,
    pub assigned_position: String,
    pub cell_id: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GameState {
    Lobby,
    ChampSelect { 
        phase: ChampSelectPhase, 
        champion_id: i64, 
        role: String,
        my_team: Vec<TeamMember>,
        their_team: Vec<TeamMember>,
        pick_index: i32,
        banned_champion_ids: Vec<i64>,
    },
    InGame,
    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChampSelectPhase {
    Banning,
    AguardandoPick,
    Escolhendo,
    Finalizing,
}

pub async fn get_game_state_from_conn(conn: &LcuConnection) -> Result<GameState, String> {
    let gameflow = conn.get("/lol-gameflow/v1/session")
        .await
        .map_err(|e| e.to_string())?;

    let phase = gameflow["phase"].as_str().unwrap_or("None");

    match phase {
        "Lobby" | "None" | "Matchmaking" | "ReadyCheck" => Ok(GameState::Lobby),
        "ChampSelect" => {
            let champ_select = conn.get("/lol-champ-select/v1/session")
                .await
                .map_err(|e| e.to_string())?;
            
            let local_cell_id = champ_select["localPlayerCellId"].as_i64().unwrap_or(-1);
            
            // Extract Teams
            let mut my_team = Vec::new();
            if let Some(team_arr) = champ_select["myTeam"].as_array() {
                for m in team_arr {
                    my_team.push(TeamMember {
                        champion_id: m["championId"].as_i64().unwrap_or(0),
                        assigned_position: m["assignedPosition"].as_str().unwrap_or("UNKNOWN").to_string(),
                        cell_id: m["cellId"].as_i64().unwrap_or(-1),
                    });
                }
            }

            let mut their_team = Vec::new();
            if let Some(team_arr) = champ_select["theirTeam"].as_array() {
                for m in team_arr {
                    their_team.push(TeamMember {
                        champion_id: m["championId"].as_i64().unwrap_or(0),
                        assigned_position: m["assignedPosition"].as_str().unwrap_or("UNKNOWN").to_string(),
                        cell_id: m["cellId"].as_i64().unwrap_or(-1),
                    });
                }
            }

            // Find local player champion and role
            let mut picked_champion_id = 0;
            let mut player_role = "UNKNOWN".to_string();
            for member in &my_team {
                if member.cell_id == local_cell_id {
                    player_role = member.assigned_position.clone();
                    break;
                }
            }

            let actions = champ_select["actions"].as_array();
            let mut has_draft_actions = false;

            if let Some(actions_groups) = actions {
                for group in actions_groups {
                    if let Some(group_array) = group.as_array() {
                        for action in group_array {
                            let action_type = action["type"].as_str().unwrap_or("");
                            if action_type == "pick" || action_type == "ban" {
                                has_draft_actions = true;
                            }
                            if action["actorCellId"].as_i64() == Some(local_cell_id) {
                                if action_type == "pick" {
                                    let cid = action["championId"].as_i64().unwrap_or(0);
                                    let completed = action["completed"].as_bool().unwrap_or(false);
                                    if cid > 0 && completed {
                                        picked_champion_id = cid;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Fallback for ARAM or non-draft modes where actions don't exist
            if !has_draft_actions {
                for member in &my_team {
                    if member.cell_id == local_cell_id {
                        if member.champion_id > 0 {
                            picked_champion_id = member.champion_id;
                        }
                        break;
                    }
                }
            }

            // Calculate Pick Index (Position in team 1-5)
            let mut pick_index = 0;
            if let Some(team_arr) = champ_select["myTeam"].as_array() {
                for (idx, member) in team_arr.iter().enumerate() {
                    if member["cellId"].as_i64() == Some(local_cell_id) {
                        pick_index = (idx as i32) + 1;
                        break;
                    }
                }
            }
            
            // Extract Bans
            let mut banned_champion_ids = Vec::new();
            if let Some(bans_obj) = champ_select["bans"].as_object() {
                if let Some(my_bans) = bans_obj["myTeamBans"].as_array() {
                    for b in my_bans {
                        if let Some(id) = b.as_i64() {
                            if id > 0 { banned_champion_ids.push(id); }
                        }
                    }
                }
                if let Some(their_bans) = bans_obj["theirTeamBans"].as_array() {
                    for b in their_bans {
                        if let Some(id) = b.as_i64() {
                            if id > 0 { banned_champion_ids.push(id); }
                        }
                    }
                }
            }

            if let Some(actions_groups) = actions {
                for group in actions_groups {
                    if let Some(group_array) = group.as_array() {
                        for action in group_array {
                            let is_in_progress = action["isInProgress"].as_bool().unwrap_or(false);
                            if is_in_progress {
                                let actor_cell_id = action["actorCellId"].as_i64().unwrap_or(-2);
                                let is_local = actor_cell_id == local_cell_id;
                                let action_type = action["type"].as_str().unwrap_or("");
                                
                                if action_type == "ban" {
                                    return Ok(GameState::ChampSelect { 
                                        phase: ChampSelectPhase::Banning, 
                                        champion_id: picked_champion_id, 
                                        role: player_role,
                                        my_team,
                                        their_team,
                                        pick_index,
                                        banned_champion_ids
                                    });
                                } else if action_type == "pick" {
                                    if is_local {
                                        return Ok(GameState::ChampSelect { 
                                            phase: ChampSelectPhase::Escolhendo, 
                                            champion_id: picked_champion_id, 
                                            role: player_role,
                                            my_team,
                                            their_team,
                                            pick_index,
                                            banned_champion_ids
                                        });
                                    } else {
                                        return Ok(GameState::ChampSelect { 
                                            phase: ChampSelectPhase::AguardandoPick, 
                                            champion_id: picked_champion_id, 
                                            role: player_role,
                                            my_team,
                                            their_team,
                                            pick_index,
                                            banned_champion_ids
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            Ok(GameState::ChampSelect { 
                phase: ChampSelectPhase::Finalizing, 
                champion_id: picked_champion_id, 
                role: player_role,
                my_team,
                their_team,
                pick_index,
                banned_champion_ids
            })
        },
        "InProgress" | "GameStart" => Ok(GameState::InGame),
        _ => Ok(GameState::Unknown(phase.to_string())),
    }
}

#[allow(dead_code)]
#[tauri::command]
pub async fn get_game_state() -> Result<GameState, String> {
    let conn = LcuConnection::new().ok_or("League Client not found")?;
    get_game_state_from_conn(&conn).await
}
