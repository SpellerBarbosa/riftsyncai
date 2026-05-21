/*
===============================================================================
                       SPELL COACH IA - HEURÍSTICAS E MATEMÁTICA
===============================================================================
Este submódulo abriga a inteligência lógica "pura" (heurísticas) do Coach.
Aqui não acessamos APIs externas, apenas computamos dados estatísticos do banco,
filtramos dados inadequados e ordenamos os campeões de forma matematicamente justa.

Para um estudante de programação:
* O que é uma Heurística? É uma regra prática ou atalho mental que nos ajuda a
  tomar decisões rápidas e boas sem precisar processar infinitas possibilidades.
* O que é o Teorema de Bayes? É uma fórmula de probabilidade que usamos aqui
  para evitar o "Erro de Amostragem Pequena". Exemplo: Se um campeão jogou 1 jogo 
  e venceu, ele tem 100% de win rate. Ele é o melhor do jogo? Não! A suavização 
  Bayesiana puxa essa taxa de vitória em direção a 50% até que ele jogue partidas 
  suficientes para provar sua verdadeira força.
===============================================================================
*/

use sqlx::{Pool, Sqlite};

/// Verifica se um determinado campeão é viável para ser jogado em uma função (role) específica.
/// 
/// **Para o estudante:**
/// Usamos a estrutura `matches!(variavel, padrao1 | padrao2)` do Rust. É um atalho extremamente 
/// rápido de leitura e performance para verificar se uma string se iguala a qualquer um dos itens listados.
/// Isso é muito mais performático que criar um array e buscar nele em tempo de execução.
pub fn is_viable_for_role(champion_id: &str, role: &str) -> bool {
    let champ_lower = champion_id.to_lowercase();
    let role_upper = role.to_uppercase();
    
    // Mapeamento estrito baseado no meta-game atual do League of Legends
    match role_upper.as_str() {
        "TOP" => {
            matches!(champ_lower.as_str(),
                "aatrox" | "akali" | "akshan" | "aurora" | "camille" | "cassiopeia" | "chogath" | "darius" | "drmundo" |
                "fiora" | "gangplank" | "garen" | "gnar" | "gragas" | "gwen" | "hecarim" | "heimerdinger" | "illaoi" |
                "irelia" | "jax" | "jayce" | "ksante" | "k'sante" | "kayle" | "kennen" | "kled" | "malphite" | "maokai" |
                "mordekaiser" | "nasus" | "olaf" | "ornn" | "pantheon" | "poppy" | "quinn" | "rammus" | "riven" |
                "rumble" | "ryze" | "sejuani" | "sett" | "shen" | "singed" | "sion" | "smolder" | "tahmkench" | "teemo" |
                "trundle" | "tryndamere" | "udyr" | "urgot" | "vayne" | "vladimir" | "volibear" | "warwick" | "wukong" |
                "yasuo" | "yone" | "yorick" | "zac"
            )
        }
        "JUNGLE" => {
            matches!(champ_lower.as_str(),
                "amumu" | "belveth" | "briar" | "brand" | "camille" | "karthus" | "chogath" | "diana" | "ekko" |
                "elise" | "evelynn" | "fiddlesticks" | "graves" | "hecarim" | "ivern" | "jarvaniv" | "jax" | "kayn" |
                "khazix" | "kindred" | "leesin" | "lillia" | "masteryi" | "maokai" | "naafiri" | "nidalee" | "nocturne" |
                "nunu" | "olaf" | "pantheon" | "poppy" | "rammus" | "reksai" | "rengar" | "riven" | "sejuani" |
                "shaco" | "shyvana" | "skarner" | "taliyah" | "talon" | "trundle" | "udyr" | "vi" | "viego" |
                "volibear" | "warwick" | "wukong" | "xinzhao" | "zac" | "zyra"
            )
        }
        "MID" | "MIDDLE" => {
            matches!(champ_lower.as_str(),
                "aatrox" | "ahri" | "akali" | "akshan" | "anivia" | "annie" | "aurelionsol" | "azir" | "brand" |
                "cassiopeia" | "corki" | "diana" | "ekko" | "fizz" | "galio" | "gragas" | "heimerdinger" | "hwei" |
                "irelia" | "janna" | "kassadin" | "katarina" | "kayle" | "leblanc" | "lissandra" | "lucian" | "lux" |
                "malzahar" | "naafiri" | "neeko" | "orianna" | "pantheon" | "qiyana" | "ryze" | "smolder" | "swain" |
                "syndra" | "taliyah" | "talon" | "tristana" | "twistedfate" | "veigar" | "velkoz" | "vex" | "viktor" |
                "vladimir" | "xerath" | "yasuo" | "yone" | "zed" | "ziggs" | "zoe"
            )
        }
        "ADC" | "BOTTOM" => {
            matches!(champ_lower.as_str(),
                "aphelios" | "ashe" | "caitlyn" | "draven" | "ezreal" | "jhin" | "jinx" | "kaisa" | "kalista" |
                "kogmaw" | "lucian" | "missfortune" | "nilah" | "samira" | "senna" | "smolder" | "sivir" |
                "tristana" | "varus" | "vayne" | "xayah" | "yasuo" | "ziggs"
            )
        }
        "SUPPORT" | "UTILITY" => {
            matches!(champ_lower.as_str(),
                "alistar" | "amumu" | "anivia" | "ashe" | "bard" | "blitzcrank" | "brand" | "braum" | "camille" |
                "fiddlesticks" | "galio" | "gragas" | "heimerdinger" | "hwei" | "janna" | "karma" | "leona" |
                "lulu" | "lux" | "maokai" | "milio" | "morgana" | "nami" | "nautilus" | "neeko" | "pantheon" |
                "poppy" | "pyke" | "rakan" | "rell" | "renata" | "senna" | "seraphine" | "shaco" | "shen" |
                "sona" | "soraka" | "swain" | "taric" | "thresh" | "velkoz" | "xerath" | "yuumi" | "zilean" | "zyra"
            )
        }
        _ => true, // Se for uma role desconhecida, assume viabilidade por precaução
    }
}

