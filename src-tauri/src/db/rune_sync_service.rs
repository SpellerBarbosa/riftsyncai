use sqlx::{Pool, Sqlite};
use serde_json::Value;

/// Fallbacks de runas por classe de campeão.
/// Garante sempre: 4 runas primárias (1 keystone + 3 sub) + 2 secundárias + 3 shards = 6 runas + 3 shards.
pub fn get_fallback_runes_for_class(tags: &str) -> (i64, i64, Vec<i64>, Vec<i64>) {
    let t = tags.to_lowercase();
    if t.contains("mage") {
        (
            8200, // Sorcery primary
            8300, // Inspiration secondary
            vec![8229, 8226, 8210, 8237, 8306, 8345], // Arcane Comet, Manaflow, Transcendence, Scorch | Hextech, Biscuit
            vec![5008, 5008, 5001],
        )
    } else if t.contains("marksman") {
        (
            8000, // Precision primary
            8300, // Inspiration secondary
            vec![8008, 9111, 9104, 8014, 8306, 8345], // Lethal Tempo, Triumph, Alacrity, Coup | Hextech, Biscuit
            vec![5005, 5008, 5001],
        )
    } else if t.contains("tank") {
        (
            8400, // Resolve primary
            8000, // Precision secondary
            vec![8437, 8446, 8429, 8451, 9111, 9104], // Grasp, Demolish, Conditioning, Overgrowth | Triumph, Alacrity
            vec![5007, 5002, 5001],
        )
    } else if t.contains("assassin") {
        (
            8100, // Domination primary
            8200, // Sorcery secondary
            vec![8112, 8126, 8138, 8135, 8210, 8237], // Electrocute, Cheap Shot, Eyeball, Treasure | Transcendence, Scorch
            vec![5008, 5008, 5001],
        )
    } else if t.contains("support") {
        (
            8200, // Sorcery primary
            8400, // Resolve secondary
            vec![8214, 8226, 8210, 8237, 8463, 8453], // Aery, Manaflow, Transcendence, Scorch | Font of Life, Revitalize
            vec![5008, 5002, 5001],
        )
    } else {
        // Fighter / Bruiser
        (
            8000, // Precision primary
            8400, // Resolve secondary
            vec![8010, 9111, 9104, 8014, 8429, 8451], // Conqueror, Triumph, Alacrity, Coup | Conditioning, Overgrowth
            vec![5008, 5008, 5001],
        )
    }
}

/// Valida se uma página de runas tem a estrutura correta: 6 runas (4 primárias + 2 secundárias) + 3 shards.
pub fn validate_rune_page(runes: &[i64], shards: &[i64]) -> bool {
    runes.len() == 6 && shards.len() == 3
}

/// Defaults de runas por árvore primária — garante que todos os círculos da
/// árvore primária sejam destacados corretamente, mesmo com dados parciais do Vercel.
fn get_tree_specific_defaults(primary_tree: i64, keystone: i64) -> (i64, i64, Vec<i64>, Vec<i64>) {
    match primary_tree {
        8000 => (8000, 8400, vec![keystone, 9111, 9104, 8014, 8429, 8451], vec![5008, 5008, 5001]),
        8100 => (8100, 8200, vec![keystone, 8139, 8138, 8135, 8210, 8237], vec![5008, 5008, 5001]),
        8200 => (8200, 8300, vec![keystone, 8226, 8210, 8237, 8306, 8345], vec![5008, 5008, 5001]),
        8300 => (8300, 8200, vec![keystone, 8306, 8345, 8352, 8226, 8210], vec![5008, 5008, 5001]),
        8400 => (8400, 8000, vec![keystone, 8446, 8429, 8451, 9111, 9104], vec![5007, 5002, 5001]),
        _    => (8000, 8400, vec![keystone, 9111, 9104, 8014, 8429, 8451], vec![5008, 5008, 5001]),
    }
}

