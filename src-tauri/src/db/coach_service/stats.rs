/*
===============================================================================
                       SPELL COACH IA - ESTATÍSTICAS E BANCO
===============================================================================
Este submódulo gerencia o acesso ao banco de dados SQLite (via SQLx) e calcula
as recomendações de Seleção de Campeões (Draft): Blind Picks, Counters e Bans.

Para um estudante de programação:
* O que é Lazy Hydration (Hidratação Preguiçosa)?
  Se o jogador solicitar as recomendações de um campeão recém-lançado que ainda não
  está no nosso banco de dados, o aplicativo não dá erro! Ele percebe a falta dos dados,
  faz o download assíncrono dos detalhes dele diretamente da API Data Dragon da Riot,
  salva no banco (hidrata) e refaz a consulta automaticamente em frações de segundo.
* O que é o CASE WHEN no SQL?
  Usamos ordenações condicionais no SQL para priorizar registros que correspondam 
  ao Elo exato do jogador. Se não houver dados desse Elo, o banco nos traz dados 
  Globais (GLOBAL) automaticamente como fallback em uma única consulta rápida!
===============================================================================
*/

use sqlx::{Pool, Sqlite};
use tauri::State;
use crate::db::models::{Matchup, RecommendedBuild, Champion, BlitzTier};
use crate::db::DbState;
// Importamos os utilitários de cálculo que separamos no submódulo de Heurísticas
use super::heuristics::{
    get_min_pick_rate_for_games,
    get_min_non_zero_pick_rate,
    is_viable_for_role,
    is_low_elo_trap,
    sort_by_bayesian_win_rate,
    sort_matchups_by_bayesian_win_rate,
};

/// Recupera as estatísticas de confronto direto (matchup) entre dois campeões específicos.
/// 
/// **Para o estudante (Lógica SQL):**
/// Usamos `ORDER BY (CASE WHEN elo = ? THEN 0 WHEN elo = 'GLOBAL' THEN 1 ELSE 2 END)`.
/// Isso faz o banco ordenar os resultados dando prioridade máxima (peso 0) para o Elo do jogador,
/// prioridade secundária (peso 1) para dados globais, e peso 2 para outros elos. O `LIMIT 1` 
/// nos garante apenas a melhor e mais específica linha correspondente!
pub async fn get_matchup(
    pool: &Pool<Sqlite>, 
    champ_id: &str, 
    opponent_id: &str,
    elo: Option<&str>
) -> Result<Option<Matchup>, sqlx::Error> {
    let target_elo = elo.unwrap_or("GLOBAL").to_uppercase();
    sqlx::query_as::<_, Matchup>(
        "SELECT * FROM matchups 
         WHERE champion_id = ? AND opponent_id = ? 
         ORDER BY (CASE WHEN elo = ? THEN 0 WHEN elo = 'GLOBAL' THEN 1 ELSE 2 END), win_rate DESC 
         LIMIT 1"
    )
    .bind(champ_id)
    .bind(opponent_id)
    .bind(&target_elo)
    .fetch_optional(pool)
    .await
}

/// Helper para converter o ID numérico clássico (Key) do campeão da Riot para sua String ID.
pub async fn get_champion_id_by_key(pool: &Pool<Sqlite>, key: i32) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM champions WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
}

/// Salva ou atualiza um registro de confronto direto no banco de dados.
#[allow(dead_code)]
pub async fn add_matchup(
    pool: &Pool<Sqlite>, 
    champ_id: &str, 
    opponent_id: &str, 
    difficulty: i32, 
    tips: &str
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO matchups (champion_id, opponent_id, difficulty, tips) VALUES (?, ?, ?, ?)"
    )
    .bind(champ_id)
    .bind(opponent_id)
    .bind(difficulty)
    .bind(tips)
    .execute(pool)
    .await?;
    Ok(())
}

/// Obtém todas as builds registradas para um campeão.
#[allow(dead_code)]
pub async fn get_recommended_builds(pool: &Pool<Sqlite>, champ_id: &str) -> Result<Vec<RecommendedBuild>, sqlx::Error> {
    sqlx::query_as::<_, RecommendedBuild>(
        "SELECT * FROM recommended_builds WHERE champion_id = ?"
    )
    .bind(champ_id)
    .fetch_all(pool)
    .await
}

