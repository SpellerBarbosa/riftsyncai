use sqlx::{Pool, Sqlite};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use std::sync::{Mutex, OnceLock};

// Armazena o último relatório gerado — a janela post-game lê ao montar
static LAST_REPORT: OnceLock<Mutex<Option<PostGameReport>>> = OnceLock::new();

fn report_store() -> &'static Mutex<Option<PostGameReport>> {
    LAST_REPORT.get_or_init(|| Mutex::new(None))
}

/// Comando chamado pela janela post-game ao montar para obter o relatório.
#[tauri::command]
pub fn get_post_game_report() -> Option<PostGameReport> {
    report_store().lock().ok()?.clone()
}

/// Salva um relatório no store — usado pelo frontend para persistir mock/preview.
#[tauri::command]
pub fn set_post_game_report(report: PostGameReport) {
    if let Ok(mut store) = report_store().lock() {
        *store = Some(report);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetricGrade {
    pub label: String,
    pub player_value: f64,
    pub benchmark_value: f64,
    pub unit: String,
    pub grade: String,       // "S", "A", "B", "C", "D"
    pub feedback: String,    // dica concreta de melhoria
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostGameReport {
    pub champion: String,
    pub role: String,
    pub win: bool,
    pub duration_min: f64,
    pub metrics: Vec<MetricGrade>,
    pub overall_grade: String,
    pub priority_tip: String,  // a dica mais importante da partida
}

fn grade(player: f64, bench: f64, higher_is_better: bool) -> &'static str {
    let ratio = if higher_is_better {
        if bench == 0.0 { 1.0 } else { player / bench }
    } else {
        if player == 0.0 { 1.0 } else { bench / player }
    };
    if ratio >= 1.10 { "S" }
    else if ratio >= 0.95 { "A" }
    else if ratio >= 0.80 { "B" }
    else if ratio >= 0.65 { "C" }
    else { "D" }
}

fn overall(metrics: &[MetricGrade]) -> String {
    let score: f64 = metrics.iter().map(|m| match m.grade.as_str() {
        "S" => 5.0, "A" => 4.0, "B" => 3.0, "C" => 2.0, _ => 1.0,
    }).sum::<f64>() / metrics.len().max(1) as f64;
    if score >= 4.5 { "S".to_string() }
    else if score >= 3.5 { "A".to_string() }
    else if score >= 2.5 { "B".to_string() }
    else if score >= 1.5 { "C".to_string() }
    else { "D".to_string() }
}

/// Analisa a partida recém encerrada e emite o relatório para o frontend.
pub async fn analyze_and_emit(
    handle: &tauri::AppHandle,
    pool: &Pool<Sqlite>,
    match_data: &serde_json::Value,
    champion_id: &str,
    role: &str,
) {
    let participants = match match_data["info"]["participants"].as_array() {
        Some(p) => p,
        None => return,
    };

    let player = participants.iter().find(|p| {
        let champ = p["championName"].as_str().unwrap_or("").to_lowercase();
        let champ_clean = champ.replace(' ', "").replace('\'', "");
        let target = champion_id.to_lowercase();
        champ == target || champ_clean == target
    });

    let player = match player {
        Some(p) => p,
        None => {
            println!("[PostGame] Não encontrou o jogador na partida.");
            return;
        }
    };

    let win = player["win"].as_bool().unwrap_or(false);
    let kills = player["kills"].as_f64().unwrap_or(0.0);
    let deaths = player["deaths"].as_f64().unwrap_or(1.0);
    let assists = player["assists"].as_f64().unwrap_or(0.0);
    let cs = player["totalMinionsKilled"].as_f64().unwrap_or(0.0)
        + player["neutralMinionsKilled"].as_f64().unwrap_or(0.0);
    let vision = player["visionScore"].as_f64().unwrap_or(0.0);
    let duration_secs = match_data["info"]["gameDuration"].as_f64().unwrap_or(1.0);
    let duration_min = duration_secs / 60.0;

    let cs_min = if duration_min > 0.0 { cs / duration_min } else { 0.0 };
    let vision_min = if duration_min > 0.0 { vision / duration_min } else { 0.0 };

    // Busca benchmarks do high elo no banco
    let bench: Option<(f64, f64, f64)> = sqlx::query_as(
        "SELECT AVG(CAST(total_cs AS REAL) / CAST(total_duration AS REAL) * 60),
                AVG(CAST(total_vision_score AS REAL) / CAST(total_duration AS REAL) * 60),
                AVG(CAST(deaths AS REAL) / CAST(games_played AS REAL))
         FROM blitz_tier_list b
         WHERE b.champion_id = ? AND b.elo IN ('CHALLENGER','GRANDMASTER','MASTER')"
    ).bind(champion_id).fetch_optional(pool).await.ok().flatten();

    // Benchmarks por role — selva tem CS/min menor que laner (campos < minions)
    let default_cs = match role.to_uppercase().as_str() {
        "JUNGLE"  => 5.5,
        "SUPPORT" => 1.5,
        _         => 7.5,
    };

    let (bench_cs_min, bench_vision_min, bench_deaths) = bench
        .map(|(cs, vis, d)| (
            if cs > 0.0 { cs } else { default_cs },
            if vis > 0.0 { vis } else { 0.6 },
            if d > 0.0 { d } else { 3.5 },
        ))
        .unwrap_or((default_cs, 0.6, 3.5));

    let mut metrics = vec![
        MetricGrade {
            label: "CS por minuto".to_string(),
            player_value: (cs_min * 10.0).round() / 10.0,
            benchmark_value: (bench_cs_min * 10.0).round() / 10.0,
            unit: "CS/min".to_string(),
            grade: grade(cs_min, bench_cs_min, true).to_string(),
            feedback: cs_feedback(cs_min, bench_cs_min),
        },
        MetricGrade {
            label: "Visão".to_string(),
            player_value: (vision_min * 100.0).round() / 100.0,
            benchmark_value: (bench_vision_min * 100.0).round() / 100.0,
            unit: "vis/min".to_string(),
            grade: grade(vision_min, bench_vision_min, true).to_string(),
            feedback: vision_feedback(vision_min, bench_vision_min),
        },
        MetricGrade {
            label: "Mortes".to_string(),
            player_value: deaths,
            benchmark_value: (bench_deaths * 10.0).round() / 10.0,
            unit: "mortes".to_string(),
            grade: grade(deaths, bench_deaths, false).to_string(),
            feedback: deaths_feedback(deaths, bench_deaths),
        },
        MetricGrade {
            label: "KDA".to_string(),
            player_value: ((kills + assists) / deaths.max(1.0) * 10.0).round() / 10.0,
            benchmark_value: 3.0,
            unit: "ratio".to_string(),
            grade: grade((kills + assists) / deaths.max(1.0), 3.0, true).to_string(),
            feedback: kda_feedback(kills, deaths, assists),
        },
    ];

    // Ordena por pior nota primeiro para destacar o que mais precisa melhorar
    metrics.sort_by(|a, b| {
        let score = |g: &str| match g { "S" => 5, "A" => 4, "B" => 3, "C" => 2, _ => 1 };
        score(&a.grade).cmp(&score(&b.grade))
    });

    let overall_grade = overall(&metrics);

    // A dica prioritária é sobre a métrica mais fraca
    let priority_tip = metrics.first()
        .map(|m| format!("{}: {}", m.label, m.feedback))
        .unwrap_or_else(|| "Continue treinando e focando em farm consistente.".to_string());

    let report = PostGameReport {
        champion: champion_id.to_string(),
        role: role.to_string(),
        win,
        duration_min: (duration_min * 10.0).round() / 10.0,
        metrics,
        overall_grade,
        priority_tip,
    };

    println!("[PostGame] Relatório gerado: nota {} para {} ({:?})",
        report.overall_grade, champion_id, if win { "VITÓRIA" } else { "DERROTA" });

    // Salva no store global — a janela post-game lê ao montar via get_post_game_report()
    if let Ok(mut store) = report_store().lock() {
        *store = Some(report.clone());
    }
    let _ = handle.emit("post-game-report", &report);
}

fn cs_feedback(player: f64, bench: f64) -> String {
    if player >= bench * 1.05 {
        "Farm excelente. Mantenha a consistência no last hit.".to_string()
    } else if player >= bench * 0.85 {
        format!("Farm bom, mas {:.1} CS/min abaixo do ideal. Priorize ondas entre ganks.", bench - player)
    } else {
        format!("Farm precisa melhorar — {:.1} CS/min a menos. Fique na rota nos primeiros 15min.", bench - player)
    }
}

fn vision_feedback(player: f64, bench: f64) -> String {
    if player >= bench * 0.95 {
        "Visão de mapa excelente. Wards nos momentos certos.".to_string()
    } else if player >= bench * 0.70 {
        "Compre Control Wards com mais frequência e ward antes dos objetivos.".to_string()
    } else {
        "Visão muito baixa. Ward o rio antes de avançar e sempre use o slot de sentinela.".to_string()
    }
}

fn deaths_feedback(deaths: f64, bench: f64) -> String {
    if deaths <= bench * 1.1 {
        "Morreu dentro do esperado. Bom posicionamento.".to_string()
    } else if deaths <= bench * 1.5 {
        format!("{:.0} mortes — evite trocas quando o JG inimigo não está visível.", deaths)
    } else {
        format!("{:.0} mortes — jogue mais conservador. Cada morte dá 300g de vantagem ao inimigo.", deaths)
    }
}

fn kda_feedback(kills: f64, deaths: f64, assists: f64) -> String {
    let kda = (kills + assists) / deaths.max(1.0);
    if kda >= 4.0 {
        "KDA impecável. Converta essa vantagem em objetivos.".to_string()
    } else if kda >= 2.5 {
        "KDA razoável. Reduza mortes para aumentar o impacto.".to_string()
    } else {
        "KDA abaixo do ideal. Priorize sobreviver — um player vivo vale mais que um player morto com kills.".to_string()
    }
}