/// Identifica se um campeão é considerado uma "Armadilha de Elo Baixo" (Low Elo Trap).
/// 
/// **Para o estudante:**
/// Campeões complexos mecanicamente como Lee Sin, Yasuo ou K'Sante exigem tanta habilidade motora 
/// que jogadores nos elos Ferro, Bronze e Prata costumam ter taxas de vitória terríveis com eles, 
/// mesmo achando-os divertidos. O Coach inteligente avisa o estudante para evitá-los se quiser subir de elo.
pub fn is_low_elo_trap(champion_id: &str, elo: &str) -> bool {
    let elo_upper = elo.to_uppercase();
    if elo_upper == "IRON" || elo_upper == "BRONZE" || elo_upper == "SILVER" {
        let champ_lower = champion_id.to_lowercase();
        matches!(champ_lower.as_str(),
            "leesin" | "yasuo" | "yone" | "riven" | "kalista" | "azir" | "gangplank" | "zed" | 
            "nidalee" | "ksante" | "k'sante" | "hwei" | "aphelios" | "vayne" | "fiora" | 
            "irelia" | "katarina" | "elise" | "bard" | "leblanc" | "ryze" | "viego"
        )
    } else {
        false
    }
}

/// Busca dinamicamente a menor taxa de escolha (pick rate) aceitável baseada na quantidade de jogos.
/// 
/// **Para o estudante:**
/// Usamos consultas SQL assíncronas (`.await`). O SQL do SQLite busca a menor taxa de escolha 
/// maior do que zero no banco. Se a base de dados do elo for muito pequena, relaxamos os filtros 
/// para não deixar o usuário sem recomendações.
pub async fn get_min_pick_rate_for_games(
    pool: &Pool<Sqlite>,
    elo: &str,
    role: &str,
    min_games: f64
) -> f64 {
    let default_min = 0.02; // 2% de pick rate mínimo padrão
    
    // Consulta SQL segura usando binds (?) para evitar ataques de SQL Injection
    let min_pr: Option<f64> = sqlx::query_scalar(
        "SELECT MIN(pick_rate) FROM blitz_tier_list WHERE elo = ? AND role = ? AND pick_rate > 0"
    )
    .bind(elo)
    .bind(role)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    if let Some(min_val) = min_pr {
        let total_games = if min_val > 0.0 { 1.0 / min_val } else { 1000.0 };
        if total_games < 300.0 {
            // Se a base de dados de amostragem for muito pequena, não pune severamente o pick rate
            default_min
        } else {
            let calculated = min_games * min_val;
            if calculated > 0.08 {
                0.08
            } else if calculated < default_min {
                default_min
            } else {
                calculated
            }
        }
    } else {
        default_min
    }
}