/// Corrige uma página de runas malformada.
/// Estratégia em cascata:
///  1. Página completa (6+3) → retorna sem alterar.
///  2. Dados parciais com keystone + árvore válida → preenche com defaults da
///     MESMA árvore (não por classe) — evita runas de Precisão em página de Feitiçaria.
///  3. Dados inválidos → fallback total por classe do campeão.
pub fn fix_rune_page(
    primary_tree: i64,
    secondary_tree: i64,
    runes: &[i64],
    shards: &[i64],
    champ_tags: &str,
) -> (i64, i64, Vec<i64>, Vec<i64>) {
    if validate_rune_page(runes, shards) {
        return (primary_tree, secondary_tree, runes.to_vec(), shards.to_vec());
    }

    println!("[RuneSync] Parcial: {} runas, {} shards (primary={}).", runes.len(), shards.len(), primary_tree);

    let valid_primary = [8000i64, 8100, 8200, 8300, 8400].contains(&primary_tree);
    let keystone = runes.first().copied().unwrap_or(0);

    if valid_primary && keystone > 0 {
        let (def_primary, def_secondary, mut def_runes, def_shards) =
            get_tree_specific_defaults(primary_tree, keystone);

        // Preserva slots 1-5 do banco se forem válidos
        for i in 1..6 {
            if i < runes.len() && runes[i] > 0 {
                def_runes[i] = runes[i];
            }
        }
        let use_secondary = if secondary_tree > 0 { secondary_tree } else { def_secondary };
        let fixed_shards: Vec<i64> = (0..3)
            .map(|i| if i < shards.len() && shards[i] > 0 { shards[i] } else { def_shards.get(i).copied().unwrap_or(5008) })
            .collect();

        return (def_primary, use_secondary, def_runes, fixed_shards);
    }

    // Fallback por classe do campeão (dados sem nenhum keystone identificável)
    let (fb_primary, fb_secondary, fb_runes, fb_shards) = get_fallback_runes_for_class(champ_tags);
    let fixed_primary   = if primary_tree   > 0 { primary_tree   } else { fb_primary };
    let fixed_secondary = if secondary_tree > 0 { secondary_tree } else { fb_secondary };
    let fixed_runes: Vec<i64> = (0..6)
        .map(|i| if i < runes.len() && runes[i] > 0 { runes[i] } else { fb_runes.get(i).copied().unwrap_or(0) })
        .collect();
    let fixed_shards: Vec<i64> = (0..3)
        .map(|i| if i < shards.len() && shards[i] > 0 { shards[i] } else { fb_shards.get(i).copied().unwrap_or(5008) })
        .collect();

    (fixed_primary, fixed_secondary, fixed_runes, fixed_shards)
}

/// Serializa uma página de runas no formato canônico JSON.
pub fn build_canonical_runes_json(
    primary_tree: i64,
    secondary_tree: i64,
    runes: &[i64],
    shards: &[i64],
) -> String {
    serde_json::json!({
        "primary_tree": primary_tree,
        "secondary_tree": secondary_tree,
        "runes": runes,
        "shards": shards
    })
    .to_string()
}

/// Sincroniza a tabela `runes` com os dados brutos de runas do DDragon (runesReforged.json).
pub async fn sync_runes_from_ddragon(
    pool: &Pool<Sqlite>,
    version: &str,
    lang: &str,
) -> Result<(), String> {
    let sync_key = format!("rune_data_synced_{}", version);
    let already_done: Option<(String,)> = sqlx::query_as("SELECT value FROM sync_metadata WHERE key = ?")
        .bind(&sync_key)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    if already_done.is_some() {
        println!("[RuneSync] Runas já sincronizadas para versão {}. Pulando.", version);
        return Ok(());
    }

    let client = crate::ddragon::DDragonClient::new();
    let endpoint = format!("/cdn/{}/data/{}/runesReforged.json", version, lang);

    let data = match client.get(&endpoint).await {
        Ok(d) => d,
        Err(_) => {
            let fallback_ep = format!("/cdn/{}/data/en_US/runesReforged.json", version);
            client.get(&fallback_ep).await.map_err(|e| e.to_string())?
        }
    };

    let trees = data.as_array().ok_or("runesReforged.json is not an array")?;
    let mut count = 0;

    for tree in trees {
        let tree_id = tree["id"].as_i64().unwrap_or(0);
        let tree_name = tree["name"].as_str().unwrap_or("");
        let tree_icon = tree["icon"].as_str().unwrap_or("");

        // Insert tree root
        sqlx::query(
            "INSERT OR REPLACE INTO runes (id, name, icon, short_desc) VALUES (?, ?, ?, ?)"
        )
        .bind(tree_id)
        .bind(tree_name)
        .bind(tree_icon)
        .bind("Tree")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

        // Iterate slots (rows)
        if let Some(slots) = tree["slots"].as_array() {
            for slot in slots {
                if let Some(runes) = slot["runes"].as_array() {
                    for rune in runes {
                        let id = rune["id"].as_i64().unwrap_or(0);
                        let name = rune["name"].as_str().unwrap_or("");
                        let icon = rune["icon"].as_str().unwrap_or("");
                        let short_desc = rune["shortDesc"].as_str().unwrap_or("");

                        sqlx::query(
                            "INSERT OR REPLACE INTO runes (id, name, icon, short_desc) VALUES (?, ?, ?, ?)"
                        )
                        .bind(id)
                        .bind(name)
                        .bind(icon)
                        .bind(short_desc)
                        .execute(pool)
                        .await
                        .map_err(|e| e.to_string())?;

                        count += 1;
                    }
                }
            }
        }
    }

    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES (?, 'true')")
        .bind(&sync_key)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    println!("[RuneSync] {} runas sincronizadas do DDragon (versão {}).", count, version);
    Ok(())
}

