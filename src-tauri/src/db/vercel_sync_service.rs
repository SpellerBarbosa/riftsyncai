use sqlx::{Pool, Sqlite};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Semaphore;

const API_BASE: &str = "https://spellcoachapiv2-bxyh.vercel.app";
// "ALL" removido — query sem filtro de elo é a mais pesada e os dados são redundantes
const ELOS: &[&str] = &["IRON","BRONZE","SILVER","GOLD","PLATINUM","EMERALD","DIAMOND","MASTER","GRANDMASTER","CHALLENGER"];
const ROLES: &[&str] = &["TOP","JUNGLE","MID","ADC","UTILITY"];
const DETAIL_LIMIT_PER_ROLE: usize = 25;
// Concorrência reduzida para não sobrecarregar o PostgreSQL do Vercel
const META_CONCURRENCY: usize = 4;
const DETAIL_CONCURRENCY: usize = 6;
const MATCHUP_CONCURRENCY: usize = 6;
const MIN_MATCHUP_GAMES: i64 = 50;

// ── Response Structs ──────────────────────────────────────────────────────────

// API v2 returns { "status": "success", "data": [...] }
#[derive(Debug, Deserialize)]
struct ApiListResponse<T> { data: Vec<T> }
#[derive(Debug, Deserialize)]
struct ApiSingleResponse<T> { data: T }

// winRate/pickRate/banRate are percentage strings ("52.0") in v2 API
fn deser_rate<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    use serde::de::{self, Visitor, Unexpected};
    use std::fmt;
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Option<f64>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "number or string") }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> { Ok(Some(v / 100.0)) }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> { Ok(Some(v as f64 / 100.0)) }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> { Ok(Some(v as f64 / 100.0)) }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            v.parse::<f64>().map(|f| Some(f / 100.0))
                .map_err(|_| de::Error::invalid_value(Unexpected::Str(v), &self))
        }
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> { Ok(None) }
        fn visit_some<D2: serde::Deserializer<'de>>(self, d: D2) -> Result<Self::Value, D2::Error> {
            d.deserialize_any(self)
        }
    }
    d.deserialize_option(V)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApiChampionStats {
    pub champion_id: Option<i32>,
    pub role: Option<String>,
    pub elo: Option<String>,
    pub tier: Option<String>,
    #[serde(default, deserialize_with = "deser_rate")]
    pub win_rate: Option<f64>,
    #[serde(default, deserialize_with = "deser_rate")]
    pub pick_rate: Option<f64>,
    #[serde(default, deserialize_with = "deser_rate")]
    pub ban_rate: Option<f64>,
    pub core_build: Option<Vec<i64>>,
    pub runes: Option<Vec<i64>>,
    pub spells: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiChampionDetails {
    pub starting_items: Option<Vec<i64>>,
    pub skill_order: Option<Vec<i32>>,
    pub jungle_path: Option<String>,
    pub runes: Option<Vec<i64>>,
    pub core_build: Option<Vec<i64>>,
    pub wards: Option<Vec<WardCoord>>,
}

#[derive(Debug, Deserialize)]
pub struct WardCoord {
    pub x: Option<f64>,
    pub y: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMatchupData {
    games_played: i64,
    win_rate_a:   f64,  // percentual (50.03) → dividir por 100 ao armazenar
    win_rate_b:   f64,
}

// Dado processado de uma requisição de meta (resultado do parse)
#[derive(Clone)]
struct MetaRecord {
    slug: String,
    elo: String,
    role: String,
    tier: String,
    win_rate: f64,
    pick_rate: f64,
    ban_rate: f64,
    items_json: Option<String>,
    runes_json: Option<String>,
    spells_json: Option<String>,
    matchups: Vec<MatchupRecord>,
}

#[derive(Clone)]
struct MatchupRecord {
    slug: String,
    opponent: String,
    elo: String,
    win_rate: f64,
    games: i32,
    wins: i32,
}

// ── Sync State ────────────────────────────────────────────────────────────────

use std::sync::{Mutex, OnceLock};

static SYNC_STATE: OnceLock<Mutex<(i32, String, bool)>> = OnceLock::new();

fn get_sync_state_mutex() -> &'static Mutex<(i32, String, bool)> {
    SYNC_STATE.get_or_init(|| Mutex::new((0, "Iniciando...".to_string(), false)))
}

#[tauri::command]
pub fn get_sync_state() -> (i32, String, bool) {
    get_sync_state_mutex().lock().map(|s| s.clone()).unwrap_or((0, "Iniciando...".to_string(), false))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn map_role(raw: &str) -> &'static str {
    match raw.to_uppercase().as_str() {
        "TOP"                          => "TOP",
        "JUNGLE" | "JG"                => "JUNGLE",
        "MID" | "MIDDLE"               => "MID",
        "ADC" | "BOTTOM" | "BOT"       => "ADC",
        "SUPPORT" | "SUPP" | "UTILITY" => "SUPPORT",
        _                              => "MID",
    }
}

fn calculate_difficulty(wr: f64) -> i32 {
    if wr > 0.58 { 1 } else if wr > 0.55 { 2 } else if wr > 0.52 { 3 }
    else if wr > 0.50 { 4 } else if wr > 0.48 { 5 } else if wr > 0.45 { 6 }
    else if wr > 0.42 { 7 } else if wr > 0.40 { 8 } else if wr > 0.35 { 9 }
    else { 10 }
}


fn emit(app: Option<&AppHandle>, pct: i32, msg: &str, done: bool) {
    if let Ok(mut s) = get_sync_state_mutex().lock() { *s = (pct, msg.to_string(), done); }
    if let Some(a) = app {
        let _ = a.emit("sync-progress", json!({"progress": pct, "message": msg, "done": done}));
    }
}

fn api_token() -> String {
    std::env::var("API_ACCESS_TOKEN")
        .or_else(|_| option_env!("API_ACCESS_TOKEN").map(|s| s.to_string()).ok_or(()))
        .unwrap_or_default()
}

fn make_client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| Client::new())
}

