// Tracks active recommendations to praise the player when completed
#[derive(Clone, PartialEq)]
pub enum ActiveRec {
    RecallGold,
    RecallHp,
    PlaySafe { enemy_champ: String, start_time: f64, initial_deaths: i64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnemyTelemetry {
    pub champion_name: String,
    pub role: String,
    pub is_visible: bool,
    pub last_visible_time: f64,
    pub x: f32,
    pub y: f32,
    pub fog_duration: f64,
    pub last_health: f64,
    pub last_cs: i64,
    pub last_level: i64,
}

pub struct CoachState {
    pub last_phase: String,
    pub last_alert_time: f64,
    pub last_danger_alert_time: f64,
    pub last_purchase_alert_time: f64,
    pub active_recommendation: Option<ActiveRec>,

    // Telemetry and high-ELO macro additions
    pub last_gank_alert_time: f64,
    pub enemy_telemetries: Vec<EnemyTelemetry>,
    pub initialized_telemetry: bool,

    // Jungle macro tracking additions
    pub first_clear_step: u32,
    pub last_clear_step_time: f64,
    pub respawn_warnings_sent: std::collections::HashSet<String>,

    // Recall and roaming high-ELO macro tracking
    pub mid_roaming_suggested: bool,

    // Support-specific macro coaching tracking
    pub sup_bush_dominance_suggested: bool,

    // Top-specific macro coaching tracking
    pub top_grubs_priority_suggested: bool,

    // Skill level-up coaching tracker
    pub last_suggested_level_up: i32,

    // Role micro coaching: last time we fired a micro tip
    pub last_micro_time: f64,
    // Track previous CS to detect farming gaps
    pub last_cs_snapshot: i64,
    pub last_cs_snapshot_time: f64,
    // Track previous deaths to detect int-ing streaks
    pub prev_deaths: i64,
    // Minimap: last time we reminded
    pub last_minimap_time: f64,

    // Level-up tip: cooldown próprio independente do global (time-sensitive)
    pub last_levelup_emit_time: f64,

    // Jungle milestone coaching flags
    pub jg_level5_suggested: bool,
    pub jg_dragon_suggested: bool,

    // Objective warnings (legado — mantido para dynamic_tip.rs)
    pub baron_warned: bool,
    pub dragon_setup_warned: bool,
    pub grubs_last_chance_warned: bool,

    // ── Pre-ward objective system ─────────────────────────────────────────────
    // Rastreia o próximo spawn de cada objetivo para acionar o card de wards 40s antes.
    // Valores atualizados via parsing dos eventos do LCA (DragonKill, BaronKill, etc.).
    pub dragon_next_spawn: f64,  // segundos; começa em 300.0 (5:00), +300 após cada kill
    pub baron_next_spawn: f64,   // segundos; começa em 1200.0 (20:00), +360 após cada kill
    pub herald_done: bool,       // true após HeraldKill ou game_time > 825s
    /// Chaves das alertas já enviadas nesta partida ("dragon:300", "baron:1200", etc.)
    /// Evita repetição do mesmo alerta.
    pub objective_ward_alerts: std::collections::HashSet<String>,

    // Prevents same-category tip firing consecutively
    pub last_tip_category: String,

    // Rastreia quantas vezes cada categoria disparou nesta partida.
    // Resetado junto com CoachState no início de cada partida.
    pub shown_categories: std::collections::HashMap<String, u32>,

    // Aviso quando um inimigo volta a aparecer no minimapa após ficar na névoa.
    pub last_ward_sighting_alert_time: f64,
    pub recent_enemy_sighting: Option<(String, String, f64)>,

    // Alerta de oportunidade de objetivo quando 2+ inimigos morrem simultaneamente.
    pub last_objective_opportunity_time: f64,

    // Rastreamento de eventos do LCA (ChampionKill, Ace) para coaching contextual.
    // Guarda quantos eventos já foram processados para evitar re-disparar nas próximas ticks.
    pub last_processed_event_count: usize,
    pub pending_event_tip: Option<(String, String)>,

    // ── Dados do banco (carregados uma vez por partida) ──────────────────────
    /// Flag — dados do DB já foram carregados para esta partida.
    pub db_loaded: bool,
    /// Skill order recomendado pelo meta (["Q","W","Q","E","Q","R",...]).
    pub db_skill_order: Vec<String>,
    /// Rota de clear da selva do meta (ex: "Blue > Gromp > Wolves").
    pub db_jungle_path: Option<String>,
    /// Tempo médio (ms) do 1º item core — usado para alertas de recall.
    pub db_first_item_time_ms: Option<i64>,
    /// Já sugerimos o alerta de timing do 1º item nesta partida.
    pub db_first_item_alerted: bool,
    /// Itens iniciais recomendados pelo meta.
    pub db_starting_items: Vec<i64>,
    /// Já sugerimos os itens iniciais nesta partida.
    pub db_starting_items_suggested: bool,
    /// Dificuldade do matchup vs inimigo de rota (1=fácil, 10=counter).
    pub db_matchup_difficulty: Option<i32>,
    /// Último game_time em que o ward map foi exibido (evita spam).
    pub last_ward_map_time: f64,
    /// Último game_time em que o ward map de push foi exibido (após kill de laner).
    pub last_push_ward_time: f64,
    /// Último game_time em que request-groq-tip foi emitido (evita saturar a API).
    pub last_groq_trigger_time: f64,
    /// Flag — dica de início de partida via Groq já foi solicitada.
    pub groq_game_start_triggered: bool,
    /// ID DDragon resolvido para queries de DB (ex: "MissFortune" a partir de "Miss Fortune").
    /// Resolvido uma vez por partida para evitar mismatch entre nomes LCA e DDragon.
    pub db_champion_key: String,
    /// Role detectada na partida (para o relatório pós-jogo).
    pub last_role: String,
}

impl CoachState {
    pub fn new() -> Self {
        Self {
            last_phase: String::new(),
            last_alert_time: 0.0,
            last_danger_alert_time: 0.0,
            last_purchase_alert_time: 0.0,
            active_recommendation: None,
            last_gank_alert_time: 0.0,
            enemy_telemetries: Vec::new(),
            initialized_telemetry: false,
            first_clear_step: 0,
            last_clear_step_time: 0.0,
            respawn_warnings_sent: std::collections::HashSet::new(),
            mid_roaming_suggested: false,
            sup_bush_dominance_suggested: false,
            top_grubs_priority_suggested: false,
            last_suggested_level_up: 0,
            last_levelup_emit_time: -10.0,
            last_micro_time: 0.0,
            last_cs_snapshot: 0,
            last_cs_snapshot_time: 0.0,
            prev_deaths: 0,
            last_minimap_time: 0.0,
            jg_level5_suggested: false,
            jg_dragon_suggested: false,
            baron_warned: false,
            dragon_setup_warned: false,
            grubs_last_chance_warned: false,
            dragon_next_spawn: 300.0,
            baron_next_spawn: 1200.0,
            herald_done: false,
            objective_ward_alerts: std::collections::HashSet::new(),
            last_tip_category: String::new(),
            shown_categories: std::collections::HashMap::new(),
            last_ward_sighting_alert_time: 0.0,
            recent_enemy_sighting: None,
            last_objective_opportunity_time: 0.0,
            last_processed_event_count: 0,
            pending_event_tip: None,
            db_loaded: false,
            db_skill_order: Vec::new(),
            db_jungle_path: None,
            db_first_item_time_ms: None,
            db_first_item_alerted: false,
            db_starting_items: Vec::new(),
            db_starting_items_suggested: false,
            db_matchup_difficulty: None,
            last_ward_map_time: -300.0,
            last_push_ward_time: -300.0,
            last_groq_trigger_time: -300.0,
            groq_game_start_triggered: false,
            db_champion_key: String::new(),
            last_role: String::new(),
        }
    }
}
