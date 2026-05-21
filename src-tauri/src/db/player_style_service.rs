use sqlx::{Pool, Sqlite};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerMatchMetrics {
    pub match_id: String,
    pub champion_name: String,
    pub win: bool,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub cs_per_min: f64,
    pub vision_score_per_min: f64,
    pub damage_dealt: i32,
    pub gold_earned: i32,
    pub duration_min: f64,
    pub position: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerAggregateProfile {
    pub total_games: i32,
    pub win_rate: f64,
    pub avg_kda: f64,
    pub avg_cs_per_min: f64,
    pub avg_vision_score_per_min: f64,
    pub avg_deaths: f64,
    pub avg_damage: f64,
    pub style_tag: String,
    pub style_description: String,
    pub coaching_tips: String,
    pub recent_matches: Vec<PlayerMatchMetrics>,
}

pub fn calculate_style(profile: &PlayerAggregateProfile) -> (String, String) {
    let avg_kills_assists_per_min = (profile.avg_kda * profile.avg_deaths) / 30.0; // approximation
    
    if profile.avg_cs_per_min >= 6.8 && avg_kills_assists_per_min < 0.35 {
        (
            "Fazendeiro Passivo".to_string(),
            "Você foca intensamente em farmar e acumular recursos, mas evita se envolver em lutas e skirmishes iniciais. Tente participar de lutas por objetivos.".to_string()
        )
    } else if profile.avg_deaths >= 7.5 && profile.avg_kda < 2.0 {
        (
            "Duelista Inconsequente".to_string(),
            "Você busca lutas constantemente, mas frequentemente se expõe a emboscadas ou inicia sem vantagem numérica. Foque em jogar ao redor de visão.".to_string()
        )
    } else if profile.avg_vision_score_per_min >= 1.1 {
        (
            "Sentinela de Visão".to_string(),
            "Excelente controle de mapa! Você prioriza visão e controle de objetivos. Tente capitalizar essa visão coordenando emboscadas com o time.".to_string()
        )
    } else if profile.avg_vision_score_per_min < 0.35 {
        (
            "Cego de Mapa".to_string(),
            "Seu controle de visão está perigosamente baixo. Você morre facilmente por falta de sentinelas nos arbustos. Compre mais Control Wards!".to_string()
        )
    } else if profile.avg_deaths <= 4.2 && profile.win_rate >= 52.0 {
        (
            "Carregador Consistente".to_string(),
            "Você joga de forma segura, com pouquíssimas mortes e alto impacto tático. Esse é o estilo padrão dos jogadores Challenger!".to_string()
        )
    } else if profile.avg_kda >= 3.0 {
        (
            "Assassino Oportunista".to_string(),
            "Você tem excelente mecânica de abate e KDA inflado, garantindo muitas eliminações isoladas. Busque flanquear nas lutas de equipe.".to_string()
        )
    } else {
        (
            "Estilo Equilibrado".to_string(),
            "Seu jogo é balanceado em todas as frentes (farm, lutas, visão). Para subir de elo, busque se especializar em uma condição de vitória clara.".to_string()
        )
    }
}

pub async fn build_player_profile(
    pool: &Pool<Sqlite>,
    puuid: &str
) -> Result<PlayerAggregateProfile, String> {
    // 1. Fetch all cached matches from database
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT match_id, data FROM matches ORDER BY created_at DESC LIMIT 15")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    if rows.is_empty() {
        return Err("Nenhuma partida encontrada no banco de dados para analisar. Jogue ou sincronize seu histórico!".to_string());
    }

    let mut parsed_matches = Vec::new();
    let mut total_wins = 0;
    let mut total_kills = 0;
    let mut total_deaths = 0;
    let mut total_assists = 0;
    let mut sum_cs_per_min = 0.0;
    let mut sum_vision_score_per_min = 0.0;
    let mut sum_damage = 0.0;

    for (match_id, data_str) in rows {
        if let Ok(match_val) = serde_json::from_str::<Value>(&data_str) {
            // Find player in participants
            if let Some(participants) = match_val.pointer("/info/participants").and_then(|p| p.as_array()) {
                let duration_sec = match_val.pointer("/info/gameDuration")
                    .and_then(|d| d.as_f64())
                    .or_else(|| match_val.pointer("/info/duration").and_then(|d| d.as_f64()))
                    .unwrap_or(1800.0);
                
                // Riot API can return duration in ms or seconds. If > 5000, it's ms
                let duration_min = if duration_sec > 5000.0 {
                    duration_sec / 60000.0
                } else {
                    duration_sec / 60.0
                };

                for p in participants {
                    if p.get("puuid").and_then(|v| v.as_str()) == Some(puuid) {
                        let champ = p.get("championName").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                        let win = p.get("win").and_then(|v| v.as_bool()).unwrap_or(false);
                        let kills = p.get("kills").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let deaths = p.get("deaths").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let assists = p.get("assists").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        
                        let cs = p.get("totalMinionsKilled").and_then(|v| v.as_i64()).unwrap_or(0)
                            + p.get("neutralMinionsKilled").and_then(|v| v.as_i64()).unwrap_or(0);
                        
                        let vision = p.get("visionScore").and_then(|v| v.as_i64()).unwrap_or(0);
                        let damage = p.get("totalDamageDealtToChampions").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let gold = p.get("goldEarned").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let pos = p.get("individualPosition").and_then(|v| v.as_str()).unwrap_or("UNKNOWN").to_string();

                        let cs_per_min = if duration_min > 0.0 { cs as f64 / duration_min } else { 0.0 };
                        let vision_score_per_min = if duration_min > 0.0 { vision as f64 / duration_min } else { 0.0 };

                        if win {
                            total_wins += 1;
                        }

                        total_kills += kills;
                        total_deaths += deaths;
                        total_assists += assists;
                        sum_cs_per_min += cs_per_min;
                        sum_vision_score_per_min += vision_score_per_min;
                        sum_damage += damage as f64;

                        parsed_matches.push(PlayerMatchMetrics {
                            match_id: match_id.clone(),
                            champion_name: champ,
                            win,
                            kills,
                            deaths,
                            assists,
                            cs_per_min,
                            vision_score_per_min,
                            damage_dealt: damage,
                            gold_earned: gold,
                            duration_min,
                            position: pos,
                        });
                        break;
                    }
                }
            }
        }
    }

    let total_games = parsed_matches.len() as i32;
    if total_games == 0 {
        return Err("Nenhuma partida associada ao seu PUUID foi encontrada no banco.".to_string());
    }

    let win_rate = (total_wins as f64 / total_games as f64) * 100.0;
    let avg_kda = if total_deaths > 0 {
        (total_kills + total_assists) as f64 / total_deaths as f64
    } else {
        (total_kills + total_assists) as f64
    };

    let avg_cs_per_min = sum_cs_per_min / total_games as f64;
    let avg_vision_score_per_min = sum_vision_score_per_min / total_games as f64;
    let avg_deaths = total_deaths as f64 / total_games as f64;
    let avg_damage = sum_damage / total_games as f64;

    let mut profile = PlayerAggregateProfile {
        total_games,
        win_rate,
        avg_kda,
        avg_cs_per_min,
        avg_vision_score_per_min,
        avg_deaths,
        avg_damage,
        style_tag: "".to_string(),
        style_description: "".to_string(),
        coaching_tips: "".to_string(),
        recent_matches: parsed_matches,
    };

    let (tag, desc) = calculate_style(&profile);
    profile.style_tag = tag;
    profile.style_description = desc;

    Ok(profile)
}

