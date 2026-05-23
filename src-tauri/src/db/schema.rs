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
            elo TEXT NOT NULL DEFAULT 'CHALLENGER',
            is_core BOOLEAN DEFAULT 0,
            items_json TEXT,
            runes_json TEXT,
            skill_priority TEXT,
            starting_items_json TEXT,
            summoner_spells_json TEXT,
            skill_order_json TEXT,
            first_item_time_ms INTEGER,
            jungle_path TEXT,
            FOREIGN KEY (champion_id) REFERENCES champions(id),
            UNIQUE(champion_id, role, elo)
        )"
    ).execute(pool).await?;

    // Migrations for existing databases
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN starting_items_json TEXT").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN summoner_spells_json TEXT").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN skill_order_json TEXT").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN first_item_time_ms INTEGER").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN jungle_path TEXT").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN elo TEXT NOT NULL DEFAULT 'CHALLENGER'").execute(pool).await;
    // updated_at: rastreia quando cada row foi atualizada — necessário para GC.
    // SQLite não permite DEFAULT CURRENT_TIMESTAMP em ALTER TABLE ADD COLUMN (só constantes literais).
    // Usamos DEFAULT NULL: as queries de INSERT/UPDATE explicitamente definem CURRENT_TIMESTAMP.
    let _ = sqlx::query("ALTER TABLE recommended_builds ADD COLUMN updated_at DATETIME").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE blitz_builds ADD COLUMN updated_at DATETIME").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE blitz_tier_list ADD COLUMN updated_at DATETIME").execute(pool).await;

    // Migração de wards: dados antigos foram salvos com elo='DIAMOND'.
    // O sistema atual usa 'HIGH_ELO' (Challenger+GM+Master combinados).
    // Renomeia os registros antigos para que as queries existentes os encontrem.
    let _ = sqlx::query("UPDATE ward_heatmaps SET elo = 'HIGH_ELO' WHERE elo = 'DIAMOND'").execute(pool).await;

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
            updated_at DATETIME,
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
            updated_at DATETIME,
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

    // Índices para o GC (queries por updated_at em tabelas grandes)
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_recommended_builds_updated ON recommended_builds(updated_at)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blitz_tier_list_updated ON blitz_tier_list(updated_at)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blitz_builds_updated ON blitz_builds(updated_at)").execute(pool).await?;
    // Índice para lookup de matchups por ELO (novo multi-ELO)
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_matchups_elo ON matchups(champion_id, opponent_id, elo)").execute(pool).await?;

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

    // 7. Groq Cache — evita chamar a API para o mesmo campeão/matchup/item
    // Chave: champion|opponent|item (mesmo cenário = mesma dica)
    // TTL: 14 dias (meta não muda a cada partida)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS groq_cache (
            cache_key TEXT PRIMARY KEY NOT NULL,
            response_json TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            expires_at TIMESTAMP NOT NULL
        )"
    ).execute(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_groq_cache_expiry ON groq_cache(expires_at)").execute(pool).await?;

    Ok(())
}
