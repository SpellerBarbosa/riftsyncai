use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub struct LcaConnection {
    pub client: Client,
}

impl LcaConnection {
    pub fn new() -> Self {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_millis(500))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    pub async fn get(&self, endpoint: &str) -> Result<Value, reqwest::Error> {
        let url = format!("https://127.0.0.1:2999/liveclientdata/{}", endpoint);
        match self.client.get(url).send().await {
            Ok(resp) => {
                let val = resp.json::<Value>().await?;
                // println!("LCA Sync OK: {}", endpoint);
                Ok(val)
            },
            Err(e) => Err(e)
        }
    }

    pub async fn is_alive(&self) -> bool {
        // LCA is only alive during a game. 
        // We use allgamedata as a health check.
        self.get("allgamedata").await.is_ok()
    }
}

#[tauri::command]
pub async fn get_lca_status() -> String {
    let lca = LcaConnection::new();
    if lca.is_alive().await {
        "Connected".to_string()
    } else {
        "Disconnected".to_string()
    }
}

#[tauri::command]
pub async fn get_all_game_data() -> Result<Value, String> {
    let lca = LcaConnection::new();
    lca.get("allgamedata").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_active_player() -> Result<Value, String> {
    let lca = LcaConnection::new();
    lca.get("activeplayer").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_player_list() -> Result<Value, String> {
    let lca = LcaConnection::new();
    lca.get("playerlist").await.map_err(|e| e.to_string())
}
