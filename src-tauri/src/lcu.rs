use sysinfo::System;
use reqwest::Client; // Async client
use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;
use std::time::Duration;
use std::sync::{Mutex, OnceLock};

static SYSTEM_INSTANCE: OnceLock<Mutex<System>> = OnceLock::new();

pub struct LcuConnection {
    pub port: String,
    pub token: String,
    pub client: Client,
}

impl LcuConnection {
    pub fn new() -> Option<Self> {
        let sys_mutex = SYSTEM_INSTANCE.get_or_init(|| Mutex::new(System::new()));
        let mut sys = sys_mutex.lock().unwrap();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let matching_processes = sys.processes().values().filter(|p| {
            let name = p.name().to_string_lossy().to_lowercase();
            name.contains("leagueclientux.exe") || 
            name.contains("leagueclient.exe") || 
            name.contains("leagueclient")
        });

        for p in matching_processes {
            let mut port = String::new();
            let mut token = String::new();

            // Método 1: Tentar ler dos argumentos de linha de comando (se não estiver elevado)
            for arg in p.cmd() {
                let arg_str = arg.to_string_lossy();
                if arg_str.starts_with("--app-port=") {
                    port = arg_str.replace("--app-port=", "");
                } else if arg_str.starts_with("--remoting-auth-token=") {
                    token = arg_str.replace("--remoting-auth-token=", "");
                }
            }

            // Método 2: Se port/token estiverem vazios (ex: processo elevado), tentar ler o lockfile do diretório do executável
            if port.is_empty() || token.is_empty() {
                if let Some(exe_path) = p.exe() {
                    if let Some(parent) = exe_path.parent() {
                        let lockfile_path = parent.join("lockfile");
                        if let Ok(content) = std::fs::read_to_string(&lockfile_path) {
                            let parts: Vec<&str> = content.split(':').collect();
                            if parts.len() >= 5 {
                                port = parts[2].to_string();
                                token = parts[3].to_string();
                            }
                        }
                    }
                }
            }

            // Método 3: Fallback de caminhos padrão do Lockfile se ainda estiver vazio
            if port.is_empty() || token.is_empty() {
                let default_lockfiles = vec![
                    "C:\\Riot Games\\League of Legends\\lockfile",
                    "D:\\Riot Games\\League of Legends\\lockfile",
                    "E:\\Riot Games\\League of Legends\\lockfile",
                ];
                for path in default_lockfiles {
                    if std::path::Path::new(path).exists() {
                        if let Ok(content) = std::fs::read_to_string(path) {
                            let parts: Vec<&str> = content.split(':').collect();
                            if parts.len() >= 5 {
                                port = parts[2].to_string();
                                token = parts[3].to_string();
                                break;
                            }
                        }
                    }
                }
            }

            if !port.is_empty() && !token.is_empty() {
                let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));
                
                let client = Client::builder()
                    .danger_accept_invalid_certs(true)
                    .timeout(Duration::from_millis(500))
                    .default_headers({
                        let mut headers = reqwest::header::HeaderMap::new();
                        let mut auth_value = reqwest::header::HeaderValue::from_str(&format!("Basic {}", auth)).unwrap();
                        auth_value.set_sensitive(true);
                        headers.insert(reqwest::header::AUTHORIZATION, auth_value);
                        headers
                    })
                    .build()
                    .ok()?;

                return Some(Self { port, token, client });
            }
        }
        
        None
    }

    pub async fn get(&self, endpoint: &str) -> Result<Value, reqwest::Error> {
        let url = format!("https://127.0.0.1:{}{}", self.port, endpoint);
        let resp = self.client.get(url).send().await?;
        resp.json().await
    }

    pub async fn is_alive(&self) -> bool {
        // Simple light check to see if LCU is still responding. 
        self.get("/license").await.is_ok()
    }
}

#[tauri::command]
pub async fn get_lcu_status() -> String {
    match LcuConnection::new() {
        Some(_) => "Connected".to_string(),
        None => "Disconnected".to_string(),
    }
}

#[tauri::command]
pub async fn get_current_summoner() -> Result<Value, String> {
    let conn = LcuConnection::new().ok_or("League Client not found")?;
    conn.get("/lol-summoner/v1/current-summoner")
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_user_rank() -> Result<Value, String> {
    let conn = LcuConnection::new().ok_or("League Client not found")?;
    // Endpoint for ranked stats
    conn.get("/lol-ranked/v1/current-ranked-stats")
        .await
        .map_err(|e| e.to_string())
}
