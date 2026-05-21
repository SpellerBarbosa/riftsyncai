use sqlx::{Pool, Sqlite};

pub async fn create_tables(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    // 1. Reference Tables
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS champions (
            id TEXT PRIMARY KEY,
            key INTEGER NOT NULL,
            name TEXT NOT NULL,
            title TEXT,
            tags TEXT,
            image_url TEXT,
            spells_description TEXT
        )"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            gold_total INTEGER,
            gold_base INTEGER,
            components_json TEXT,
            plaintext TEXT
        )"
    ).execute(pool).await?;

    let _ = sqlx::query("ALTER TABLE items ADD COLUMN gold_base INTEGER").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE items ADD COLUMN components_json TEXT").execute(pool).await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS runes (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            icon TEXT,
            short_desc TEXT
        )"
    ).execute(pool).await?;

    // 2. Coaching & Relationship Tables
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS matchups (
            champion_id TEXT NOT NULL,
            opponent_id TEXT NOT NULL,
            elo TEXT NOT NULL DEFAULT 'GLOBAL',
            difficulty INTEGER DEFAULT 5,
            win_rate REAL,
            games_count INTEGER DEFAULT 0,
            wins_count INTEGER DEFAULT 0,
            tips TEXT,
            PRIMARY KEY (champion_id, opponent_id, elo),
            FOREIGN KEY (champion_id) REFERENCES champions(id),
            FOREIGN KEY (opponent_id) REFERENCES champions(id)
        )"
    ).execute(pool).await?;

    // recommended_builds — extended with fields from /api/stats/champion/details
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS recommended_builds (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            champion_id TEXT NOT NULL,
            role TEXT,
            is_core BOOLEAN DEFAULT 0,
            items_json TEXT,             -- core build items
            runes_json TEXT,             -- {primary_tree, secondary_tree, runes[], shards[]}
            skill_priority TEXT,         -- ex: 'Q > E > W'
            starting_items_json TEXT,    -- itens comprados nos primeiros 60s
            summoner_spells_json TEXT,   -- [spell1_id, spell2_id]
            skill_order_json TEXT,       -- JSON array de habilidades por nivel (ex: Q,W,Q,E,Q,R)
            first_item_time_ms INTEGER,  -- ms até o 1º item core
            jungle_path TEXT,            -- rota de limpeza (ex: 'Blue > Gromp > Wolves > Raptors')
            FOREIGN KEY (champion_id) REFERENCES champions(id),
            UNIQUE(champion_id, role)
        )"
    ).execute(pool).await?;

    // Migrations for existing databases
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN starting_items_json TEXT").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN summoner_spells_json TEXT").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN skill_order_json TEXT").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN first_item_time_ms INTEGER").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN jungle_path TEXT").execute(pool).await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS matches (
            match_id TEXT PRIMARY KEY,
            data TEXT NOT NULL,
            analyzed BOOLEAN DEFAULT 0,
            feedback TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )"
    ).execute(pool).await?;

    // 3. Metadata & Cache
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sync_metadata (
            key TEXT PRIMARY KEY,
            value TEXT
        )"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS situational_items (
            champion_id TEXT NOT NULL,
            opponent_id TEXT,
            opponent_type TEXT,
            slot_type TEXT NOT NULL,
            item_id INTEGER NOT NULL,
            win_rate REAL,
            games INTEGER,
            PRIMARY KEY (champion_id, opponent_id, opponent_type, slot_type, item_id),
            FOREIGN KEY (champion_id) REFERENCES champions(id)
        )"
    ).execute(pool).await?;

    // 4. Meta / Tier List
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS blitz_tier_list (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            elo TEXT NOT NULL,
            role TEXT NOT NULL,
            champion_id TEXT NOT NULL,
            tier TEXT,
            win_rate REAL,
            pick_rate REAL,
            ban_rate REAL,
            UNIQUE(elo, role, champion_id)
        )"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS blitz_builds (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            champion_id TEXT NOT NULL,
            role TEXT NOT NULL,
            elo TEXT NOT NULL,
            items_json TEXT,
            runes_json TEXT,
            UNIQUE(champion_id, role, elo)
        )"
    ).execute(pool).await?;

    // 5. Ward Heatmap — coordenadas de sentinelas por campeão/role/elo
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ward_heatmaps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            champion_id TEXT NOT NULL,
            role TEXT NOT NULL,
            elo TEXT NOT NULL DEFAULT 'DIAMOND',
            x_coord INTEGER NOT NULL,
            y_coord INTEGER NOT NULL,
            placed_at_ms INTEGER,
            FOREIGN KEY (champion_id) REFERENCES champions(id)
        )"
    ).execute(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_ward_heatmap ON ward_heatmaps(champion_id, role, elo)").execute(pool).await?;

    // 6. Voice Cache
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS voice_cache (
            id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
            phrase_key TEXT UNIQUE NOT NULL,
            audio_path TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            audio_format TEXT NOT NULL DEFAULT 'wav',
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            expires_at TIMESTAMP NOT NULL
        )"
    ).execute(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_voice_cache_key ON voice_cache(phrase_key)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_voice_cache_expiry ON voice_cache(expires_at)").execute(pool).await?;

    Ok(())
}