/// Comando Tauri para carregar a build recomendada de um campeão de forma robusta e inteligente.
/// 
/// **Para o estudante (Padrão Cache Lazy Hydration):**
/// 1. Mapeia a rota do jogador para nomes correspondentes no banco de dados.
/// 2. Busca nas builds pré-calculadas (do Blitz.gg) no Elo e Rota específicos.
/// 3. Se não houver, busca nas builds padrão por Rota (de recomendação estática).
/// 4. Se ainda assim estiver vazio (o banco de dados nunca ouviu falar desse campeão):
///    * Dispara a **Lazy Hydration**! Consulta a API Data Dragon oficial da Riot usando o crate `crate::ddragon`.
///    * Insere os dados de itens no banco SQLite.
///    * Refaz a consulta ao banco de dados e retorna a build hidratada.
/// Este é um dos maiores padrões de Clean Code para gerenciamento de dados remotos persistidos localmente!
#[tauri::command]
pub async fn get_recommended_builds_command(
    state: State<'_, DbState>,
    champ_id: String,
    role: Option<String>,
    elo: Option<String>,
) -> Result<Vec<RecommendedBuild>, String> {
    let pool = &state.0;
    
    // resolve_champion_db_id usa fetch_optional (seguro) + matching flexível por id/name/normalized
    let resolved_id = {
        let r = resolve_champion_db_id(pool, &champ_id).await;
        if r.is_empty() { champ_id.clone() } else { r }
    };

    let mapped_role = role.as_ref().map(|r| match r.to_lowercase().as_str() {
        "top" => "TOP",
        "jungle" => "JUNGLE",
        "middle" | "mid" => "MID",
        "bottom" | "adc" => "ADC",
        "utility" | "support" => "SUPPORT",
        _ => r.as_str(),
    }.to_string());
    
    let mapped_elo = elo.as_ref().map(|e| e.to_uppercase());

    let mut builds = Vec::new();

    // 1. Blitz Builds
    if let (Some(r), Some(e)) = (&mapped_role, &mapped_elo) {
        let blitz_row: Option<(String, String)> = sqlx::query_as(
            "SELECT items_json, runes_json FROM blitz_builds WHERE champion_id = ? AND role = ? AND elo = ?"
        )
        .bind(&resolved_id)
        .bind(r)
        .bind(e)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((items, runes)) = blitz_row {
            builds.push(RecommendedBuild {
                id: 0,
                champion_id: resolved_id.clone(),
                role: Some(r.clone()),
                is_core: true,
                items_json: Some(items),
                runes_json: Some(runes),
                skill_priority: None,
            });
        }
    }

    // 2. Recommended Builds — prioridade: high elo mais alto com dados
    let elo_order = "CASE elo WHEN 'CHALLENGER' THEN 1 WHEN 'GRANDMASTER' THEN 2 WHEN 'MASTER' THEN 3 WHEN 'DIAMOND' THEN 4 ELSE 5 END";
    if builds.is_empty() {
        builds = if let Some(ref r) = mapped_role {
            sqlx::query_as::<_, RecommendedBuild>(&format!(
                "SELECT * FROM recommended_builds WHERE champion_id = ?
                 ORDER BY (CASE WHEN role = ? THEN 0 ELSE 1 END), {}, id DESC", elo_order
            ))
            .bind(&resolved_id).bind(r)
            .fetch_all(pool).await.unwrap_or_default()
        } else {
            sqlx::query_as::<_, RecommendedBuild>(&format!(
                "SELECT * FROM recommended_builds WHERE champion_id = ?
                 ORDER BY {}, id DESC", elo_order
            ))
            .bind(&resolved_id)
            .fetch_all(pool).await.unwrap_or_default()
        };
    }

    // 3. LAZY HYDRATION (Cache-Miss Hydrate)
    if builds.is_empty() {
        println!("Lazy hydration disparada para: {}", resolved_id);
        if let Ok(version) = crate::ddragon::get_latest_version_internal(pool).await {
            let _ = crate::ddragon::get_ddragon_champion_details_internal(pool, &version, crate::config::DEFAULT_LANG, &resolved_id).await;
            builds = if let Some(ref r) = mapped_role {
                sqlx::query_as::<_, RecommendedBuild>(&format!(
                    "SELECT * FROM recommended_builds WHERE champion_id = ?
                     ORDER BY (CASE WHEN role = ? THEN 0 ELSE 1 END), {}, id DESC", elo_order
                ))
                .bind(&resolved_id).bind(r)
                .fetch_all(pool).await.unwrap_or_default()
            } else {
                sqlx::query_as::<_, RecommendedBuild>(&format!(
                    "SELECT * FROM recommended_builds WHERE champion_id = ?
                     ORDER BY {}, id DESC", elo_order
                ))
                .bind(&resolved_id)
                .fetch_all(pool).await.unwrap_or_default()
            };
        }
    }

    Ok(builds)
}

/// Comando Tauri para atualizar manualmente builds meta específicas de campeões.
#[allow(dead_code)]
#[tauri::command]
pub async fn update_meta_build_command(
    state: State<'_, DbState>,
    champ_id: String,
    items: Vec<String>,
    role: String,
) -> Result<(), String> {
    let pool = &state.0;
    let items_json = serde_json::to_string(&items).unwrap_or_default();
    
    sqlx::query(
        "INSERT OR REPLACE INTO recommended_builds (champion_id, role, is_core, items_json) VALUES (?, ?, ?, ?)"
    )
    .bind(champ_id)
    .bind(role)
    .bind(true)
    .bind(items_json)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    
    Ok(())
}

