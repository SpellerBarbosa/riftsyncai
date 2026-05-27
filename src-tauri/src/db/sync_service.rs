use sqlx::{Pool, Sqlite};
use serde_json::Value;

pub async fn sync_champions(pool: &Pool<Sqlite>, data: &Value) -> Result<(), sqlx::Error> {
    if let Some(champions) = data["data"].as_object() {
        for (_id, champ_data) in champions {
            let id = champ_data["id"].as_str().unwrap_or_default();
            let key_str = champ_data["key"].as_str().unwrap_or("0");
            let key: i32 = key_str.parse().unwrap_or(0);
            let name = champ_data["name"].as_str().unwrap_or_default();
            let title = champ_data["title"].as_str().unwrap_or_default();
            let tags = champ_data["tags"].to_string(); // JSON Array
            
            sqlx::query(
                "INSERT OR REPLACE INTO champions (id, key, name, title, tags) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(key)
            .bind(name)
            .bind(title)
            .bind(tags)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

pub async fn sync_items(pool: &Pool<Sqlite>, data: &Value) -> Result<(), sqlx::Error> {
    if let Some(items) = data["data"].as_object() {
        for (id, item_data) in items {
            let name = item_data["name"].as_str().unwrap_or_default();
            let description = item_data["description"].as_str().unwrap_or_default();
            let gold = item_data["gold"]["total"].as_i64().unwrap_or(0) as i32;
            let gold_base = item_data["gold"]["base"].as_i64().unwrap_or(0) as i32;
            let components_json = if let Some(from) = item_data.get("from").and_then(|f| f.as_array()) {
                serde_json::to_string(from).unwrap_or_else(|_| "[]".to_string())
            } else {
                "[]".to_string()
            };
            let plaintext = item_data["plaintext"].as_str().unwrap_or_default();

            sqlx::query(
                "INSERT OR REPLACE INTO items (id, name, description, gold_total, gold_base, components_json, plaintext) VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(name)
            .bind(description)
            .bind(gold)
            .bind(gold_base)
            .bind(components_json)
            .bind(plaintext)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}
fn clean_html_tags(s: &str) -> String {
    let mut clean = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            clean.push(c);
        }
    }
    clean = clean.replace("\n", " ").replace("  ", " ");
    clean.trim().to_string()
}

pub async fn sync_recommended_builds(pool: &Pool<Sqlite>, champ_id: &str, data: &Value) -> Result<(), sqlx::Error> {
    println!("--- INICIANDO SYNC DE BUILDS PARA {} ---", champ_id);
    
    if let Some(champ_data) = data["data"][champ_id].as_object() {
        // Ensure champion exists in champions table
        let key_str = champ_data["key"].as_str().unwrap_or("0");
        let key: i32 = key_str.parse().unwrap_or(0);
        let name = champ_data["name"].as_str().unwrap_or(champ_id);
        
        sqlx::query("INSERT OR REPLACE INTO champions (id, key, name) VALUES (?, ?, ?)")
            .bind(champ_id)
            .bind(key)
            .bind(name)
            .execute(pool)
            .await?;

        // Extract spells and passive to construct the spells_description summary
        let mut spells_description = String::new();
        if let Some(spells) = champ_data.get("spells").and_then(|s| s.as_array()) {
            let keys = vec!["Q", "W", "E", "R"];
            for (idx, spell) in spells.iter().enumerate() {
                if idx < keys.len() {
                    let spell_key = keys[idx];
                    let s_name = spell["name"].as_str().unwrap_or_default();
                    let s_desc = spell["description"].as_str().unwrap_or_default();
                    let clean_desc = clean_html_tags(s_desc);
                    spells_description.push_str(&format!("{}: {} ({}) | ", spell_key, s_name, clean_desc));
                }
            }
        }

        if let Some(passive) = champ_data.get("passive") {
            let p_name = passive["name"].as_str().unwrap_or_default();
            let p_desc = passive["description"].as_str().unwrap_or_default();
            let clean_desc = clean_html_tags(p_desc);
            spells_description.push_str(&format!("Passiva: {} ({})", p_name, clean_desc));
        }

        let title = champ_data["title"].as_str();
        let tags_str = champ_data.get("tags").map(|t| t.to_string());

        // Update the champion record with our newly mined detailed fields!
        sqlx::query(
            "UPDATE champions SET title = COALESCE(?, title), tags = COALESCE(?, tags), spells_description = ? WHERE id = ?"
        )
        .bind(title)
        .bind(tags_str)
        .bind(&spells_description)
        .bind(champ_id)
        .execute(pool)
        .await?;
        // 1. HARDCODED META OVERRIDES (Patch 26.10)
        let meta_overrides = vec![
            ("Aatrox", vec!["6630", "3071", "3047", "6333", "3053", "3156"]),
            ("Briar", vec!["3142", "3147", "3047", "3036", "3814", "3156"]), // Youmuu, Collector, Steelcaps, LDR, Edge, Maw
            ("LeeSin", vec!["6610", "3071", "3111", "3053", "6333", "3156"]),
            ("Yasuo", vec!["6672", "3031", "3006", "3046", "3053", "3036"]),
            ("Jinx", vec!["6672", "3031", "3006", "3046", "3036", "3094"]),
        ];

        for (id, items) in meta_overrides {
            if champ_id == id {
                let items_json = serde_json::to_string(&items).unwrap_or_default();
                sqlx::query("INSERT OR REPLACE INTO recommended_builds (champion_id, role, is_core, items_json) VALUES (?, ?, ?, ?)")
                    .bind(champ_id).bind("META").bind(true).bind(items_json).execute(pool).await?;
                println!("Build META aplicada para {}", champ_id);
                return Ok(());
            }
        }

        // 2. CLASS-BASED FALLBACK (Based on Tags)
        let tags = champ_data.get("tags").and_then(|t| t.as_array());
        let mut fallback_items = vec![];
        let mut role = "UNKNOWN";

        if let Some(t) = tags {
            if t.iter().any(|v| v.as_str() == Some("Tank")) {
                fallback_items = vec!["3068", "3075", "3111", "3065", "3083", "3075"]; 
                role = "TANK";
            } else if t.iter().any(|v| v.as_str() == Some("Mage")) {
                fallback_items = vec!["3006", "3157", "3089", "3165", "3135", "3020"]; 
                role = "MAGE";
            } else if t.iter().any(|v| v.as_str() == Some("Assassin")) {
                fallback_items = vec!["3142", "3147", "3117", "3179", "3156", "3814"]; 
                role = "ASSASSIN";
            } else if t.iter().any(|v| v.as_str() == Some("Marksman")) {
                fallback_items = vec!["6672", "3031", "3006", "3046", "3036", "3094"]; 
                role = "ADC";
            } else if t.iter().any(|v| v.as_str() == Some("Fighter")) {
                fallback_items = vec!["3078", "3053", "3047", "3071", "6333", "3156"]; 
                role = "BRUISER";
            } else if t.iter().any(|v| v.as_str() == Some("Support")) {
                fallback_items = vec!["3854", "3111", "3190", "3050", "3107", "3110"]; 
                role = "SUPP";
            }
        }

        if !fallback_items.is_empty() {
            let items_json = serde_json::to_string(&fallback_items).unwrap_or_default();
            sqlx::query("INSERT OR REPLACE INTO recommended_builds (champion_id, role, is_core, items_json) VALUES (?, ?, ?, ?)")
                .bind(champ_id).bind(role).bind(true).bind(items_json).execute(pool).await?;
            println!("Build de CLASSE ({}) aplicada para {}", role, champ_id);
        } else {
             println!("ERRO: Nenhuma build encontrada para {}", champ_id);
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn hydrate_all_builds(pool: &Pool<Sqlite>, version: &str, lang: &str) -> Result<(), String> {
    // 0. Check if already hydrated for this version
    let sync_key = format!("hydration_done_{}", version);
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM sync_metadata WHERE key = ?")
        .bind(&sync_key)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Check if any champion is missing spells_description
    let missing_spells: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM champions WHERE spells_description IS NULL OR spells_description = ''")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    if row.is_some() && missing_spells == 0 {
        println!("Hidratação já concluída para a versão {}. Pulando...", version);
        return Ok(());
    }

    // 1. Ensure champions are synced first
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM champions")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    if count == 0 {
        println!("Tabela de campeões vazia. Forçando sincronização imediata...");
        let client = crate::ddragon::DDragonClient::new();
        let endpoint = format!("/cdn/{}/data/{}/champion.json", version, lang);
        
        // Try primary language
        let data = match client.get(&endpoint).await {
            Ok(d) => d,
            Err(_) => {
                // Fallback to en_US if pt_BR fails
                let fallback = format!("/cdn/{}/data/en_US/champion.json", version);
                client.get(&fallback).await.map_err(|e| e.to_string())?
            }
        };
        sync_champions(pool, &data).await.map_err(|e| e.to_string())?;
        println!("Sincronização inicial de campeões concluída.");
    }

    let champions: Vec<crate::db::models::Champion> = sqlx::query_as("SELECT * FROM champions")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    

    println!("Verificando integridade de {} campeões...", champions.len());
    let client = crate::ddragon::DDragonClient::new();
    let mut hydrated_count = 0;

    for champ in champions {
        // Optimization: Check if champion has recommended builds AND spells_description
        let spells_desc: Option<String> = sqlx::query_scalar("SELECT spells_description FROM champions WHERE id = ?")
            .bind(&champ.id)
            .fetch_one(pool)
            .await
            .unwrap_or(None);

        let build_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM recommended_builds WHERE champion_id = ?")
            .bind(&champ.id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        if build_count > 0 && spells_desc.filter(|s| !s.is_empty()).is_some() {
            continue;
        }

        hydrated_count += 1;
        println!("Hidratando: {} ({} novos encontrados)", champ.id, hydrated_count);
        let endpoint = format!("/cdn/{}/data/{}/champion/{}.json", version, lang, champ.id);
        if let Ok(data) = client.get(&endpoint).await {
            let _ = sync_recommended_builds(pool, &champ.id, &data).await;
        }
        // Small sleep to avoid rate limiting
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    if hydrated_count == 0 {
        println!("Todos os campeões já estão atualizados.");
    } else {
        println!("Hidratação concluída! {} campeões foram atualizados.", hydrated_count);
    }

    // Mark as done
    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES (?, 'true')")
        .bind(&sync_key)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    println!("Hidratação concluída!");
    Ok(())
}