pub async fn sync_lcu_matches_fallback(
    pool: &Pool<Sqlite>,
    _puuid_filter: &str,
) -> Result<(), String> {
    let conn = match crate::lcu::LcuConnection::new() {
        Some(c) => c,
        None => return Err("Cliente do League of Legends não está aberto.".to_string()),
    };

    println!("[PlayerStyle LCU Fallback] Sincronizando partidas locais do LCU...");

    let res_val = conn.get("/lol-match-history/v1/products/lol/current-summoner/matches")
        .await
        .map_err(|e| format!("Falha ao consultar histórico do LCU: {}", e))?;

    let games_array = match res_val.pointer("/games/games").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Ok(()),
    };

    for game in games_array {
        let game_id = match game.get("gameId").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => continue,
        };

        let match_id = format!("LOCAL_{}", game_id);

        // Check if we already have it in the database
        if let Ok(Some(_)) = crate::db::match_service::get_match(pool, &match_id).await {
            continue;
        }

        let duration_sec = game.get("gameDuration").and_then(|v| v.as_f64()).unwrap_or(1800.0);
        let participants = match game.get("participants").and_then(|v| v.as_array()) {
            Some(p) => p,
            None => continue,
        };
        let identities = match game.get("participantIdentities").and_then(|v| v.as_array()) {
            Some(i) => i,
            None => continue,
        };

        let mut mock_participants = Vec::new();

        for p in participants {
            let p_id = p.get("participantId").and_then(|v| v.as_i64()).unwrap_or(0);
            
            // Find PUUID in identities
            let mut puuid = String::new();
            for ident in identities {
                if ident.get("participantId").and_then(|v| v.as_i64()) == Some(p_id) {
                    if let Some(player) = ident.get("player") {
                        puuid = player.get("puuid").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    }
                    break;
                }
            }

            if puuid.is_empty() {
                continue;
            }

            let champ_id = p.get("championId").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            
            // Resolve champion name from key/champ_id
            let champ_name: String = sqlx::query_scalar("SELECT id FROM champions WHERE key = ?")
                .bind(champ_id)
                .fetch_one(pool)
                .await
                .unwrap_or_else(|_| "Unknown".to_string());

            let stats = match p.get("stats") {
                Some(s) => s,
                None => continue,
            };

            let win = stats.get("win").and_then(|v| v.as_bool()).unwrap_or(false);
            let kills = stats.get("kills").and_then(|v| v.as_i64()).unwrap_or(0);
            let deaths = stats.get("deaths").and_then(|v| v.as_i64()).unwrap_or(0);
            let assists = stats.get("assists").and_then(|v| v.as_i64()).unwrap_or(0);
            let minions = stats.get("totalMinionsKilled").and_then(|v| v.as_i64()).unwrap_or(0);
            let jungle_monsters = stats.get("neutralMinionsKilled").and_then(|v| v.as_i64()).unwrap_or(0);
            let vision = stats.get("visionScore").and_then(|v| v.as_i64()).unwrap_or(0);
            let damage = stats.get("totalDamageDealtToChampions").and_then(|v| v.as_i64()).unwrap_or(0);
            let gold = stats.get("goldEarned").and_then(|v| v.as_i64()).unwrap_or(0);
            let lane = p.pointer("/timeline/lane").and_then(|v| v.as_str()).unwrap_or("UNKNOWN").to_string();

            mock_participants.push(json!({
                "puuid": puuid,
                "championName": champ_name,
                "win": win,
                "kills": kills,
                "deaths": deaths,
                "assists": assists,
                "totalMinionsKilled": minions,
                "neutralMinionsKilled": jungle_monsters,
                "visionScore": vision,
                "totalDamageDealtToChampions": damage,
                "goldEarned": gold,
                "individualPosition": lane
            }));
        }

        let mock_match_data = json!({
            "info": {
                "gameDuration": duration_sec,
                "participants": mock_participants
            }
        });

        let _ = crate::db::match_service::save_match(pool, &match_id, &mock_match_data).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_player_style_analysis(
    db: tauri::State<'_, Pool<Sqlite>>,
    puuid: String,
    region: String,
    count: i32
) -> Result<Value, String> {
    let pool = db.inner();

    // 1. Fetch match list from Riot API (sync up to count matches)
    let mut synced = false;
    if let Ok(var_val) = std::env::var("RIOT_API_KEY") {
        if !var_val.trim().is_empty() {
            println!("[PlayerStyle] Sincronizando últimas {} partidas do PUUID: {} com a Riot API...", count, puuid);
            let client = reqwest::Client::new();
            let list_url = format!(
                "https://{}.api.riotgames.com/lol/match/v5/matches/by-puuid/{}/ids?start=0&count={}",
                region, puuid, count
            );
            
            let list_res = client.get(&list_url)
                .header("X-Riot-Token", &var_val)
                .send()
                .await;

            match list_res {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        if let Ok(match_ids) = resp.json::<Vec<String>>().await {
                            println!("[PlayerStyle] Encontradas {} partidas no Riot API. Buscando detalhes...", match_ids.len());
                            for match_id in match_ids {
                                // Check if we already have it
                                if let Ok(Some(_)) = crate::db::match_service::get_match(pool, &match_id).await {
                                    continue; // skip cached
                                }
                                // Fetch details
                                let detail_url = format!(
                                    "https://{}.api.riotgames.com/lol/match/v5/matches/{}",
                                    region, match_id
                                );
                                let detail_res = client.get(&detail_url)
                                    .header("X-Riot-Token", &var_val)
                                    .send()
                                    .await;
                                
                                if let Ok(d_resp) = detail_res {
                                    if d_resp.status().is_success() {
                                        if let Ok(d_json) = d_resp.json::<Value>().await {
                                            let _ = crate::db::match_service::save_match(pool, &match_id, &d_json).await;
                                        }
                                    } else {
                                        println!("[PlayerStyle] Falha ao baixar detalhes da partida {}: {}", match_id, d_resp.status());
                                    }
                                }
                            }
                            synced = true;
                        } else {
                            println!("[PlayerStyle] Falha ao deserializar resposta da Riot API: JSON inválido.");
                        }
                    } else {
                        println!("[PlayerStyle] Riot API retornou erro HTTP: {}. Verifique se a sua chave API é válida, ativa e se a região está correta.", status);
                    }
                }
                Err(e) => {
                    println!("[PlayerStyle] Erro de rede ao conectar à Riot API: {}", e);
                }
            }
        } else {
            println!("[PlayerStyle] A chave RIOT_API_KEY no arquivo .env está vazia.");
        }
    } else {
        println!("[PlayerStyle] Chave RIOT_API_KEY não foi encontrada no ambiente.");
    }

    if !synced {
        // Fallback: sync from LCU local client
        let _ = sync_lcu_matches_fallback(pool, &puuid).await;
    }

    // 2. Build local profile from DB
    let mut profile = build_player_profile(pool, &puuid).await?;

    profile.coaching_tips = "{\"erros\": [\"Baixa taxa de sentinelas.\", \"Mortes desnecessárias em desvantagem.\"], \"treinos\": [\"2 sentinelas de controle por retorno à base.\", \"Só inicie lutas com o caçador inimigo rastreado.\", \"Farm acima de 7 CS/min por partida.\"]}".to_string();

    Ok(json!(profile))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedSummoner {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
}

