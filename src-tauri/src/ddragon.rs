use reqwest::Client;
use serde_json::{Value};
use std::time::Duration;
use tauri::State;
use crate::db::sync_service;

pub struct DDragonClient {
    pub client: Client,
}

impl DDragonClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    pub async fn get(&self, endpoint: &str) -> Result<Value, reqwest::Error> {
        let url = format!("{}{}", crate::config::DDRAGON_BASE_URL, endpoint);
        let resp = self.client.get(url).send().await?;
        resp.json().await
    }
}

pub async fn get_latest_version_internal(db: &crate::db::DbPool) -> Result<String, String> {
    let client = DDragonClient::new();
    let data = match client.get("/api/versions.json").await {
        Ok(d) => {
            // Cache it in the database
            let _ = sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('versions', ?)")
                .bind(d.to_string())
                .execute(db)
                .await;
            d
        }
        Err(e) => {
            eprintln!("[DDragon] Erro de rede ao buscar versões online: {}. Buscando no cache local...", e);
            let row: Option<(String,)> = sqlx::query_as("SELECT value FROM sync_metadata WHERE key = 'versions'")
                .fetch_optional(db)
                .await
                .unwrap_or(None);
            
            if let Some((cached_val,)) = row {
                serde_json::from_str(&cached_val).unwrap_or_else(|_| serde_json::json!(["14.10.1"]))
            } else {
                serde_json::json!(["14.10.1"])
            }
        }
    };

    if let Some(v) = data.as_array().and_then(|a| a.first()).and_then(|v| v.as_str()) {
        Ok(v.to_string())
    } else {
        Ok("14.10.1".to_string())
    }
}

pub async fn get_ddragon_champion_details_internal(
    db: &crate::db::DbPool,
    version: &str,
    lang: &str,
    champion_id: &str
) -> Result<Value, String> {
    let client = DDragonClient::new();
    let endpoint = format!("/cdn/{}/data/{}/champion/{}.json", version, lang, champion_id);
    let data = client.get(&endpoint).await.map_err(|e| e.to_string())?;

    // Sync builds to database
    sync_service::sync_recommended_builds(db, champion_id, &data)
        .await
        .map_err(|e| e.to_string())?;

    Ok(data)
}

#[tauri::command]
pub async fn get_ddragon_versions(state: State<'_, crate::db::DbState>) -> Result<Value, String> {
    let version = get_latest_version_internal(&state.0).await?;
    Ok(serde_json::json!([version]))
}

#[tauri::command]
pub async fn get_ddragon_champions(
    state: State<'_, crate::db::DbState>,
    version: String, 
    lang: String
) -> Result<Value, String> {
    let db = &state.0;
    let client = DDragonClient::new();
    let endpoint = format!("/cdn/{}/data/{}/champion.json", version, lang);
    let data = client.get(&endpoint).await.map_err(|e| e.to_string())?;

    // Sync to structured tables
    sync_service::sync_champions(db, &data).await.map_err(|e| e.to_string())?;

    Ok(data)
}

#[allow(dead_code)]
#[tauri::command]
pub async fn get_ddragon_items(
    state: State<'_, crate::db::DbState>,
    version: String, 
    lang: String
) -> Result<Value, String> {
    let db = &state.0;
    let client = DDragonClient::new();
    let endpoint = format!("/cdn/{}/data/{}/item.json", version, lang);
    let data = client.get(&endpoint).await.map_err(|e| e.to_string())?;

    // Sync to structured tables
    sync_service::sync_items(db, &data).await.map_err(|e| e.to_string())?;

    Ok(data)
}

#[allow(dead_code)]
#[tauri::command]
pub async fn get_ddragon_spells(
    _state: State<'_, crate::db::DbState>,
    version: String, 
    lang: String
) -> Result<Value, String> {
    let client = DDragonClient::new();
    let endpoint = format!("/cdn/{}/data/{}/summoner.json", version, lang);
    let data = client.get(&endpoint).await.map_err(|e| e.to_string())?;
    Ok(data)
}

#[allow(dead_code)]
#[tauri::command]
pub async fn get_ddragon_champion_details(
    state: State<'_, crate::db::DbState>,
    version: String, 
    lang: String, 
    champion_id: String
) -> Result<Value, String> {
    get_ddragon_champion_details_internal(&state.0, &version, &lang, &champion_id).await
}

#[allow(dead_code)]
#[tauri::command]
pub async fn hydrate_builds(
    state: State<'_, crate::db::DbState>,
    version: String,
    lang: String
) -> Result<(), String> {
    sync_service::hydrate_all_builds(&state.0, &version, &lang).await
}