/// Comando Tauri para listar todos os campeões registrados no banco.
#[tauri::command]
pub async fn get_db_champions(state: State<'_, DbState>) -> Result<Vec<Champion>, String> {
    let pool = &state.0;
    sqlx::query_as::<_, Champion>("SELECT * FROM champions ORDER BY name ASC")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Coleta estatísticas consolidadas para sugestão de duelo na tela.
#[allow(dead_code)]
pub async fn get_coaching_suggestion(
    pool: &Pool<Sqlite>, 
    player_champ: &str, 
    opponent_champ: &str
) -> Result<serde_json::Value, sqlx::Error> {
    let matchup = get_matchup(pool, player_champ, opponent_champ, None).await?;
    let builds = get_recommended_builds(pool, player_champ).await?;
    
    Ok(serde_json::json!({
        "matchup": matchup,
        "recommended_builds": builds
    }))
}

/// Recomenda os dois melhores campeões recomendados para aquela role estatisticamente.
#[allow(dead_code)]
#[tauri::command]
pub async fn get_top_champions(state: State<'_, DbState>, role: String) -> Result<Vec<Champion>, String> {
    let pool = &state.0;
    
    let db_role = match role.to_lowercase().as_str() {
        "jungle" => "JUNGLE".to_string(),
        "utility" | "support" => "SUPPORT".to_string(),
        "bottom" | "adc" => "ADC".to_string(),
        "middle" | "mid" => "MID".to_string(),
        "top" => "TOP".to_string(),
        _ => role.to_uppercase(),
    };

    println!("[Coach] Tentando buscar campeões para role: {} (Mapeado: {})", role, db_role);

    let mut top_ids: Vec<String> = sqlx::query_scalar(
        "SELECT m.champion_id FROM matchups m
         JOIN recommended_builds rb ON m.champion_id = rb.champion_id
         WHERE rb.role = ?
         GROUP BY m.champion_id 
         ORDER BY AVG(m.win_rate) DESC 
         LIMIT 2"
    )
    .bind(&db_role)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if top_ids.is_empty() {
        println!("[Coach] Sem dados para role {}. Buscando top global...", db_role);
        top_ids = sqlx::query_scalar(
            "SELECT champion_id FROM matchups 
             GROUP BY champion_id 
             ORDER BY AVG(win_rate) DESC 
             LIMIT 2"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    }

    if top_ids.is_empty() {
        println!("[Coach] Banco de estatísticas vazio. Usando fallback de emergência...");
        top_ids = sqlx::query_scalar("SELECT id FROM champions LIMIT 2")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    }

    let mut champions = Vec::new();
    for id in top_ids {
        if let Ok(Some(champ)) = sqlx::query_as::<_, Champion>("SELECT * FROM champions WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await {
            champions.push(champ);
        }
    }

    println!("[Coach] Sugerindo: {:?}", champions.iter().map(|c| &c.name).collect::<Vec<_>>());
    Ok(champions)
}

/// Comando Tauri para fornecer dados agregados rápidos simplificados da Tier List do Blitz.gg.
#[allow(dead_code)]
#[tauri::command]
pub async fn get_blitz_recommendations(role: String, rank: String) -> Result<serde_json::Value, String> {
    let _is_low_elo = rank.contains("BRONZE") || rank.contains("SILVER") || rank.contains("IRON");
    
    let (picks, bans) = match role.to_lowercase().as_str() {
        "top" => (vec!["Yorick", "Teemo"], vec!["Darius", "Illaoi"]),
        "jungle" => (vec!["Rammus", "Shyvana"], vec!["Master Yi", "Amumu"]),
        "middle" => (vec!["Malzahar", "Naafiri"], vec!["Yasuo", "Zed"]),
        "bottom" => (vec!["Smolder", "Miss Fortune"], vec!["Samira", "Vayne"]),
        "utility" => (vec!["Teemo", "Xerath"], vec!["Blitzcrank", "Lux"]),
        _ => (vec!["Garen", "Annie"], vec!["Master Yi", "Lux"]),
    };

    Ok(serde_json::json!({
        "source": "Blitz.gg",
        "role": role,
        "rank": rank,
        "recommended_picks": picks,
        "recommended_bans": bans
    }))
}

/// Calcula a lista dos 3 melhores campeões para BANIMENTO (Bans) táticos na fase de seleção.
/// 
/// **Para o estudante (Filtragens Inteligentes):**
/// 1. Tenta coletar campeões com altos win/pick rates no elo alvo. Se a amostragem for pequena, 
///    faz fallback automático para dados estáveis do elo PRATA (SILVER).
/// 2. Aplica filtros fundamentais usando o `.retain`:
///    * `.retain(|c| c.pick_rate >= min_pr)` - Elimina campeões que quase ninguém joga (irrelevantes para gastar ban).
///    * `.retain(|c| is_viable_for_role(...))` - Elimina seleções bizarras/inválidas.
///    * `.retain(|c| !is_low_elo_trap(...))` - Garante que não sugira banir um campeão que os inimigos já 
///      vão errar sozinhos no elo baixo (como banir Ezreal ou Lee Sin no Bronze, onde eles têm win rate péssimo).
/// 3. Aplica a ordenação Bayesiana e escolhe os 3 campeões de maior impacto.
pub async fn get_top_bans(pool: &Pool<Sqlite>, elo: &str, role: &str) -> Result<Vec<BlitzTier>, sqlx::Error> {
    let mapped_role = match role.to_lowercase().as_str() {
        "top" => "TOP",
        "jungle" => "JUNGLE",
        "middle" | "mid" => "MID",
        "bottom" | "adc" => "ADC",
        "utility" | "support" => "SUPPORT",
        _ => role,
    };

    let min_non_zero = get_min_non_zero_pick_rate(pool, elo, &mapped_role).await;
    let total_games = if min_non_zero > 0.0 { 1.0 / min_non_zero } else { 1000.0 };
    
    let target_elo = if total_games < 150.0 && elo != "SILVER" && elo != "GOLD" {
        println!("[Coach] Base de dados local para Elo {} é muito pequena ({:.0} jogos). Usando SILVER como base estável.", elo, total_games);
        "SILVER"
    } else {
        elo
    };

    println!("[Coach] Buscando bans para Elo: {}, Role: {} (Mapeado: {}) [Usando {} para dados estatísticos]", elo, role, mapped_role, target_elo);
    
    // Consulta 1: Tenta S/OP no elo específico
    let mut tiers = sqlx::query_as::<_, BlitzTier>(
        "SELECT * FROM blitz_tier_list WHERE elo = ? AND role = ? AND tier IN ('S', 'OP') AND pick_rate >= 0.02 ORDER BY win_rate DESC LIMIT 30"
    )
    .bind(target_elo)
    .bind(mapped_role)
    .fetch_all(pool)
    .await?;

    // Fallback 1: SILVER
    if tiers.is_empty() && target_elo != "GOLD" && target_elo != "SILVER" {
        println!("[Coach] Sem dados meta para {}. Tentando fallback para SILVER...", target_elo);
        tiers = sqlx::query_as::<_, BlitzTier>(
            "SELECT * FROM blitz_tier_list WHERE elo = 'SILVER' AND role = ? AND tier IN ('S', 'OP') AND pick_rate >= 0.02 ORDER BY win_rate DESC LIMIT 30"
        )
        .bind(mapped_role)
        .fetch_all(pool)
        .await?;
    }

    // Fallback 2: Qualquer dado geral
    if tiers.is_empty() {
        println!("[Coach] Fallback final: Buscando qualquer dado para a role {}...", mapped_role);
        tiers = sqlx::query_as::<_, BlitzTier>(
            "SELECT * FROM blitz_tier_list WHERE role = ? AND pick_rate >= 0.02 ORDER BY win_rate DESC LIMIT 30"
        )
        .bind(mapped_role)
        .fetch_all(pool)
        .await?;
    }

    let min_pr = get_min_pick_rate_for_games(pool, target_elo, &mapped_role, 10.0).await;
    tiers.retain(|c| c.pick_rate.unwrap_or(0.0) >= min_pr);
    tiers.retain(|c| is_viable_for_role(&c.champion_id, &mapped_role) && !is_low_elo_trap(&c.champion_id, elo));
    
    let min_non_zero_target = get_min_non_zero_pick_rate(pool, target_elo, &mapped_role).await;
    let total_games_target = if min_non_zero_target > 0.0 { 1.0 / min_non_zero_target } else { 1000.0 };
    sort_by_bayesian_win_rate(&mut tiers, total_games_target);
    tiers.truncate(3);
    
    // Emergência de segurança absoluta (hardcoded)
    if tiers.is_empty() {
        let emergency_bans = match mapped_role {
            "JUNGLE" => vec!["MasterYi", "Amumu", "Rammus"],
            "TOP" => vec!["Garen", "Darius", "Malphite"],
            "MID" => vec!["Annie", "Lux", "Ahri"],
            "ADC" => vec!["MissFortune", "Ashe", "Jinx"],
            "SUPPORT" => vec!["Lux", "Blitzcrank", "Nautilus"],
            _ => vec!["Garen", "Annie"],
        };
        for b in emergency_bans {
            tiers.push(BlitzTier {
                id: 0, elo: elo.to_string(), role: mapped_role.to_string(), champion_id: b.to_string(),
                tier: Some("S".to_string()), win_rate: Some(52.0), pick_rate: Some(5.0), ban_rate: Some(5.0),
            });
        }
    }
    
    Ok(tiers)
}

/// Recomenda os 3 melhores campeões para contra-atacar (Counter Pick) a escolha do inimigo.
pub async fn get_counter_picks(
    pool: &Pool<Sqlite>, 
    enemy_champ_id: &str, 
    _role: &str,
    elo: Option<&str>
) -> Result<Vec<Matchup>, sqlx::Error> {
    let target_elo = elo.unwrap_or("GLOBAL").to_uppercase();
    
    let mut results = sqlx::query_as::<_, Matchup>(
        "SELECT * FROM matchups 
         WHERE opponent_id = ? AND difficulty <= 2 AND elo = ? 
         ORDER BY win_rate DESC LIMIT 20"
    )
    .bind(enemy_champ_id)
    .bind(&target_elo)
    .fetch_all(pool)
    .await?;

    if results.is_empty() && target_elo != "GLOBAL" {
        results = sqlx::query_as::<_, Matchup>(
            "SELECT * FROM matchups 
             WHERE opponent_id = ? AND difficulty <= 2 AND elo = 'GLOBAL' 
             ORDER BY win_rate DESC LIMIT 20"
        )
        .bind(enemy_champ_id)
        .fetch_all(pool)
        .await?;
    }

    if results.is_empty() {
        results = sqlx::query_as::<_, Matchup>(
            "SELECT * FROM matchups 
             WHERE opponent_id = ? AND difficulty <= 2 
             GROUP BY champion_id 
             ORDER BY win_rate DESC LIMIT 20"
        )
        .bind(enemy_champ_id)
        .fetch_all(pool)
        .await?;
    }

    sort_matchups_by_bayesian_win_rate(&mut results);
    results.truncate(3);

    Ok(results)
}

/// Recomenda os 3 melhores campeões para serem escolhidos às cegas (Blind Pick) na seleção.
/// 
/// **Para o estudante:**
/// Escolher às cegas exige campeões de alta consistência que não possuem counters óbvios e fáceis.
/// A lógica é similar à de bans, porém buscamos campeões S+ / S com alta taxa de vitória estável.
pub async fn get_blind_picks(pool: &Pool<Sqlite>, elo: &str, role: &str) -> Result<Vec<BlitzTier>, sqlx::Error> {
    let mapped_role = match role.to_lowercase().as_str() {
        "top" => "TOP",
        "jungle" => "JUNGLE",
        "middle" | "mid" => "MID",
        "bottom" | "adc" => "ADC",
        "utility" | "support" => "SUPPORT",
        _ => role,
    };

    let min_non_zero = get_min_non_zero_pick_rate(pool, elo, &mapped_role).await;
    let total_games = if min_non_zero > 0.0 { 1.0 / min_non_zero } else { 1000.0 };
    
    let target_elo = if total_games < 150.0 && elo != "SILVER" && elo != "GOLD" {
        println!("[Coach] Base de dados local para Elo {} é muito pequena ({:.0} jogos). Usando SILVER para blind pick seguro.", elo, total_games);
        "SILVER"
    } else {
        elo
    };

    println!("[Coach] Buscando blind picks para Elo: {}, Role: {} (Mapeado: {}) [Usando {} para dados estatísticos]", elo, role, mapped_role, target_elo);

    let mut picks = sqlx::query_as::<_, BlitzTier>(
        "SELECT * FROM blitz_tier_list WHERE elo = ? AND role = ? AND tier IN ('S+', 'S', 'OP') AND pick_rate >= 0.02 ORDER BY win_rate DESC LIMIT 30"
    )
    .bind(target_elo)
    .bind(mapped_role)
    .fetch_all(pool)
    .await?;

    if picks.is_empty() {
        picks = sqlx::query_as::<_, BlitzTier>(
            "SELECT * FROM blitz_tier_list WHERE elo = ? AND role = ? AND pick_rate >= 0.02 ORDER BY win_rate DESC LIMIT 30"
        )
        .bind(target_elo)
        .bind(mapped_role)
        .fetch_all(pool)
        .await?;
    }

    if picks.is_empty() && target_elo != "GOLD" {
        picks = sqlx::query_as::<_, BlitzTier>(
            "SELECT * FROM blitz_tier_list WHERE elo = 'GOLD' AND role = ? AND tier IN ('S', 'OP') AND pick_rate >= 0.02 ORDER BY win_rate DESC LIMIT 30"
        )
        .bind(mapped_role)
        .fetch_all(pool)
        .await?;
    }

    if picks.is_empty() {
        picks = sqlx::query_as::<_, BlitzTier>(
            "SELECT * FROM blitz_tier_list WHERE role = ? AND pick_rate >= 0.02 ORDER BY win_rate DESC LIMIT 30"
        )
        .bind(mapped_role)
        .fetch_all(pool)
        .await?;
    }

    let min_pr = get_min_pick_rate_for_games(pool, target_elo, &mapped_role, 10.0).await;
    picks.retain(|c| c.pick_rate.unwrap_or(0.0) >= min_pr);
    picks.retain(|c| is_viable_for_role(&c.champion_id, &mapped_role) && !is_low_elo_trap(&c.champion_id, elo));
    
    let min_non_zero_target = get_min_non_zero_pick_rate(pool, target_elo, &mapped_role).await;
    let total_games_target = if min_non_zero_target > 0.0 { 1.0 / min_non_zero_target } else { 1000.0 };
    sort_by_bayesian_win_rate(&mut picks, total_games_target);
    picks.truncate(3);

    if picks.is_empty() {
        let emergency_picks = match mapped_role {
            "JUNGLE" => vec!["MasterYi", "Amumu", "Rammus"],
            "TOP" => vec!["Garen", "Darius", "Malphite"],
            "MID" => vec!["Annie", "Lux", "Ahri"],
            "ADC" => vec!["MissFortune", "Ashe", "Jinx"],
            "SUPPORT" => vec!["Lux", "Blitzcrank", "Nautilus"],
            _ => vec!["Garen", "Annie"],
        };
        
        for p in emergency_picks {
            picks.push(BlitzTier {
                id: 0, elo: elo.to_string(), role: mapped_role.to_string(), champion_id: p.to_string(),
                tier: Some("S".to_string()), win_rate: Some(52.0), pick_rate: Some(5.0), ban_rate: None
            });
        }
    }

    Ok(picks)
}

/// Varre o histórico de partidas local do jogador e retorna os seus campeões mais jogados.
/// 
/// **Para o estudante (Agregador de Histórico de Jogos):**
/// 1. Coleta todas as partidas armazenadas no banco de dados local.
/// 2. As partidas contêm um documento JSON interno gigantesco (`data`). Nós fazemos o parse 
///    dessa string JSON dinamicamente.
/// 3. Buscamos a lista de participantes e identificamos a linha correspondente ao PUUID do jogador.
/// 4. Adicionamos a contagem em uma tabela chave-valor (`HashMap` do Rust) mapeando `Nome do Campeão -> Qtd Jogos`.
/// 5. Convertemos a tabela de contagem em um vetor, ordenamos do maior para o menor e retornamos os primeiros!
pub async fn get_most_played_champions(
    pool: &Pool<Sqlite>,
    puuid: &str,
    limit: usize,
) -> Vec<String> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT match_id, data FROM matches")
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    // Cria o dicionário chave-valor na memória
    let mut champ_counts = std::collections::HashMap::new();

    for (_match_id, data_str) in rows {
        if let Ok(match_val) = serde_json::from_str::<serde_json::Value>(&data_str) {
            // Varre o ponteiro do JSON usando navegação de propriedades do Serde JSON
            if let Some(participants) = match_val.pointer("/info/participants").and_then(|p| p.as_array()) {
                for p in participants {
                    if p.get("puuid").and_then(|v| v.as_str()) == Some(puuid) {
                        if let Some(champ) = p.get("championName").and_then(|v| v.as_str()) {
                            if champ != "Unknown" && !champ.is_empty() {
                                // Incrementa a contagem de forma elegante e segura usando a API Entry do Rust
                                *champ_counts.entry(champ.to_string()).or_insert(0) += 1;
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    // Ordenação do dicionário convertendo para vetor
    let mut champ_vec: Vec<(String, i32)> = champ_counts.into_iter().collect();
    champ_vec.sort_by(|a, b| b.1.cmp(&a.1));

    champ_vec.into_iter().take(limit).map(|(name, _)| name).collect()
}

// ── Dados de meta para coaching in-game ──────────────────────────────────────

/// Dados do banco usados pelo coach durante a partida.
pub struct ChampionMetaDb {
    /// Ordem de skills recomendada pelo meta (ex: ["Q","W","Q","E","Q","R",...]).
    pub skill_order: Vec<String>,
    /// Rota de limpeza recomendada para jungle (ex: "Blue > Gromp > Wolves").
    pub jungle_path: Option<String>,
    /// Tempo médio (ms) até o 1º item core. Usado para alertas de recall.
    pub first_item_time_ms: Option<i64>,
    /// Itens iniciais recomendados pelo meta.
    pub starting_items: Vec<i64>,
}

/// Resolve o ID DDragon de um campeão a partir do nome retornado pela LCA.
/// A LCA retorna "Miss Fortune" (com espaço), mas o DB armazena "MissFortune" (DDragon key).
/// Usa fallback triplo: id exato, name exato, name sem espaço/apostrofe.
pub async fn resolve_champion_db_id(pool: &Pool<Sqlite>, lca_name: &str) -> String {
    if lca_name.is_empty() { return String::new(); }
    let normalized = lca_name.to_lowercase().replace(' ', "").replace('\'', "");
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM champions
         WHERE LOWER(id) = LOWER(?)
            OR LOWER(name) = LOWER(?)
            OR LOWER(REPLACE(REPLACE(id, ' ', ''), '''', '')) = LOWER(?)
            OR LOWER(REPLACE(REPLACE(name, ' ', ''), '''', '')) = LOWER(?)
         LIMIT 1"
    )
    .bind(lca_name).bind(lca_name).bind(&normalized).bind(&normalized)
    .fetch_optional(pool).await.ok().flatten();
    row.map(|(id,)| id).unwrap_or_else(|| lca_name.to_string())
}

/// Carrega os dados de meta do banco para o campeão/role atual.
/// Chamado uma única vez por partida (cacheado no CoachState).
pub async fn get_champion_meta_from_db(
    pool: &Pool<Sqlite>,
    champion_id: &str,
    role: &str,
) -> Option<ChampionMetaDb> {
    // Resolve to DDragon ID before querying (LCA returns "Miss Fortune", DB has "MissFortune")
    let resolved = resolve_champion_db_id(pool, champion_id).await;
    let key = if resolved.is_empty() { champion_id } else { &resolved };

    type Row = (Option<String>, Option<String>, Option<i64>, Option<String>);
    // Skill order não muda por elo — usa o high elo mais alto que tiver dados.
    // Builds/items: prioridade CHALLENGER > GM > MASTER > DIAMOND.
    let row: Option<Row> = sqlx::query_as::<_, Row>(
        "SELECT skill_order_json, jungle_path, first_item_time_ms, starting_items_json
         FROM recommended_builds WHERE champion_id = ? AND role = ?
         ORDER BY CASE elo
           WHEN 'CHALLENGER'  THEN 1 WHEN 'GRANDMASTER' THEN 2
           WHEN 'MASTER'      THEN 3 WHEN 'DIAMOND'     THEN 4
           ELSE 5 END
         LIMIT 1"
    )
    .bind(key)
    .bind(role)
    .fetch_optional(pool)
    .await
    .ok()?;

    let (skill_json, jungle_path, first_item_ms, starting_json) = row?;

    let skill_order = skill_json
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default();

    let starting_items = starting_json
        .and_then(|s| serde_json::from_str::<Vec<i64>>(&s).ok())
        .unwrap_or_default();

    Some(ChampionMetaDb { skill_order, jungle_path, first_item_time_ms: first_item_ms, starting_items })
}

/// Retorna a dificuldade do matchup do campeão atual vs um oponente específico.
/// 1 = fácil (boa win_rate), 10 = difícil (counter).
/// Ponto de ward com coordenadas e frequência relativa.
#[derive(serde::Serialize, Debug, Clone)]
pub struct WardPoint {
    pub x: i64,
    pub y: i64,
    /// 1–5: prioridade (1 = mais frequente nos dados de Challenger/Diamond)
    pub priority: i32,
    /// "ward" (sentinela verde) ou "pink" (ward de controle rosa)
    pub ward_type: String,
}

/// Retorna os pontos de ward mais comuns para o campeão/role no momento atual.
/// Usa SEMPRE dados Diamond/Challenger — referência de alto nível para ensinar progressão.
/// Agrupa coordenadas em buckets de 700u para reduzir pontos redundantes.
pub async fn get_ward_suggestions(
    pool: &Pool<Sqlite>,
    champion_id: &str,
    role: &str,
    game_time_secs: f64,
) -> Vec<WardPoint> {
    let window_ms_min = ((game_time_secs - 180.0).max(0.0) * 1000.0) as i64;
    let window_ms_max = ((game_time_secs + 180.0) * 1000.0) as i64;

    type Row = (i64, i64, i64);

    // Top 2 stealth wards (YELLOW_TRINKET / SIGHT_WARD)
    let mut green = sqlx::query_as::<_, Row>(
        "SELECT (x_coord/700)*700+350 AS bx, (y_coord/700)*700+350 AS by, COUNT(*) AS cnt
         FROM ward_heatmaps
         WHERE champion_id=? AND role=? AND elo='HIGH_ELO'
           AND ward_type != 'CONTROL_WARD'
           AND (placed_at_ms IS NULL OR placed_at_ms BETWEEN ? AND ?)
         GROUP BY bx, by ORDER BY cnt DESC LIMIT 2"
    )
    .bind(champion_id).bind(role).bind(window_ms_min).bind(window_ms_max)
    .fetch_all(pool).await.unwrap_or_default();

    if green.is_empty() {
        green = sqlx::query_as::<_, Row>(
            "SELECT (x_coord/700)*700+350 AS bx, (y_coord/700)*700+350 AS by, COUNT(*) AS cnt
             FROM ward_heatmaps
             WHERE champion_id=? AND role=? AND elo='HIGH_ELO' AND ward_type != 'CONTROL_WARD'
             GROUP BY bx, by ORDER BY cnt DESC LIMIT 2"
        )
        .bind(champion_id).bind(role)
        .fetch_all(pool).await.unwrap_or_default();
    }

    // Top 1 pink ward (CONTROL_WARD)
    let mut pink = sqlx::query_as::<_, Row>(
        "SELECT (x_coord/700)*700+350 AS bx, (y_coord/700)*700+350 AS by, COUNT(*) AS cnt
         FROM ward_heatmaps
         WHERE champion_id=? AND role=? AND elo='HIGH_ELO'
           AND ward_type = 'CONTROL_WARD'
           AND (placed_at_ms IS NULL OR placed_at_ms BETWEEN ? AND ?)
         GROUP BY bx, by ORDER BY cnt DESC LIMIT 1"
    )
    .bind(champion_id).bind(role).bind(window_ms_min).bind(window_ms_max)
    .fetch_all(pool).await.unwrap_or_default();

    if pink.is_empty() {
        pink = sqlx::query_as::<_, Row>(
            "SELECT (x_coord/700)*700+350 AS bx, (y_coord/700)*700+350 AS by, COUNT(*) AS cnt
             FROM ward_heatmaps
             WHERE champion_id=? AND role=? AND elo='HIGH_ELO' AND ward_type = 'CONTROL_WARD'
             GROUP BY bx, by ORDER BY cnt DESC LIMIT 1"
        )
        .bind(champion_id).bind(role)
        .fetch_all(pool).await.unwrap_or_default();
    }

    let max_green = green.first().map(|(_, _, c)| *c).unwrap_or(1).max(1);
    let mut result: Vec<WardPoint> = green.into_iter().enumerate().map(|(_i, (x, y, cnt))| {
        let priority = (5 - (cnt * 4 / max_green).min(4)) as i32;
        WardPoint { x, y, priority: priority.max(1).min(5), ward_type: "ward".to_string() }
    }).collect();

    for (x, y, _) in pink {
        result.push(WardPoint { x, y, priority: 1, ward_type: "pink".to_string() });
    }

    result
}

/// Retorna pontos de ward relevantes para um objetivo neutro específico.
/// Filtra por:
///   - janela de tempo ±90s ao redor do spawn do objetivo
///   - bounding box de coordenadas em torno da área do objetivo
/// Fallback: se não houver dados na área, usa a query geral sem filtro geográfico.
/// Wards pré-objetivo: sempre Diamond/Challenger — ensina posicionamento de alto nível.
pub async fn get_ward_suggestions_for_objective(
    pool: &Pool<Sqlite>,
    champion_id: &str,
    role: &str,
    objective_spawn_secs: f64,
    x_min: i64, x_max: i64,
    y_min: i64, y_max: i64,
) -> Vec<WardPoint> {
    let window_ms_min = ((objective_spawn_secs - 90.0).max(0.0) * 1000.0) as i64;
    let window_ms_max = ((objective_spawn_secs + 90.0) * 1000.0) as i64;

    type Row = (i64, i64, i64);

    // Top 2 stealth wards na área do objetivo
    let mut green = sqlx::query_as::<_, Row>(
        "SELECT (x_coord/700)*700+350 AS bx, (y_coord/700)*700+350 AS by, COUNT(*) AS cnt
         FROM ward_heatmaps
         WHERE champion_id=? AND role=? AND elo='HIGH_ELO'
           AND ward_type != 'CONTROL_WARD'
           AND (placed_at_ms IS NULL OR placed_at_ms BETWEEN ? AND ?)
           AND x_coord BETWEEN ? AND ? AND y_coord BETWEEN ? AND ?
         GROUP BY bx, by ORDER BY cnt DESC LIMIT 2"
    )
    .bind(champion_id).bind(role)
    .bind(window_ms_min).bind(window_ms_max)
    .bind(x_min).bind(x_max).bind(y_min).bind(y_max)
    .fetch_all(pool).await.unwrap_or_default();

    if green.is_empty() {
        green = sqlx::query_as::<_, Row>(
            "SELECT (x_coord/700)*700+350 AS bx, (y_coord/700)*700+350 AS by, COUNT(*) AS cnt
             FROM ward_heatmaps
             WHERE champion_id=? AND role=? AND elo='HIGH_ELO' AND ward_type != 'CONTROL_WARD'
               AND (placed_at_ms IS NULL OR placed_at_ms BETWEEN ? AND ?)
             GROUP BY bx, by ORDER BY cnt DESC LIMIT 2"
        )
        .bind(champion_id).bind(role)
        .bind(window_ms_min).bind(window_ms_max)
        .fetch_all(pool).await.unwrap_or_default();
    }

    // Top 1 pink na área do objetivo
    let mut pink = sqlx::query_as::<_, Row>(
        "SELECT (x_coord/700)*700+350 AS bx, (y_coord/700)*700+350 AS by, COUNT(*) AS cnt
         FROM ward_heatmaps
         WHERE champion_id=? AND role=? AND elo='HIGH_ELO'
           AND ward_type = 'CONTROL_WARD'
           AND (placed_at_ms IS NULL OR placed_at_ms BETWEEN ? AND ?)
           AND x_coord BETWEEN ? AND ? AND y_coord BETWEEN ? AND ?
         GROUP BY bx, by ORDER BY cnt DESC LIMIT 1"
    )
    .bind(champion_id).bind(role)
    .bind(window_ms_min).bind(window_ms_max)
    .bind(x_min).bind(x_max).bind(y_min).bind(y_max)
    .fetch_all(pool).await.unwrap_or_default();

    if pink.is_empty() {
        pink = sqlx::query_as::<_, Row>(
            "SELECT (x_coord/700)*700+350 AS bx, (y_coord/700)*700+350 AS by, COUNT(*) AS cnt
             FROM ward_heatmaps
             WHERE champion_id=? AND role=? AND elo='HIGH_ELO' AND ward_type = 'CONTROL_WARD'
               AND (placed_at_ms IS NULL OR placed_at_ms BETWEEN ? AND ?)
             GROUP BY bx, by ORDER BY cnt DESC LIMIT 1"
        )
        .bind(champion_id).bind(role)
        .bind(window_ms_min).bind(window_ms_max)
        .fetch_all(pool).await.unwrap_or_default();
    }

    let max_green = green.first().map(|(_, _, c)| *c).unwrap_or(1).max(1);
    let mut result: Vec<WardPoint> = green.into_iter().enumerate().map(|(i, (x, y, cnt))| {
        let priority = (5 - (cnt * 4 / max_green).min(4)) as i32;
        WardPoint { x, y, priority: (priority.max(1) + i as i32 / 3).min(5), ward_type: "ward".to_string() }
    }).collect();

    for (x, y, _) in pink {
        result.push(WardPoint { x, y, priority: 1, ward_type: "pink".to_string() });
    }

    result
}

pub async fn get_matchup_difficulty(
    pool: &Pool<Sqlite>,
    champion_id: &str,
    opponent_id: &str,
) -> Option<i32> {
    let row: Option<(i32,)> = sqlx::query_as::<_, (i32,)>(
        "SELECT COALESCE(difficulty, 5) FROM matchups
         WHERE champion_id = ? AND opponent_id = ?
         ORDER BY games_count DESC LIMIT 1"
    )
    .bind(champion_id)
    .bind(opponent_id)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(|(d,)| d)
}