#[tauri::command]
pub async fn save_lcu_summoner_command(
    app: tauri::AppHandle,
    puuid: String,
    game_name: String,
    tag_line: String,
) -> Result<(), String> {
    let state = app.try_state::<crate::db::DbState>()
        .ok_or_else(|| "Banco de dados não está pronto.".to_string())?;
    let pool = &state.0;

    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('last_summoner_puuid', ?)")
        .bind(&puuid)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('last_summoner_name', ?)")
        .bind(&game_name)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('last_summoner_tag', ?)")
        .bind(&tag_line)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_saved_summoner_command(
    app: tauri::AppHandle,
) -> Result<Option<SavedSummoner>, String> {
    let state = match app.try_state::<crate::db::DbState>() {
        Some(s) => s,
        None => return Ok(None),
    };
    let pool = &state.0;

    let puuid: Option<String> = sqlx::query_scalar("SELECT value FROM sync_metadata WHERE key = 'last_summoner_puuid'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

    let game_name: Option<String> = sqlx::query_scalar("SELECT value FROM sync_metadata WHERE key = 'last_summoner_name'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

    let tag_line: Option<String> = sqlx::query_scalar("SELECT value FROM sync_metadata WHERE key = 'last_summoner_tag'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

    if let (Some(p), Some(n), Some(t)) = (puuid, game_name, tag_line) {
        Ok(Some(SavedSummoner {
            puuid: p,
            game_name: n,
            tag_line: t,
        }))
    } else {
        Ok(None)
    }
}