async fn fetch_json<T: for<'de> Deserialize<'de>>(
    client: &Client,
    url: &str,
    token: &str,
) -> Option<T> {
    // A API pode retornar HTTP 5xx com body JSON válido (bug no servidor Vercel).
    // Por isso tentamos parsear o body independente do status code.
    // Só retentamos se o body for inválido/vazio (cold start real).
    const SERVER_ERROR_DELAYS: &[u64] = &[2000, 5000, 10000, 15000];

    for attempt in 1..=5u8 {
        let resp = client.get(url)
            .header("x-api-key", token)
            .header("User-Agent", "SpellCoachIA/2.0")
            .send().await;

        match resp {
            Ok(r) => {
                let status = r.status();
                match r.text().await {
                    Ok(body) => {
                        // Tenta parsear o body independente do HTTP status
                        match serde_json::from_str::<T>(&body) {
                            Ok(data) => {
                                if !status.is_success() {
                                    println!("[Sync] HTTP {} com dados válidos — usando. ({})", status, url);
                                }
                                return Some(data);
                            }
                            Err(_) => {
                                // Body inválido: pode ser cold start real
                                if status.as_u16() == 404 {
                                    println!("[Sync] 404 — endpoint não existe: {}", url);
                                    return None;
                                }
                                if attempt < 5 {
                                    println!("[Sync] HTTP {} body inválido, tentativa {}/5: {}", status, attempt, url);
                                    let delay = SERVER_ERROR_DELAYS[(attempt as usize - 1).min(SERVER_ERROR_DELAYS.len() - 1)];
                                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("[Sync] Erro ao ler body tentativa {}/5 ({}): {}", attempt, url, e);
                        if attempt < 5 {
                            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        }
                    }
                }
            }
            Err(e) => {
                println!("[Sync] Net error tentativa {}/5 ({}): {}", attempt, url, e);
                if attempt < 5 {
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                }
            }
        }
    }
    println!("[Sync] Falhou após 5 tentativas — sem dados para: {}", url);
    None
}

async fn fetch_list<T: for<'de> Deserialize<'de>>(client: &Client, url: &str, token: &str) -> Option<Vec<T>> {
    fetch_json::<ApiListResponse<T>>(client, url, token).await.map(|r| r.data)
}

async fn fetch_single<T: for<'de> Deserialize<'de>>(client: &Client, url: &str, token: &str) -> Option<T> {
    fetch_json::<ApiSingleResponse<T>>(client, url, token).await.map(|r| r.data)
}

// ── Main Sync ─────────────────────────────────────────────────────────────────

pub async fn sync_vercel_data(pool: &Pool<Sqlite>, app: Option<&AppHandle>) -> Result<(), String> {
    // Cache de 2h
    let last: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM sync_metadata WHERE key = 'vercel_last_sync'"
    ).fetch_optional(pool).await.unwrap_or(None);
    if let Some((ts,)) = last {
        if let Ok(t) = ts.parse::<u64>() {
            let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().saturating_sub(t);
            if elapsed < 7200 {
                let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blitz_tier_list")
                    .fetch_one(pool).await.unwrap_or(0);
                if n > 0 {
                    println!("[Sync] Cache válido ({:.0} min). Pulando.", elapsed as f64 / 60.0);
                    emit(app, 100, "Dados já atualizados!", true);
                    return Ok(());
                }
            }
        }
    }

    emit(app, 0, "Iniciando sincronização com SpellCoach API v2...", false);

    // WAL mode para writes mais rápidos
    let _ = sqlx::query("PRAGMA journal_mode=WAL").execute(pool).await;
    let _ = sqlx::query("PRAGMA synchronous=NORMAL").execute(pool).await;

    let token = api_token();
    let client = Arc::new(make_client());

    // ── Warm-up: acorda a função Vercel antes do burst paralelo ──────────────
    // Envia um request leve e aguarda sucesso para evitar cold-start em cascata.
    {
        emit(app, 1, "Conectando à API...", false);
        let warmup_url = format!("{}/api/health", API_BASE);
        let warmup_client = (*client).clone();
        let warmup_token = token.clone();
        let mut warmed = false;
        for attempt in 1..=5u8 {
            match warmup_client.get(&warmup_url)
                .header("x-api-key", &warmup_token)
                .send().await
            {
                Ok(r) if r.status().is_success() || r.status().as_u16() == 401 => {
                    warmed = true;
                    break;
                }
                Ok(r) => {
                    println!("[Sync] Warm-up HTTP {} tentativa {}/5", r.status(), attempt);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
                Err(e) => {
                    println!("[Sync] Warm-up erro tentativa {}/5: {}", attempt, e);
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            }
        }
        if !warmed {
            return Err("API indisponível após 5 tentativas de warm-up.".to_string());
        }
        println!("[Sync] API aquecida. Iniciando burst paralelo.");
    }

    // ── Campeões (DDragon) ────────────────────────────────────────────────────
    let champ_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM champions")
        .fetch_one(pool).await.unwrap_or(0);
    if champ_count == 0 {
        emit(app, 2, "Carregando campeões (DDragon)...", false);
        if let Ok(version) = crate::ddragon::get_latest_version_internal(pool).await {
            let dc = crate::ddragon::DDragonClient::new();
            let data = match dc.get(&format!("/cdn/{}/data/pt_BR/champion.json", version)).await {
                Ok(d) => d,
                Err(_) => dc.get(&format!("/cdn/{}/data/en_US/champion.json", version))
                    .await.map_err(|e| e.to_string())?,
            };
            crate::db::sync_service::sync_champions(pool, &data).await.map_err(|e| e.to_string())?;
        }
    }

    let rows: Vec<(i32, String)> = sqlx::query_as("SELECT key, id FROM champions")
        .fetch_all(pool).await.map_err(|e| e.to_string())?;
    let champ_map: Arc<HashMap<i32, String>> = Arc::new(rows.into_iter().collect());
    let slug_to_key: Arc<HashMap<String, i32>> =
        Arc::new(champ_map.iter().map(|(k, v)| (v.clone(), *k)).collect());

    // ── FASE 1: Meta em paralelo ──────────────────────────────────────────────
    emit(app, 4, "Buscando meta em paralelo (elos × roles)...", false);

    let sem_meta = Arc::new(Semaphore::new(META_CONCURRENCY));
    let total_meta = ELOS.len() * ROLES.len();
    let meta_done = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Gera todos os pares (elo, role) e lança tasks simultâneas
    let mut meta_tasks = Vec::with_capacity(total_meta);
    for &elo in ELOS {
        for &role in ROLES {
            let permit = sem_meta.clone().acquire_owned().await.map_err(|e| e.to_string())?;
            let client = client.clone();
            let token = token.clone();
            let champ_map = champ_map.clone();
            let done_ctr = meta_done.clone();
            let app_handle = app.cloned();
            let elo = elo.to_string();
            let role = role.to_string();

            meta_tasks.push(tokio::spawn(async move {
                let _permit = permit;
                let url = format!("{}/api/stats/champions?elo={}&role={}", API_BASE, elo, role);
                let data: Option<Vec<ApiChampionStats>> = fetch_list(&client, &url, &token).await;

                let n = done_ctr.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                let pct = 4 + (n * 68 / total_meta) as i32;
                emit(app_handle.as_ref(), pct,
                    &format!("Meta: {}/{} ({}×{})", n, total_meta, elo, role), false);

                let records: Vec<MetaRecord> = match data {
                    None => return vec![],
                    Some(list) => {
                        list.into_iter().filter_map(|cs| {
                            let num_id = cs.champion_id?;
                            let slug = champ_map.get(&num_id).cloned()
                                .unwrap_or_else(|| format!("Champion{}", num_id));
                            let champ_elo = cs.elo.clone()
                                .unwrap_or_else(|| elo.clone()).to_uppercase();
                            let mapped_role = map_role(cs.role.as_deref().unwrap_or(&role)).to_string();
                            let tier = cs.tier.as_deref().unwrap_or("B").to_string();

                            let items_json = cs.core_build.as_ref()
                                .filter(|v| !v.is_empty())
                                .map(|v| serde_json::to_string(v).unwrap_or_default());

                            // meta runes: [primary_tree_id, keystone_id]
                            let runes_json_str = cs.runes.as_ref()
                                .filter(|r| r.len() >= 2)
                                .map(|r| serde_json::to_string(&json!({
                                    "primary_tree": r[0], "secondary_tree": null,
                                    "runes": [r[1]], "shards": []
                                })).unwrap_or_default());

                            let spells_json = cs.spells.as_ref()
                                .filter(|v| !v.is_empty())
                                .map(|v| serde_json::to_string(v).unwrap_or_default());

                            Some(MetaRecord {
                                slug, elo: champ_elo, role: mapped_role, tier,
                                win_rate: cs.win_rate.unwrap_or(0.0),
                                pick_rate: cs.pick_rate.unwrap_or(0.0),
                                ban_rate: cs.ban_rate.unwrap_or(0.0),
                                items_json, runes_json: runes_json_str, spells_json,
                                matchups: Vec::new(),
                            })
                        }).collect()
                    }
                };
                records
            }));
        }
    }

    // Coleta todos os resultados
    let mut all_meta: Vec<MetaRecord> = Vec::new();
    for task in meta_tasks {
        if let Ok(recs) = task.await { all_meta.extend(recs); }
    }
    println!("[Sync] Fase 1 completa: {} registros coletados.", all_meta.len());

    if all_meta.is_empty() {
        return Err("Nenhum dado retornado pela API — verifique o token, URL e formato JSON.".to_string());
    }

    // ── Commit Phase 1 em transação única ────────────────────────────────────
    emit(app, 73, "Salvando meta no banco de dados...", false);
    {
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

        for rec in &all_meta {
            if let Err(e) = sqlx::query(
                "INSERT OR REPLACE INTO blitz_tier_list (elo, role, champion_id, tier, win_rate, pick_rate, ban_rate)
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            ).bind(&rec.elo).bind(&rec.role).bind(&rec.slug)
            .bind(&rec.tier).bind(rec.win_rate).bind(rec.pick_rate).bind(rec.ban_rate)
            .execute(&mut *tx).await {
                eprintln!("[Sync] blitz_tier_list insert error ({}:{}): {}", rec.slug, rec.elo, e);
            }

            // blitz_builds: only when we have item data from meta (may be empty in v2 API)
            if let (Some(ref items), Some(ref runes)) = (&rec.items_json, &rec.runes_json) {
                if let Err(e) = sqlx::query(
                    "INSERT OR REPLACE INTO blitz_builds (champion_id, role, elo, items_json, runes_json)
                     VALUES (?, ?, ?, ?, ?)"
                ).bind(&rec.slug).bind(&rec.role).bind(&rec.elo)
                .bind(items).bind(runes)
                .execute(&mut *tx).await {
                    eprintln!("[Sync] blitz_builds insert error ({}:{}): {}", rec.slug, rec.role, e);
                }
            }

            // recommended_builds: cria o row sem runes_json — Phase 2 preencherá com
            // os 11 IDs completos do endpoint de detalhes. Armazenar as 2 runas parciais
            // do meta aqui causaria "Página inválida: 1 runa, 0 shards" no RuneSync.
            if let Err(e) = sqlx::query(
                "INSERT OR REPLACE INTO recommended_builds
                 (champion_id, role, is_core, items_json, runes_json, summoner_spells_json)
                 VALUES (?, ?, 1, ?, NULL, ?)"
            ).bind(&rec.slug).bind(&rec.role)
            .bind(rec.items_json.as_deref())
            .bind(rec.spells_json.as_deref())
            .execute(&mut *tx).await {
                eprintln!("[Sync] recommended_builds insert error ({}:{}): {}", rec.slug, rec.role, e);
            }

            for m in &rec.matchups {
                if let Err(e) = sqlx::query(
                    "INSERT OR REPLACE INTO matchups
                     (champion_id, opponent_id, elo, difficulty, win_rate, games_count, wins_count)
                     VALUES (?, ?, ?, ?, ?, ?, ?)"
                ).bind(&m.slug).bind(&m.opponent).bind(&m.elo)
                .bind(calculate_difficulty(m.win_rate)).bind(m.win_rate).bind(m.games).bind(m.wins)
                .execute(&mut *tx).await {
                    eprintln!("[Sync] matchups insert error ({}→{}): {}", m.slug, m.opponent, e);
                }
            }
        }

        tx.commit().await.map_err(|e| e.to_string())?;
        println!("[Sync] Fase 1 commitada no banco.");
    }

    // ── FASE 2: Detalhes — top N por role em DIAMOND, paralelo ───────────────
    emit(app, 75, "Selecionando top campeões para detalhes...", false);

    // Seleciona top DETAIL_LIMIT_PER_ROLE por role (maior pick_rate em DIAMOND)
    let mut detail_map: HashMap<String, Vec<(String, i32)>> = HashMap::new(); // role -> [(slug, num_id)]
    {
        // Agrupa os registros DIAMOND por role ordenados por pick_rate desc
        let mut diamond: Vec<&MetaRecord> = all_meta.iter()
            .filter(|r| r.elo == "DIAMOND")
            .collect();
        diamond.sort_by(|a, b| b.pick_rate.partial_cmp(&a.pick_rate).unwrap_or(std::cmp::Ordering::Equal));

        for rec in diamond {
            let entry = detail_map.entry(rec.role.clone()).or_default();
            if entry.len() >= DETAIL_LIMIT_PER_ROLE { continue; }
            if let Some(&num_id) = slug_to_key.get(&rec.slug) {
                if !entry.iter().any(|(s, _)| s == &rec.slug) {
                    entry.push((rec.slug.clone(), num_id));
                }
            }
        }
    }

    // Fase 2: detalhes de CHALLENGER — builds, runas e skills com maior win rate.
    // Wards também de CHALLENGER — referência de alto nível para ensinar progressão.
    let detail_targets: Vec<(String, String, i32)> = detail_map.into_iter()
        .flat_map(|(role, champs)| champs.into_iter().map(move |(slug, id)| (slug, role.clone(), id)))
        .collect();
    let total_details = detail_targets.len();
    let det_done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let sem_det = Arc::new(Semaphore::new(DETAIL_CONCURRENCY));
    let mut det_tasks = Vec::with_capacity(total_details);

    println!("[Sync] Fase 2: {} chamadas de detalhe (CHALLENGER).", total_details);

    for (slug, role, num_id) in detail_targets {
        let permit = sem_det.clone().acquire_owned().await.map_err(|e| e.to_string())?;
        let client = client.clone();
        let token = token.clone();
        let done_ctr = det_done.clone();
        let app_handle = app.cloned();

        det_tasks.push(tokio::spawn(async move {
            let _permit = permit;
            let url = format!("{}/api/stats/champion/details?championId={}&role={}&elo=CHALLENGER",
                API_BASE, num_id, role);
            let detail: Option<ApiChampionDetails> = fetch_single(&client, &url, &token).await;

            let n = done_ctr.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if n % 5 == 0 {
                let pct = 75 + (n * 20 / total_details.max(1)) as i32;
                emit(app_handle.as_ref(), pct.min(94),
                    &format!("Detalhes: {}/{}", n, total_details), false);
            }
            (slug, role, detail)
        }));
    }

    let mut detail_results: Vec<(String, String, String, ApiChampionDetails)> = Vec::new();
    for task in det_tasks {
        if let Ok((slug, role, Some(d))) = task.await {
            detail_results.push((slug, role, "CHALLENGER".to_string(), d));
        }
    }
    println!("[Sync] Fase 2 completa: {}/{} detalhes obtidos.", detail_results.len(), total_details);

    emit(app, 95, "Salvando detalhes e wards no banco...", false);
    {
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

        for (slug, role, elo, d) in &detail_results {
            let starting  = d.starting_items.as_ref().filter(|v| !v.is_empty())
                .map(|v| serde_json::to_string(v).unwrap_or_default());
            let spells: Option<String> = None;
            let skill_ord = d.skill_order.as_ref().filter(|v| !v.is_empty())
                .map(|v| {
                    let names: Vec<&str> = v.iter().map(|&i| match i {
                        1 => "Q", 2 => "W", 3 => "E", 4 => "R", _ => "?"
                    }).collect();
                    serde_json::to_string(&names).unwrap_or_default()
                });
            let runes_det = d.runes.as_ref().filter(|r| r.len() >= 6).map(|r| {
                let primary = r[0];
                let secondary = r[5];
                let main_runes: Vec<i64> = r.get(1..5).unwrap_or(&[]).to_vec();
                let sub_runes:  Vec<i64> = r.get(6..8).unwrap_or(&[]).to_vec();
                let shards:     Vec<i64> = r.get(8..).unwrap_or(&[]).to_vec();
                let all_runes: Vec<i64> = main_runes.into_iter().chain(sub_runes).collect();
                serde_json::to_string(&json!({
                    "primary_tree": primary, "secondary_tree": secondary,
                    "runes": all_runes, "shards": shards
                })).unwrap_or_default()
            });
            let core = d.core_build.as_ref().filter(|v| !v.is_empty())
                .map(|v| serde_json::to_string(v).unwrap_or_default());

            // Upsert recommended_builds com ELO
            if starting.is_some() || skill_ord.is_some() || runes_det.is_some()
                || core.is_some() || d.jungle_path.is_some()
            {
                let _ = sqlx::query(
                    "INSERT OR IGNORE INTO recommended_builds (champion_id, role, elo, is_core)
                     VALUES (?, ?, ?, 1)"
                ).bind(slug).bind(role).bind(elo).execute(&mut *tx).await;

                let mut sets = Vec::new();
                if starting.is_some()      { sets.push("starting_items_json = ?"); }
                if spells.is_some()        { sets.push("summoner_spells_json = ?"); }
                if skill_ord.is_some()     { sets.push("skill_order_json = ?"); }
                if runes_det.is_some()     { sets.push("runes_json = ?"); }
                if core.is_some()          { sets.push("items_json = ?"); }
                if d.jungle_path.is_some() { sets.push("jungle_path = ?"); }

                if !sets.is_empty() {
                    let q = format!(
                        "UPDATE recommended_builds SET {} WHERE champion_id=? AND role=? AND elo=?",
                        sets.join(", ")
                    );
                    let mut query = sqlx::query(&q);
                    if let Some(ref v) = starting  { query = query.bind(v); }
                    if let Some(ref v) = spells    { query = query.bind(v); }
                    if let Some(ref v) = skill_ord { query = query.bind(v); }
                    if let Some(ref v) = runes_det { query = query.bind(v); }
                    if let Some(ref v) = core      { query = query.bind(v); }
                    if let Some(ref v) = d.jungle_path { query = query.bind(v); }
                    if let Err(e) = query.bind(slug).bind(role).bind(elo).execute(&mut *tx).await {
                        eprintln!("[Sync] recommended_builds update error ({}:{}:{}): {}", slug, role, elo, e);
                    }
                }
            }

            // Wards: apex tiers combinados (CHALLENGER+GRANDMASTER+MASTER) via API
            // Armazenado como 'HIGH_ELO' para desacoplar do ELO específico
            let wards = d.wards.as_deref().unwrap_or(&[]);
            let valid_wards: Vec<(i64, i64)> = wards.iter()
                .filter_map(|w| Some((w.x? as i64, w.y? as i64)))
                .collect();

            if !valid_wards.is_empty() {
                let _ = sqlx::query(
                    "DELETE FROM ward_heatmaps WHERE champion_id = ? AND role = ? AND elo = 'HIGH_ELO'"
                ).bind(slug).bind(role).execute(&mut *tx).await;

                let placeholders = valid_wards.iter()
                    .map(|_| "(?, ?, 'HIGH_ELO', ?, ?)").collect::<Vec<_>>().join(",");
                let q = format!("INSERT INTO ward_heatmaps (champion_id, role, elo, x_coord, y_coord) VALUES {}", placeholders);
                let mut query = sqlx::query(&q);
                for (x, y) in &valid_wards {
                    query = query.bind(slug).bind(role).bind(x).bind(y);
                }
                if let Err(e) = query.execute(&mut *tx).await {
                    eprintln!("[Sync] ward_heatmaps insert error ({}:{}): {}", slug, role, e);
                }
            }
        } // for detail_results

        tx.commit().await.map_err(|e| e.to_string())?;
        println!("[Sync] Fase 2 commitada.");
    } // tx block

    // ── FASE 3: Matchups DIAMOND ──────────────────────────────────────────────
    emit(app, 96, "Coletando pares de matchup DIAMOND...", false);

    let mut role_champs: HashMap<String, Vec<(String, i32)>> = HashMap::new();
    for rec in all_meta.iter().filter(|r| r.elo == "DIAMOND") {
        if let Some(&num_id) = slug_to_key.get(&rec.slug) {
            let entry = role_champs.entry(rec.role.clone()).or_default();
            if !entry.iter().any(|(s, _)| s == &rec.slug) {
                entry.push((rec.slug.clone(), num_id));
            }
        }
    }

    struct MatchupTarget { slug_a: String, id_a: i32, slug_b: String, id_b: i32 }
    let mut pairs: Vec<MatchupTarget> = Vec::new();
    for champs in role_champs.values() {
        for i in 0..champs.len() {
            for j in (i + 1)..champs.len() {
                pairs.push(MatchupTarget {
                    slug_a: champs[i].0.clone(), id_a: champs[i].1,
                    slug_b: champs[j].0.clone(), id_b: champs[j].1,
                });
            }
        }
    }
    let total_pairs = pairs.len();
    println!("[Sync] Fase 3: {} pares de matchup DIAMOND.", total_pairs);

    let sem_mu  = Arc::new(Semaphore::new(MATCHUP_CONCURRENCY));
    let mu_done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut mu_tasks: Vec<tokio::task::JoinHandle<Option<(String, String, ApiMatchupData)>>>
        = Vec::with_capacity(total_pairs);

    for pair in pairs {
        let permit     = sem_mu.clone().acquire_owned().await.map_err(|e| e.to_string())?;
        let client     = client.clone();
        let token      = token.clone();
        let done_ctr   = mu_done.clone();
        let app_handle = app.cloned();

        mu_tasks.push(tokio::spawn(async move {
            let _permit = permit;
            let url = format!(
                "{}/api/stats/matchup?championAId={}&championBId={}",
                API_BASE, pair.id_a, pair.id_b
            );
            let result: Option<ApiMatchupData> = fetch_single(&client, &url, &token).await;
            let n = done_ctr.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if n % 10 == 0 || n == total_pairs {
                let pct = 96 + (n * 3 / total_pairs.max(1)) as i32;
                emit(app_handle.as_ref(), pct.min(99), &format!("Matchups: {}/{}", n, total_pairs), false);
            }
            result.map(|d| (pair.slug_a, pair.slug_b, d))
        }));
    }

    let mut matchup_rows: Vec<(String, String, ApiMatchupData)> = Vec::new();
    for task in mu_tasks {
        if let Ok(Some(row)) = task.await { matchup_rows.push(row); }
    }
    println!("[Sync] Fase 3: {}/{} matchups obtidos.", matchup_rows.len(), total_pairs);

    emit(app, 99, "Salvando matchups no banco...", false);
    {
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
        for (slug_a, slug_b, d) in &matchup_rows {
            if d.games_played < MIN_MATCHUP_GAMES { continue; }
            let wr_a = d.win_rate_a / 100.0;
            let wr_b = d.win_rate_b / 100.0;
            let wins_a = (d.games_played as f64 * wr_a).round() as i32;
            let wins_b = d.games_played as i32 - wins_a;

            if let Err(e) = sqlx::query(
                "INSERT OR REPLACE INTO matchups (champion_id, opponent_id, elo, difficulty, win_rate, games_count, wins_count) VALUES (?, ?, 'DIAMOND', ?, ?, ?, ?)"
            ).bind(slug_a).bind(slug_b).bind(calculate_difficulty(wr_a)).bind(wr_a)
            .bind(d.games_played as i32).bind(wins_a).execute(&mut *tx).await {
                eprintln!("[Sync] matchup error ({}→{}): {}", slug_a, slug_b, e);
            }

            if let Err(e) = sqlx::query(
                "INSERT OR REPLACE INTO matchups (champion_id, opponent_id, elo, difficulty, win_rate, games_count, wins_count) VALUES (?, ?, 'DIAMOND', ?, ?, ?, ?)"
            ).bind(slug_b).bind(slug_a).bind(calculate_difficulty(wr_b)).bind(wr_b)
            .bind(d.games_played as i32).bind(wins_b).execute(&mut *tx).await {
                eprintln!("[Sync] matchup error ({}→{}): {}", slug_b, slug_a, e);
            }
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        println!("[Sync] Fase 3 commitada: {} pares → {} linhas.", matchup_rows.len(), matchup_rows.len() * 2);
    }

    // ── Timestamp ─────────────────────────────────────────────────────────────
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let _ = sqlx::query(
        "INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('vercel_last_sync', ?)"
    ).bind(now.to_string()).execute(pool).await;

    // Restaura modo síncrono padrão
    let _ = sqlx::query("PRAGMA synchronous=FULL").execute(pool).await;

    emit(app, 100, "Sincronização concluída com sucesso!", true);
    println!("[Sync] Finalizado: {} meta + {} detalhes + {} matchup pares.", all_meta.len(), detail_results.len(), matchup_rows.len());
    Ok(())
}

#[tauri::command]
pub async fn sync_vercel_command(
    state: State<'_, crate::db::DbState>,
    app: AppHandle,
) -> Result<(), String> {
    sync_vercel_data(&state.0, Some(&app)).await
}

/// Força sincronização ignorando o cache de 2h — útil para diagnóstico e após updates.
#[tauri::command]
pub async fn force_sync_vercel_command(
    state: State<'_, crate::db::DbState>,
    app: AppHandle,
) -> Result<(), String> {
    sqlx::query("DELETE FROM sync_metadata WHERE key = 'vercel_last_sync'")
        .execute(&state.0)
        .await
        .map_err(|e| e.to_string())?;
    println!("[Sync] Cache invalidado — forçando sincronização completa.");
    sync_vercel_data(&state.0, Some(&app)).await
}

/// Retorna um resumo de quantos registros existem no banco por ELO e tabela.
/// Permite verificar exatamente quais ELOs foram populados com sucesso.
#[tauri::command]
pub async fn get_sync_coverage(
    state: State<'_, crate::db::DbState>,
) -> Result<serde_json::Value, String> {
    let pool = &state.0;

    // Contagem por ELO na blitz_tier_list (picks/bans)
    let tier_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT elo, COUNT(*) FROM blitz_tier_list GROUP BY elo ORDER BY elo"
    ).fetch_all(pool).await.unwrap_or_default();

    // Contagem por ELO nos matchups
    let matchup_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT elo, COUNT(*) FROM matchups GROUP BY elo ORDER BY elo"
    ).fetch_all(pool).await.unwrap_or_default();

    // Contagem por ELO nos recommended_builds
    let build_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT elo, COUNT(*) FROM recommended_builds GROUP BY elo ORDER BY elo"
    ).fetch_all(pool).await.unwrap_or_default();

    // Contagem de wards (apenas CHALLENGER)
    let ward_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ward_heatmaps WHERE elo = 'HIGH_ELO'"
    ).fetch_one(pool).await.unwrap_or(0);

    // Total de campeões com dados
    let champ_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT champion_id) FROM blitz_tier_list"
    ).fetch_one(pool).await.unwrap_or(0);

    let tier_map: serde_json::Value = tier_rows.iter()
        .map(|(e, c)| (e.clone(), serde_json::json!(c)))
        .collect::<serde_json::Map<_, _>>()
        .into();
    let matchup_map: serde_json::Value = matchup_rows.iter()
        .map(|(e, c)| (e.clone(), serde_json::json!(c)))
        .collect::<serde_json::Map<_, _>>()
        .into();
    let build_map: serde_json::Value = build_rows.iter()
        .map(|(e, c)| (e.clone(), serde_json::json!(c)))
        .collect::<serde_json::Map<_, _>>()
        .into();

    Ok(serde_json::json!({
        "champions_with_data": champ_count,
        "tier_list_por_elo": tier_map,
        "matchups_por_elo": matchup_map,
        "builds_por_elo": build_map,
        "wards_challenger": ward_count,
    }))
}
