use sqlx::{Pool, Sqlite};
use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};
use regex::Regex;

#[allow(dead_code)]
pub async fn sync_blitz_data(pool: &Pool<Sqlite>, app: Option<&AppHandle>) -> Result<(), String> {
    println!("[MetaSRC] Iniciando sincronização via Web Scraping (Metasrc)...");
    
    let elos = vec![
        "iron", "bronze", "silver", "gold", "platinum", 
        "emerald", "diamond", "master", "grandmaster", "challenger"
    ];
    let roles = vec!["top", "jungle", "mid", "adc", "support"];
    let client = Client::new();

    let total = elos.len() * roles.len();
    let mut current = 0;

    for elo in &elos {
        for role in &roles {
            current += 1;
            if let Some(a) = app {
                let _ = a.emit("sync-progress", serde_json::json!({
                    "progress": (current as f32 / total as f32 * 100.0) as i32,
                    "message": format!("Tier List: {} {}", elo.to_uppercase(), role.to_uppercase()),
                    "done": false
                }));
            }
            
            if let Err(e) = fetch_and_save_metasrc_tier_list(pool, &client, elo, role).await {
                eprintln!("[MetaSRC] Erro em {} {}: {}", elo, role, e);
            }

            // Pequeno delay para não sobrecarregar
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    // Registrar data
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let _ = sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('blitz_last_sync', ?)")
        .bind(now.to_string())
        .execute(pool)
        .await;

    Ok(())
}

#[allow(dead_code)]
async fn fetch_and_save_metasrc_tier_list(pool: &Pool<Sqlite>, client: &Client, elo: &str, role: &str) -> Result<(), String> {
    let url = format!("https://www.metasrc.com/lol/tier-list/{}?ranks={}", role, elo);
    
    let response = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() != 200 {
        return Err(format!("Erro HTTP {}: {}", response.status(), url));
    }

    let html = response.text().await.map_err(|e| e.to_string())?;
    
    // Regex para extrair blocos de Tier e seus campeões
    // O HTML do Metasrc agrupa por <h2>TIER</h2> seguido de ícones de campeões
    let tier_regex = Regex::new(r"(?s)<h2>(.*?)</h2>(.*?)<div class='champion-grid-container'").unwrap();
    let champ_regex = Regex::new(r"href='/lol/build/(.*?)'").unwrap();

    let mut found_any = false;

    for cap in tier_regex.captures_iter(&html) {
        let tier_name = cap[1].trim().to_uppercase();
        let content = &cap[2];
        
        // Mapear "GOD TIER / S+ TIER" para "S+"
        let tier_code = if tier_name.contains("GOD") || tier_name.contains("S+") { "S+" }
                        else if tier_name.contains("S TIER") { "S" }
                        else if tier_name.contains("A TIER") { "A" }
                        else if tier_name.contains("B TIER") { "B" }
                        else if tier_name.contains("C TIER") { "C" }
                        else if tier_name.contains("D TIER") { "D" }
                        else { "A" };

        for champ_cap in champ_regex.captures_iter(content) {
            let champ_key = &champ_cap[1]; // Ex: "aatrox"
            
            // Tentar resolver o nome real do campeão do DB ou apenas capitalizar
            let champ_id = champ_key.to_lowercase();
            
            sqlx::query(
                "INSERT OR REPLACE INTO blitz_tier_list (elo, role, champion_id, tier, win_rate, pick_rate) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(elo.to_uppercase())
            .bind(role.to_uppercase())
            .bind(champ_id)
            .bind(Some(tier_code))
            .bind(Some(0.0)) // Metasrc HTML scraping simples não pega WR/PR fácil
            .bind(Some(0.0))
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
            
            found_any = true;
        }
    }

    if !found_any {
        // Fallback para Regex de tabela se o Grid falhar
        let table_regex = Regex::new(r"data-label='Name'.*?>(.*?)</a>.*?data-label='Tier'.*?>(.*?)</td>").unwrap();
        for cap in table_regex.captures_iter(&html) {
            let champ_name = cap[1].trim().to_lowercase();
            let tier_raw = cap[2].trim().to_uppercase();
            let tier_code = if tier_raw.contains("S+") || tier_raw.contains("GOD") { "S+" }
                            else if tier_raw.contains("S") { "S" }
                            else { "A" };

            sqlx::query(
                "INSERT OR REPLACE INTO blitz_tier_list (elo, role, champion_id, tier, win_rate, pick_rate) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(elo.to_uppercase())
            .bind(role.to_uppercase())
            .bind(champ_name)
            .bind(Some(tier_code))
            .bind(Some(0.0))
            .bind(Some(0.0))
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[allow(dead_code)]
#[tauri::command]
pub async fn sync_blitz_command(
    state: State<'_, crate::db::DbState>,
    app: AppHandle,
) -> Result<(), String> {
    sync_blitz_data(&state.0, Some(&app)).await
}

#[allow(dead_code)]
pub async fn check_maintenance_update(pool: &Pool<Sqlite>) -> bool {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM sync_metadata WHERE key = 'blitz_last_sync'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

    if let Some((val,)) = row {
        if let Ok(last_sync) = val.parse::<u64>() {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            return now - last_sync > 86400; // 24h
        }
    }
    true
}
