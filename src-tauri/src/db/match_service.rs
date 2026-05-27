use sqlx::{Pool, Sqlite};
use serde_json::Value;

pub async fn save_match(pool: &Pool<Sqlite>, match_id: &str, data: &Value) -> Result<(), sqlx::Error> {
    let data_str = data.to_string();
    sqlx::query(
        "INSERT OR REPLACE INTO matches (match_id, data) VALUES (?, ?)"
    )
    .bind(match_id)
    .bind(data_str)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_match(pool: &Pool<Sqlite>, match_id: &str) -> Result<Option<Value>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT data FROM matches WHERE match_id = ?")
        .bind(match_id)
        .fetch_optional(pool)
        .await?;

    if let Some((data,)) = row {
        if let Ok(json) = serde_json::from_str(&data) {
            return Ok(Some(json));
        }
    }
    Ok(None)
}

#[allow(dead_code)]
pub async fn get_unanalyzed_matches(pool: &Pool<Sqlite>) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT match_id FROM matches WHERE analyzed = 0")
        .fetch_all(pool)
        .await?;
    
    Ok(rows.into_iter().map(|(id,)| id).collect())
}