/// Helper para obter a menor taxa de escolha válida registrada para aquela combinação no banco.
pub async fn get_min_non_zero_pick_rate(
    pool: &Pool<Sqlite>,
    elo: &str,
    role: &str
) -> f64 {
    let min_pr: Option<f64> = sqlx::query_scalar(
        "SELECT MIN(pick_rate) FROM blitz_tier_list WHERE elo = ? AND role = ? AND pick_rate > 0"
    )
    .bind(elo)
    .bind(role)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    
    min_pr.unwrap_or(0.001)
}

/// Implementa a Suavização Bayesiana (Bayesian Win Rate Smoothing) para a lista de campeões BlitzTier.
/// 
/// **Para o estudante:**
/// A ordenação padrão por taxa de vitória seca (win_rate) é perigosa porque ignora o tamanho da amostragem.
/// 
/// **Fórmula Utilizada:**
/// `Bayesian = (Games * WinRate + K * Prior) / (Games + K)`
/// * `Games` = Quantidade estimada de partidas que o campeão teve.
/// * `WinRate` = Taxa de vitória nominal do campeão.
/// * `K` = Peso da nossa incerteza (usamos 15 partidas como padrão de confiança).
/// * `Prior` = Média global estável (50% ou 0.50).
/// 
/// Se um campeão tem apenas 2 partidas e 100% de vitórias, a fórmula reduz a sua pontuação para próximo de 55%.
/// Se ele tem 1.000 partidas e 54% de vitórias, a pontuação permanece muito próxima de 54%, provando confiabilidade.
pub fn sort_by_bayesian_win_rate(
    picks: &mut Vec<crate::db::models::BlitzTier>,
    total_games: f64
) {
    picks.sort_by(|a, b| {
        let games_a = a.pick_rate.unwrap_or(0.0) * total_games;
        let wr_a = a.win_rate.unwrap_or(0.50);
        let bayesian_a = (games_a * wr_a + 15.0 * 0.50) / (games_a + 15.0);

        let games_b = b.pick_rate.unwrap_or(0.0) * total_games;
        let wr_b = b.win_rate.unwrap_or(0.50);
        let bayesian_b = (games_b * wr_b + 15.0 * 0.50) / (games_b + 15.0);

        // Ordenamos do maior para o menor (descendente)
        bayesian_b.partial_cmp(&bayesian_a).unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Ordena confrontos diretos (matchups) usando a Suavização Bayesiana.
/// 
/// **Para o estudante:**
/// Funciona de forma análoga à função acima, porém lê a contagem exata de jogos (`games_count`)
/// que já foi salva diretamente no nosso banco de dados de matchups.
pub fn sort_matchups_by_bayesian_win_rate(
    matchups: &mut Vec<crate::db::models::Matchup>
) {
    matchups.sort_by(|a, b| {
        let games_a = a.games_count.unwrap_or(0) as f64;
        let wr_a = a.win_rate.unwrap_or(0.50);
        let bayesian_a = (games_a * wr_a + 10.0 * 0.50) / (games_a + 10.0);

        let games_b = b.games_count.unwrap_or(0) as f64;
        let wr_b = b.win_rate.unwrap_or(0.50);
        let bayesian_b = (games_b * wr_b + 10.0 * 0.50) / (games_b + 10.0);

        bayesian_b.partial_cmp(&bayesian_a).unwrap_or(std::cmp::Ordering::Equal)
    });
}
