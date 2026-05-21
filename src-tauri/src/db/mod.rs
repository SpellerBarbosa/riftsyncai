pub mod schema;
pub mod models;
pub mod sync_service;
pub mod coach_service;
pub mod match_service;
pub mod matchup_service;
pub mod build_sync_service;
pub mod rune_sync_service;
pub mod blitz_service;
pub mod vercel_sync_service;
pub mod player_style_service;

use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::fs;
use tauri::Manager;

pub type DbPool = Pool<Sqlite>;

pub struct DbState(pub DbPool);

pub async fn init_db(app_handle: &tauri::AppHandle) -> Result<DbPool, sqlx::Error> {
    let app_dir = app_handle.path().app_data_dir().expect("Failed to get app data dir");
    
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).expect("Failed to create app data dir");
    }
    
    let db_path = app_dir.join(crate::config::APP_DB_NAME);
    let db_url = format!("sqlite:{}", db_path.to_string_lossy());

    if !db_path.exists() {
        fs::File::create(&db_path).expect("Failed to create database file");
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // --- DATABASE MIGRATIONS ---
    // 1. Check if matchups has the 'elo' and 'games_count' columns. If not, drop it so it gets recreated with the new schema.
    let elo_check: Result<Option<String>, _> = sqlx::query_scalar("SELECT name FROM pragma_table_info('matchups') WHERE name = 'elo'")
        .fetch_optional(&pool)
        .await;
    let games_count_check: Result<Option<String>, _> = sqlx::query_scalar("SELECT name FROM pragma_table_info('matchups') WHERE name = 'games_count'")
        .fetch_optional(&pool)
        .await;

    if let Ok(None) = elo_check {
        println!("[Migration] Upgrading matchups table to support Elo-based data...");
        let _ = sqlx::query("DROP TABLE IF EXISTS matchups").execute(&pool).await;
    } else if let Ok(None) = games_count_check {
        println!("[Migration] Upgrading matchups table to support games_count and wins_count...");
        let _ = sqlx::query("DROP TABLE IF EXISTS matchups").execute(&pool).await;
    }

    // 2. Check if recommended_builds table has a UNIQUE constraint.
    let unique_check: Result<Option<String>, _> = sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE type='table' AND name='recommended_builds'")
        .fetch_optional(&pool)
        .await;

    if let Ok(Some(sql)) = unique_check {
        if !sql.contains("UNIQUE") {
            println!("[Migration] Upgrading recommended_builds table to support UNIQUE constraints...");
            let _ = sqlx::query("DROP TABLE IF EXISTS recommended_builds").execute(&pool).await;
        }
    }

    // 3. Check if champions has the 'spells_description' column. If not, drop it so it gets recreated.
    let spells_check = sqlx::query("SELECT spells_description FROM champions LIMIT 1")
        .execute(&pool)
        .await;

    if spells_check.is_err() {
        let table_exists: Result<Option<String>, _> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name='champions'")
            .fetch_optional(&pool)
            .await;
            
        if let Ok(Some(_)) = table_exists {
            println!("[Migration] Upgrading champions table to support spells_description...");
            let _ = sqlx::query("DROP TABLE IF EXISTS champions").execute(&pool).await;
            let _ = sqlx::query("DELETE FROM sync_metadata WHERE key LIKE 'hydration_done_%'").execute(&pool).await;
        }
    }

    // Initialize schema
    schema::create_tables(&pool).await?;

    // 4. Auto-validate rune integrity on startup (background, non-blocking)
    let pool_for_rune_check = pool.clone();
    tauri::async_runtime::spawn(async move {
        println!("[Migration] Verificando integridade de runas (4+2+3)...");
        match rune_sync_service::validate_all_champion_runes(&pool_for_rune_check).await {
            Ok((total, fixed)) => {
                if fixed > 0 {
                    println!("[Migration] Integridade de runas: {}/{} registros corrigidos automaticamente.", fixed, total);
                } else {
                    println!("[Migration] Integridade de runas OK: {} registros validados.", total);
                }
            }
            Err(e) => eprintln!("[Migration] Aviso ao validar runas: {}", e),
        }
    });

    Ok(pool)
}
