use sqlx::{Pool, Sqlite};
use serde_json::Value;
use crate::db::{models};
use tauri::{AppHandle, Emitter, State};
use futures_util::StreamExt;

pub async fn sync_all_builds_from_ugg(pool: &Pool<Sqlite>, version: &str, app: Option<&AppHandle>) -> Result<(), String> {
    let patch_parts: Vec<&str> = version.split('.').collect();
    if patch_parts.len() < 2 {
        return Err("Invalid version format".to_string());
    }
    let ugg_patch = format!("{}_{}", patch_parts[0], patch_parts[1]);

    let sync_key = format!("build_sync_done_{}", ugg_patch);
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM sync_metadata WHERE key = ?")
        .bind(&sync_key)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    if row.is_some() {
        println!("Sincronização de builds já concluída para o patch {}.", ugg_patch);
        return Ok(());
    }

    let champions: Vec<models::Champion> = sqlx::query_as("SELECT * FROM champions")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let total = champions.len();
    println!("Iniciando sincronização de builds para {} campeões (Patch {})...", total, ugg_patch);
    let client = reqwest::Client::new();

    // Use a stream with concurrency limit to speed up hydration
    let results = futures_util::stream::iter(champions.into_iter().enumerate())
        .map(|(i, champ)| {
            let client = client.clone();
            let pool = pool.clone();
            let app = app.cloned();
            let ugg_patch = ugg_patch.clone();
            
            async move {
                let url = format!("https://stats2.u.gg/lol/1.5/overview/{}/ranked_solo_5x5/{}/1.5.0.json", ugg_patch, champ.key);
                let progress = ((i + 1) as f32 / total as f32 * 100.0) as i32;
                
                if let Some(a) = app {
                    let _ = a.emit("sync-progress", serde_json::json!({
                        "progress": progress,
                        "message": format!("Sincronizando: {}", champ.name),
                        "done": false
                    }));
                }

                match client.get(&url)
                    .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
                    .header("Accept", "application/json, text/plain, */*")
                    .header("Accept-Language", "en-US,en;q=0.9,pt-BR;q=0.8,pt;q=0.7")
                    .header("Referer", "https://u.gg/")
                    .header("Origin", "https://u.gg")
                    .header("sec-ch-ua", "\"Chromium\";v=\"124\", \"Google Chrome\";v=\"124\", \"Not-A.Brand\";v=\"99\"")
                    .header("sec-ch-ua-mobile", "?0")
                    .header("sec-ch-ua-platform", "\"Windows\"")
                    .header("sec-fetch-dest", "empty")
                    .header("sec-fetch-mode", "cors")
                    .header("sec-fetch-site", "same-site")
                    .send().await {
                    Ok(resp) => {
                        if let Ok(json) = resp.json::<Value>().await {
                            let _ = process_build_json(&pool, &champ.id, &json).await;
                        }
                    }
                    Err(e) => eprintln!("Erro em {}: {}", champ.name, e),
                }
                Ok::<(), String>(())
            }
        })
        .buffer_unordered(5) // Concurrecy limit (5 at a time)
        .collect::<Vec<_>>()
        .await;

    if let Some(a) = app {
        let _ = a.emit("sync-progress", serde_json::json!({
            "progress": 100,
            "message": "Sincronização concluída!",
            "done": true
        }));
    }

    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES (?, 'true')")
        .bind(&sync_key)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

async fn process_build_json(pool: &Pool<Sqlite>, champ_id: &str, json: &Value) -> Result<(), String> {
    // Region 1, Rank 10
    if let Some(regions) = json.as_object() {
        if let Some(ranks) = regions.get("1").and_then(|v| v.as_object()) {
            if let Some(roles) = ranks.get("10").and_then(|v| v.as_object()) {
                for (role_id, role_data) in roles {
                    if let Some(data_array) = role_data.as_array() {
                        if data_array.len() >= 3 {
                            let build_data = &data_array[2];
                            // Aggregate items for a full build
                            let mut full_items: Vec<i64> = Vec::new();
                            
                            // 1. Items
                            if let Some(starting) = build_data.get(2).and_then(|v| v.as_array()) {
                                for item in starting { if let Some(id) = item.as_i64() { full_items.push(id); } }
                            }
                            if let Some(core) = build_data.get(3).and_then(|v| v.as_array()) {
                                for item in core { if let Some(id) = item.as_i64() { full_items.push(id); } }
                            }

                            // 2. Runes & Shards (New!)
                            let mut primary_tree: i64 = 8000;
                            let mut secondary_tree: i64 = 8100;
                            let mut runes: Vec<i64> = Vec::new();
                            let mut shards: Vec<i64> = Vec::new();

                            if let Some(meta_data) = data_array.get(0).and_then(|v| v.as_array()) {
                                if meta_data.len() >= 5 {
                                    primary_tree = meta_data[2].as_i64().unwrap_or(8000);
                                    secondary_tree = meta_data[3].as_i64().unwrap_or(8100);
                                    if let Some(runes_arr) = meta_data[4].as_array() {
                                        runes = runes_arr.iter().filter_map(|v| v.as_i64()).collect();
                                    }
                                }
                            }
                            if let Some(shard_data) = data_array.get(8).and_then(|v| v.as_array()) {
                                if shard_data.len() >= 3 {
                                    if let Some(shard_arr) = shard_data[2].as_array() {
                                        shards = shard_arr.iter().filter_map(|v| v.as_i64()).collect();
                                    }
                                }
                            }

                            // Validate and fix rune page to guarantee 4+2+3 structure
                            let champ_tags: String = sqlx::query_scalar(
                                "SELECT COALESCE(tags, '[]') FROM champions WHERE id = ?"
                            )
                            .bind(champ_id)
                            .fetch_optional(pool)
                            .await
                            .unwrap_or(None)
                            .unwrap_or_default();

                            let (fp, fs, fixed_runes, fixed_shards) = crate::db::rune_sync_service::fix_rune_page(
                                primary_tree, secondary_tree, &runes, &shards, &champ_tags
                            );

                            let runes_json = crate::db::rune_sync_service::build_canonical_runes_json(
                                fp, fs, &fixed_runes, &fixed_shards
                            );

                            let items_json = serde_json::to_string(&full_items).unwrap_or("[]".to_string());

                            sqlx::query(
                                "INSERT OR REPLACE INTO recommended_builds (champion_id, role, is_core, items_json, runes_json) VALUES (?, ?, 1, ?, ?)"
                            )
                            .bind(champ_id)
                            .bind(role_id)
                            .bind(items_json)
                            .bind(runes_json)
                            .execute(pool)
                            .await
                            .map_err(|e| e.to_string())?;

                            if let Some(situational_obj) = data_array.get(3).and_then(|v| v.as_object()) {
                                for (slot, items) in situational_obj {
                                    if let Some(items_list) = items.as_array() {
                                        for item_entry in items_list {
                                            if let Some(entry) = item_entry.as_array() {
                                                if entry.len() >= 3 {
                                                    let item_id = entry[0].as_i64().unwrap_or(0);
                                                    let wr = entry[1].as_f64().unwrap_or(0.0) / entry[2].as_f64().unwrap_or(1.0);
                                                    sqlx::query("INSERT OR REPLACE INTO situational_items (champion_id, opponent_type, slot_type, item_id, win_rate) VALUES (?, 'GENERAL', ?, ?, ?)")
                                                        .bind(champ_id).bind(slot).bind(item_id).bind(wr).execute(pool).await.map_err(|e| e.to_string())?;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_champion_build_command(
    state: State<'_, crate::db::DbState>,
    champ_id: String,
    role: Option<String>,
) -> Result<Vec<Value>, String> {
    let pool = &state.0;

    // Resolve DDragon ID: "Miss Fortune" → "MissFortune"
    let resolved = crate::db::coach_service::resolve_champion_db_id(pool, &champ_id).await;
    let key = if resolved.is_empty() { champ_id.as_str() } else { resolved.as_str() };

    let mapped_role = role.as_ref().map(|r| match r.to_lowercase().as_str() {
        "top" => "TOP",
        "jungle" => "JUNGLE",
        "middle" | "mid" => "MID",
        "bottom" | "adc" => "ADC",
        "utility" | "support" => "SUPPORT",
        _ => r.as_str(),
    }.to_string());

    let rows = if let Some(ref r) = mapped_role {
        sqlx::query_as::<_, (Option<String>,)>(
            "SELECT DISTINCT items_json FROM recommended_builds WHERE champion_id = ? AND items_json IS NOT NULL ORDER BY (CASE WHEN role = ? THEN 0 ELSE 1 END), id DESC"
        ).bind(key).bind(r).fetch_all(pool).await
    } else {
        sqlx::query_as::<_, (Option<String>,)>(
            "SELECT DISTINCT items_json FROM recommended_builds WHERE champion_id = ? AND items_json IS NOT NULL ORDER BY id DESC"
        ).bind(key).fetch_all(pool).await
    };

    rows.map(|rows| {
        rows.into_iter()
            .filter_map(|(s,)| s.and_then(|j| serde_json::from_str(&j).ok()))
            .collect()
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_situational_items_command(
    state: State<'_, crate::db::DbState>,
    champ_id: String,
) -> Result<Vec<Value>, String> {
    let pool = &state.0;
    let resolved = crate::db::coach_service::resolve_champion_db_id(pool, &champ_id).await;
    let key = if resolved.is_empty() { champ_id.as_str() } else { resolved.as_str() };
    sqlx::query("SELECT item_id, win_rate, slot_type FROM situational_items WHERE champion_id = ? ORDER BY win_rate DESC")
        .bind(key)
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| {
                    use sqlx::Row;
                    serde_json::json!({
                        "item_id": row.get::<i32, _>("item_id"),
                        "win_rate": row.get::<f64, _>("win_rate"),
                        "slot_type": row.get::<String, _>("slot_type")
                    })
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_champion_runes_command(
    state: State<'_, crate::db::DbState>,
    champ_id: String,
    role: Option<String>,
) -> Result<Value, String> {
    let pool = &state.0;

    // Resolve DDragon ID antes de qualquer query
    let resolved = crate::db::coach_service::resolve_champion_db_id(pool, &champ_id).await;
    let key = if resolved.is_empty() { champ_id.as_str() } else { resolved.as_str() };

    let mapped_role = role.as_ref().map(|r| match r.to_lowercase().as_str() {
        "top" => "TOP",
        "jungle" => "JUNGLE",
        "middle" | "mid" => "MID",
        "bottom" | "adc" => "ADC",
        "utility" | "support" => "SUPPORT",
        _ => r.as_str(),
    }.to_string());

    let mut runes_str: Option<String> = None;

    if let Some(ref r) = mapped_role {
        runes_str = sqlx::query_scalar(
            "SELECT runes_json FROM recommended_builds WHERE champion_id = ? AND role = ? AND elo = 'CHALLENGER' AND runes_json IS NOT NULL LIMIT 1"
        ).bind(key).bind(r).fetch_optional(pool).await.unwrap_or(None);

        if runes_str.is_none() {
            runes_str = sqlx::query_scalar(
                "SELECT runes_json FROM recommended_builds WHERE champion_id = ? AND role = ? AND runes_json IS NOT NULL LIMIT 1"
            ).bind(key).bind(r).fetch_optional(pool).await.unwrap_or(None);
        }

        if runes_str.is_none() {
            runes_str = sqlx::query_scalar(
                "SELECT runes_json FROM blitz_builds WHERE champion_id = ? AND role = ? AND runes_json IS NOT NULL LIMIT 1"
            ).bind(key).bind(r).fetch_optional(pool).await.unwrap_or(None);
        }
    }

    if runes_str.is_none() {
        runes_str = sqlx::query_scalar(
            "SELECT runes_json FROM recommended_builds WHERE champion_id = ? AND elo = 'CHALLENGER' AND runes_json IS NOT NULL LIMIT 1"
        ).bind(key).fetch_optional(pool).await.unwrap_or(None);
    }

    if runes_str.is_none() {
        runes_str = sqlx::query_scalar(
            "SELECT runes_json FROM blitz_builds WHERE champion_id = ? AND runes_json IS NOT NULL LIMIT 1"
        ).bind(key).fetch_optional(pool).await.unwrap_or(None);
    }

    match runes_str {
        Some(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
        None => Err(format!("Nenhuma runa encontrada para '{}'", champ_id))
    }
}

#[tauri::command]
pub async fn sync_builds_command(
    state: State<'_, crate::db::DbState>,
    app: AppHandle,
) -> Result<(), String> {
    let pool = &state.0;
    if let Ok(version) = crate::ddragon::get_latest_version_internal(pool).await {
        sync_all_builds_from_ugg(pool, &version, Some(&app)).await?;
    }
    Ok(())
}
