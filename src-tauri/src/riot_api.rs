use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tauri::{State, Manager};
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

/// Busca os dados da partida mais recente do jogador — chamado internamente pelo bridge
/// após o fim de jogo para gerar o relatório pós-partida.
pub async fn get_last_match(handle: &tauri::AppHandle, puuid: &str) -> Result<Value, String> {
    let api_key = get_api_key()?;
    let api = RiotApi::new();

    // Tenta ranked Solo/Duo primeiro, depois Flex, depois qualquer partida
    let queues = [Some(420u32), Some(440), None];
    let mut match_id = String::new();

    for queue_opt in &queues {
        let endpoint = match queue_opt {
            Some(q) => format!("/lol/match/v5/matches/by-puuid/{}/ids?start=0&count=1&queue={}", puuid, q),
            None    => format!("/lol/match/v5/matches/by-puuid/{}/ids?start=0&count=1", puuid),
        };
        let ids: Value = api.get(&api_key, "americas", &endpoint).await
            .map_err(|e| e.to_string())?;
        if let Some(id) = ids[0].as_str() {
            match_id = id.to_string();
            break;
        }
    }

    if match_id.is_empty() {
        return Err("Nenhuma partida encontrada para este jogador.".to_string());
    }

    let match_endpoint = format!("/lol/match/v5/matches/{}", match_id);
    let data = api.get(&api_key, "americas", &match_endpoint).await
        .map_err(|e| e.to_string())?;

    // Salva no banco para histórico
    if let Some(db_state) = handle.try_state::<crate::db::DbState>() {
        let _ = crate::db::match_service::save_match(&db_state.0, &match_id, &data).await;
    }

    Ok(data)
}

/// Comando Tauri chamado manualmente pelo botão 📊 da menu bar.
/// Busca a última partida ranqueada do jogador logado e emite o relatório pós-jogo.
#[tauri::command]
pub async fn trigger_post_game_analysis(app: tauri::AppHandle) -> Result<(), String> {
    // Pega o PUUID do summoner atual via LCU
    let conn = crate::lcu::LcuConnection::new()
        .ok_or_else(|| "League Client não encontrado — abra o LoL primeiro.".to_string())?;
    let summoner = conn.get("/lol-summoner/v1/current-summoner").await
        .map_err(|e| format!("Erro ao obter summoner: {}", e))?;
    let puuid = summoner["puuid"].as_str()
        .ok_or_else(|| "PUUID não disponível".to_string())?
        .to_string();

    let match_data = get_last_match(&app, &puuid).await?;

    // Detecta o campeão e role do jogador na partida
    let participants = match_data["info"]["participants"].as_array()
        .ok_or("Formato de partida inválido")?;

    let player = participants.iter()
        .find(|p| p["puuid"].as_str().unwrap_or("") == puuid)
        .ok_or("Jogador não encontrado na partida")?;

    let champ = player["championName"].as_str().unwrap_or("Unknown").to_string();
    let role  = player["teamPosition"].as_str().unwrap_or("MID").to_string();

    if let Some(db_state) = app.try_state::<crate::db::DbState>() {
        crate::post_game::analyze_and_emit(&app, &db_state.0, &match_data, &champ, &role).await;
    }

    Ok(())
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