/// Valida e corrige TODOS os registros de runas no banco de dados.
/// Chamada após a sincronização para garantir integridade 4+2+3 em todos os campeões.
pub async fn validate_all_champion_runes(pool: &Pool<Sqlite>) -> Result<(usize, usize), String> {
    let rows: Vec<(i32, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, champion_id, COALESCE(role, 'UNKNOWN'), runes_json FROM recommended_builds WHERE runes_json IS NOT NULL"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut total = 0usize;
    let mut fixed = 0usize;

    for (id, champ_id, _role, runes_json_opt) in rows {
        total += 1;
        let runes_json = match runes_json_opt {
            Some(j) => j,
            None => continue,
        };

        let parsed: Value = match serde_json::from_str(&runes_json) {
            Ok(v) => v,
            Err(_) => {
                // Completely malformed JSON — apply fallback
                let (fp, fs, fr, fsh) = get_fallback_runes_for_class("");
                let canonical = build_canonical_runes_json(fp, fs, &fr, &fsh);
                sqlx::query("UPDATE recommended_builds SET runes_json = ? WHERE id = ?")
                    .bind(&canonical)
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| e.to_string())?;
                fixed += 1;
                continue;
            }
        };

        let primary_tree = parsed["primary_tree"].as_i64().unwrap_or(8000);
        let secondary_tree = parsed["secondary_tree"].as_i64().unwrap_or(8100);
        let runes: Vec<i64> = parsed["runes"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();
        let shards: Vec<i64> = parsed["shards"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();

        if validate_rune_page(&runes, &shards) {
            continue; // Already valid
        }

        // Fetch champion tags for class-based fallback
        let tags: String = sqlx::query_scalar("SELECT COALESCE(tags, '[]') FROM champions WHERE id = ?")
            .bind(&champ_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
            .unwrap_or_default();

        let (fp, fs, fr, fsh) = fix_rune_page(primary_tree, secondary_tree, &runes, &shards, &tags);
        let canonical = build_canonical_runes_json(fp, fs, &fr, &fsh);

        sqlx::query("UPDATE recommended_builds SET runes_json = ? WHERE id = ?")
            .bind(&canonical)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

        println!("[RuneSync] Corrigido: {} ({} runas → 6, {} shards → 3)", champ_id, runes.len(), shards.len());
        fixed += 1;
    }

    // Also fix blitz_builds
    let blitz_rows: Vec<(i32, String, Option<String>)> = sqlx::query_as(
        "SELECT id, champion_id, runes_json FROM blitz_builds WHERE runes_json IS NOT NULL"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    for (id, champ_id, runes_json_opt) in blitz_rows {
        total += 1;
        let runes_json = match runes_json_opt {
            Some(j) => j,
            None => continue,
        };

        let parsed: Value = match serde_json::from_str(&runes_json) {
            Ok(v) => v,
            Err(_) => {
                let (fp, fs, fr, fsh) = get_fallback_runes_for_class("");
                let canonical = build_canonical_runes_json(fp, fs, &fr, &fsh);
                sqlx::query("UPDATE blitz_builds SET runes_json = ? WHERE id = ?")
                    .bind(&canonical)
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| e.to_string())?;
                fixed += 1;
                continue;
            }
        };

        let primary_tree = parsed["primary_tree"].as_i64().unwrap_or(8000);
        let secondary_tree = parsed["secondary_tree"].as_i64().unwrap_or(8100);
        let runes: Vec<i64> = parsed["runes"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();
        let shards: Vec<i64> = parsed["shards"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();

        if validate_rune_page(&runes, &shards) {
            continue;
        }

        let tags: String = sqlx::query_scalar("SELECT COALESCE(tags, '[]') FROM champions WHERE id = ?")
            .bind(&champ_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
            .unwrap_or_default();

        let (fp, fs, fr, fsh) = fix_rune_page(primary_tree, secondary_tree, &runes, &shards, &tags);
        let canonical = build_canonical_runes_json(fp, fs, &fr, &fsh);

        sqlx::query("UPDATE blitz_builds SET runes_json = ? WHERE id = ?")
            .bind(&canonical)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

        fixed += 1;
    }

    println!("[RuneSync] Validação concluída: {}/{} registros corrigidos.", fixed, total);
    Ok((total, fixed))
}

/// Command: Sincroniza runas do DDragon e valida integridade de todos os campeões.
#[tauri::command]
pub async fn sync_and_validate_runes_command(
    state: tauri::State<'_, crate::db::DbState>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    use tauri::Emitter;
    let pool = &state.0;

    let _ = app.emit("sync-progress", serde_json::json!({
        "progress": 5,
        "message": "Buscando versão do DDragon...",
        "done": false
    }));

    let version = crate::ddragon::get_latest_version_internal(pool).await?;

    let _ = app.emit("sync-progress", serde_json::json!({
        "progress": 15,
        "message": "Sincronizando dados de runas do DDragon...",
        "done": false
    }));

    sync_runes_from_ddragon(pool, &version, crate::config::DEFAULT_LANG).await?;

    let _ = app.emit("sync-progress", serde_json::json!({
        "progress": 50,
        "message": "Validando integridade (4+2+3) de todos os campeões...",
        "done": false
    }));

    let (total, fixed) = validate_all_champion_runes(pool).await?;

    let _ = app.emit("sync-progress", serde_json::json!({
        "progress": 100,
        "message": format!("Validação concluída! {} de {} registros corrigidos.", fixed, total),
        "done": true
    }));

    Ok(serde_json::json!({
        "version": version,
        "total_checked": total,
        "fixed": fixed
    }))
}

/// Command: Reset TOTAL do banco de dados (Opção A).
/// Apaga TODOS os dados de todas as tabelas e limpa todos os flags de sincronização.
/// O esquema (tabelas) é preservado — apenas os dados são removidos.
/// Após este comando, o app faz resincronização completa via DDragon + U.GG.
#[tauri::command]
pub async fn reset_builds_and_runes_command(
    state: tauri::State<'_, crate::db::DbState>,
) -> Result<String, String> {
    let pool = &state.0;

    println!("[RuneSync] INICIANDO RESET TOTAL DO BANCO DE DADOS...");

    // Wipe ALL data tables
    let tables = [
        "recommended_builds",
        "blitz_builds",
        "blitz_tier_list",
        "runes",
        "items",
        "champions",
        "matchups",
        "situational_items",
        "matches",
        "sync_metadata",
    ];

    for table in &tables {
        match sqlx::query(&format!("DELETE FROM {}", table)).execute(pool).await {
            Ok(res) => println!("[RuneSync] Tabela '{}' limpa: {} registros removidos.", table, res.rows_affected()),
            Err(e) => println!("[RuneSync] Aviso ao limpar '{}': {}", table, e),
        }
    }

    println!("[RuneSync] RESET TOTAL CONCLUÍDO. Banco de dados zerado e pronto para resincronização completa.");
    Ok("Banco de dados zerado com sucesso. Reinicie a aplicação ou execute 'Sincronizar' para repopular todos os dados.".to_string())
}
