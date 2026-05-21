use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tauri::State;
use sqlx::{Pool, Sqlite};
use crate::db::match_service;

pub struct RiotApi {
    pub client: Client,
}

impl RiotApi {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    pub async fn get(&self, api_key: &str, routing: &str, endpoint: &str) -> Result<Value, reqwest::Error> {
        let url = format!("https://{}.api.riotgames.com{}", routing, endpoint);
        let resp = self.client.get(url)
            .header("X-Riot-Token", api_key)
            .send()
            .await?;
        resp.json().await
    }
}

fn get_api_key() -> Result<String, String> {
    std::env::var("RIOT_API_KEY").map_err(|_| "RIOT_API_KEY not found in .env file".to_string())
}

#[tauri::command]
pub async fn get_account_by_riot_id(
    region: String, 
    game_name: String, 
    tag_line: String
) -> Result<Value, String> {
    let api_key = get_api_key()?;
    let api = RiotApi::new();
    let endpoint = format!("/riot/account/v1/accounts/by-riot-id/{}/{}", game_name, tag_line);
    api.get(&api_key, &region, &endpoint).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_summoner_by_puuid(
    platform: String, 
    puuid: String
) -> Result<Value, String> {
    let api_key = get_api_key()?;
    let api = RiotApi::new();
    let endpoint = format!("/lol/summoner/v4/summoners/by-puuid/{}", puuid);
    api.get(&api_key, &platform, &endpoint).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_match_history(
    region: String, 
    puuid: String, 
    count: i32
) -> Result<Value, String> {
    let api_key = get_api_key()?;
    let api = RiotApi::new();
    let endpoint = format!("/lol/match/v5/matches/by-puuid/{}/ids?start=0&count={}", puuid, count);
    api.get(&api_key, &region, &endpoint).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_match_details(
    db: State<'_, Pool<Sqlite>>,
    region: String, 
    match_id: String
) -> Result<Value, String> {
    // 1. Check DB first
    if let Ok(Some(cached_match)) = match_service::get_match(db.inner(), &match_id).await {
        return Ok(cached_match);
    }

    // 2. Fetch from Riot API
    let api_key = get_api_key()?;
    let api = RiotApi::new();
    let endpoint = format!("/lol/match/v5/matches/{}", match_id);
    let data = api.get(&api_key, &region, &endpoint).await.map_err(|e| e.to_string())?;

    // 3. Persist to DB for analysis
    let _ = match_service::save_match(db.inner(), &match_id, &data).await;

    Ok(data)
}
