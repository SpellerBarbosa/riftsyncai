use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Champion {
    pub id: String,
    pub key: i32,
    pub name: String,
    pub title: Option<String>,
    pub tags: Option<String>, // JSON
    pub image_url: Option<String>,
    pub spells_description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Matchup {
    pub champion_id: String,
    pub opponent_id: String,
    pub elo: String,
    pub difficulty: i32,
    pub win_rate: Option<f64>,
    pub games_count: Option<i32>,
    pub wins_count: Option<i32>,
    pub tips: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RecommendedBuild {
    pub id: i32,
    pub champion_id: String,
    pub role: Option<String>,
    pub is_core: bool,
    pub items_json: Option<String>,
    pub runes_json: Option<String>,
    pub skill_priority: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct BlitzTier {
    pub id: i32,
    pub elo: String,
    pub role: String,
    pub champion_id: String,
    pub tier: Option<String>,
    pub win_rate: Option<f64>,
    pub pick_rate: Option<f64>,
    pub ban_rate: Option<f64>,
}
