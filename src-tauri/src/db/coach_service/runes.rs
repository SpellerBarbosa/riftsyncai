/*
===============================================================================
                       SPELL COACH IA - RECOMENDADOR DE RUNAS
===============================================================================
Este submódulo é responsável por analisar a melhor combinação de runas e feitiços 
de invocador para o campeão do jogador na rota selecionada.

Para um estudante de programação:
* O que é um IPC Command (Inter-Process Communication)? 
  O Tauri roda em dois processos principais: o Backend (Rust) e o Frontend (HTML/CSS/JS).
  Para o Frontend conversar com o Rust, usamos comandos anotados com `#[tauri::command]`.
  Isso gera uma ponte de comunicação assíncrona automática para o JavaScript invocar!
* O que é o padrão de Injeção de Dependências?
  Na função `get_rune_overlay_data_command`, recebemos um argumento `state: State<DbState>`. 
  Não somos nós que instanciamos esse banco de dados na chamada da função. O próprio framework 
  Tauri "injeta" a conexão global pré-configurada para que possamos usá-la!
===============================================================================
*/

use sqlx::{Pool, Sqlite};
use tauri::State;

/// Estrutura de dados que descreve as runas recomendadas que serão exibidas na tela do usuário.
/// 
/// **Para o estudante:**
/// Usamos os atributos `#[derive(serde::Serialize, serde::Deserialize)]`.
/// Como o Rust e o JavaScript possuem sistemas de tipos totalmente diferentes, 
/// a biblioteca `serde` serializa a nossa Struct do Rust em uma string JSON que 
/// o JavaScript consegue entender e renderizar nativamente!
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RuneOverlayData {
    pub champion_id: String,
    pub champion_name: String,
    pub role: String,
    pub elo: String,
    pub primary_tree_id: i64,
    pub primary_tree_name: String,
    pub keystone_id: i64,
    pub keystone_name: String,
    pub keystone_desc: String,
    pub secondary_tree_id: i64,
    pub secondary_tree_name: String,
    pub runes: Vec<i64>,
    pub shards: Vec<i64>,
    pub explanation: String,
    pub summoner_spells: Vec<String>,
}

/// Helper didático: Mapeia regras clássicas do LoL para sugerir os melhores Feitiços de Invocador.
/// 
/// **Para o estudante:**
/// O caçador (JUNGLE) SEMPRE precisa de Smite (Golpear). 
/// Algumas exceções divertidas de campeões também são mapeadas aqui (ex: Hecarim usa Fantasma/Smite).
fn get_recommended_summoner_spells(role: &str, champ_name: &str) -> Vec<String> {
    let mut spells = vec!["Flash".to_string()];
    let champ_lower = champ_name.to_lowercase();
    
    // Tratamento de exceções específicas do jogo
    if champ_lower == "shaco" && role == "JUNGLE" {
        return vec!["Ignite".to_string(), "Smite".to_string()];
    }
    
    if matches!(champ_lower.as_str(), "hecarim" | "darius" | "olaf" | "singed" | "vladimir" | "nasus" | "tryndamere") {
        spells = vec!["Ghost".to_string()];
        if champ_lower == "hecarim" && role == "JUNGLE" {
            spells.push("Smite".to_string());
            return spells;
        }
    }

    // Regra geral baseada na rota (role) de atuação do campeão
    match role {
        "JUNGLE" => spells.push("Smite".to_string()),
        "TOP" => {
            if matches!(champ_lower.as_str(), "darius" | "garen" | "renekton" | "riven" | "pantheon" | "sett" | "olaf" | "tryndamere") {
                spells.push("Ignite".to_string());
            } else {
                spells.push("Teleport".to_string());
            }
        },
        "MID" => {
            if matches!(champ_lower.as_str(), "zed" | "talon" | "katarina" | "fizz" | "sylas" | "qiyana" | "akali" | "leblanc" | "naafiri" | "yasuo" | "yone") {
                spells.push("Ignite".to_string());
            } else {
                spells.push("Teleport".to_string());
            }
        },
        "ADC" => {
            if matches!(champ_lower.as_str(), "samira" | "nilah") {
                spells.push("Exhaust".to_string());
            } else if matches!(champ_lower.as_str(), "zeri" | "jinx" | "sivir" | "ashe" | "ghost") {
                spells.push("Ghost".to_string());
            } else {
                spells.push("Heal".to_string());
            }
        },
        "SUPPORT" => {
            if matches!(champ_lower.as_str(), "leona" | "nautilus" | "pyke" | "blitzcrank" | "thresh" | "rell" | "pantheon" | "alistar" | "braum") {
                spells.push("Ignite".to_string());
            } else {
                spells.push("Exhaust".to_string());
            }
        },
        _ => spells.push("Ignite".to_string()),
    }
    
    spells
}

/// Traduz as IDs das árvores de runas oficiais da Riot Games para seus nomes em Português.
fn get_tree_name(id: i64) -> &'static str {
    match id {
        8000 => "Precisão",
        8100 => "Dominação",
        8200 => "Feitiçaria",
        8300 => "Inspiração",
        8400 => "Determinação",
        _ => "Adaptativa",
    }
}

/// Traduz IDs das Runas Principais (Keystones) para seus nomes e descrições táticas.
fn get_keystone_details(id: i64) -> (&'static str, &'static str) {
    match id {
        8005 => ("Pressione o Ataque", "Melhora trocas rápidas e amplifica o dano no alvo."),
        8008 => ("Ritmo Fatal", "Concede velocidade de ataque cumulativa ao bater."),
        8021 => ("Agilidade nos Pés", "Garante cura e velocidade de movimento ao atacar."),
        8010 => ("Conquistador", "Concede força adaptativa e cura em lutas longas."),
        8112 => ("Eletrocutar", "Causa dano explosivo adaptativo ao acertar 3 ataques/habs."),
        8124 => ("Predador", "Concede velocidade de movimento extrema ativa nas boots."),
        8128 => ("Colheita Sombria", "Causa dano adaptativo a alvos com vida baixa e escala."),
        9923 => ("Chuva de Lâminas", "Velocidade de ataque massiva para os 3 primeiros ataques."),
        8214 => ("Invocação: Aery", "Envia Aery para causar dano extra ou proteger aliados."),
        8229 => ("Cometa Arcano", "Dispara um cometa que causa dano em área ao acertar habilidades."),
        8230 => ("Ímpeto Gradual", "Velocidade e resistência a lentidão após 3 ataques/habs."),
        8437 => ("Aperto Morto-Vivo", "Causa dano mágico, cura e aumenta vida máxima permanentemente."),
        8439 => ("Pós-choque", "Aumenta defesas após imobilizar e explode em dano."),
        8465 => ("Guardião", "Concede escudo a si e a um aliado próximo ao sofrer dano."),
        8351 => ("Aprimoramento Glacial", "Cria caminhos de lentidão extrema ao imobilizar inimigos."),
        8360 => ("Livro de Feitiços", "Permite trocar seus feitiços de invocador ativos."),
        8369 => ("Primeiro Ataque", "Causa dano extra inicial e concede ouro adaptativo."),
        _ => ("Runa Adaptativa", "Runa recomendada para otimização da rota."),
    }
}

/// Cria um parágrafo explicativo e tático em português com base na categoria e runa do campeão.
/// 
/// **Para o estudante:**
/// Isso demonstra a criação de uma inteligência geradora de explicações estáticas robustas 
/// baseada em condicionais limpas. O código analisa as categorias do campeão (tags como mage, tank, etc.)
/// e cria uma narrativa amigável para guiar o usuário final na tela do aplicativo.
fn generate_coaching_explanation(champ_name: &str, tags: &str, keystone_name: &str, primary_tree: &str, secondary_tree: &str) -> String {
    let t = tags.to_lowercase();
    let is_mage = t.contains("mage");
    let is_marksman = t.contains("marksman");
    let is_tank = t.contains("tank");
    let is_assassin = t.contains("assassin");
    let is_fighter = t.contains("fighter");
    let is_support = t.contains("support");

    let mut explanation = format!(
        "A árvore de **{}** (com **{}**) combinada com **{}** é a melhor opção estatística para o **{}**.",
        primary_tree, keystone_name, secondary_tree, champ_name
    );

    if is_mage {
        if keystone_name == "Cometa Arcano" {
            explanation = format!(
                "O **Cometa Arcano** é excelente para o **{}**. Ele maximiza o seu poke a longa distância e controle de rota, punindo o adversário toda vez que você acertar suas habilidades principais. A árvore secundária de **{}** oferece sustentação de mana ou aceleração para manter a pressão.",
                champ_name, secondary_tree
            );
        } else if keystone_name == "Eletrocutar" {
            explanation = format!(
                "O **Eletrocutar** potencializa o burst e o potencial de abate instantâneo do **{}**. Ideal para explodir alvos frágeis no meio do jogo. Combine com a utilidade e aceleração da árvore de **{}** para rotacionar com maestria.",
                champ_name, secondary_tree
            );
        } else if keystone_name == "Primeiro Ataque" {
            explanation = format!(
                "O **Primeiro Ataque** acelera a geração de ouro do **{}**, permitindo fechar seus itens core muito antes do tempo normal. Essencial para atingir picos de poder avassaladores e dominar o mapa através da vantagem econômica.",
                champ_name
            );
        }
    } else if is_marksman {
        if keystone_name == "Ritmo Fatal" || keystone_name == "Conquistador" {
            explanation = format!(
                "Para o **{}**, o **{}** é indispensável nas lutas longas. Ele aumenta o seu escalamento de dano por segundo (DPS) contínuo e garante que você consiga derreter a linha de frente inimiga com facilidade. A secundária de **{}** oferece proteção adicional ou mobilidade.",
                champ_name, keystone_name, secondary_tree
            );
        } else if keystone_name == "Pressione o Ataque" {
            explanation = format!(
                "O **Pressione o Ataque** garante trocas curtas explosivas e foca o dano amplificado no alvo prioritário. Perfeito para impor pressão de abate na rota do bot com o **{}**.",
                champ_name
            );
        } else if keystone_name == "Agilidade nos Pés" {
            explanation = format!(
                "A **Agilidade nos Pés** garante a sustentação de vida necessária para o **{}** sobreviver a rotas de poke difíceis, além de conceder mobilidade essencial para desviar de skillshots importantes.",
                champ_name
            );
        }
    } else if is_tank {
        if keystone_name == "Aperto Morto-Vivo" {
            explanation = format!(
                "O **Aperto Morto-Vivo** é o melhor recurso para o **{}** no topo. Ele melhora o seu dano de troca corpo a corpo, cura sua vida e escala sua vida máxima permanentemente a cada acerto. A secundária de **{}** concede tenacidade ou velocidade de ataque.",
                champ_name, secondary_tree
            );
        } else if keystone_name == "Pós-choque" {
            explanation = format!(
                "O **Pós-choque** transforma o **{}** em uma parede indestrutível logo após aplicar controles de grupo (CC). Excelente para suportes ou caçadores tanque iniciarem lutas com segurança.",
                champ_name
            );
        }
    } else if is_fighter {
        if keystone_name == "Conquistador" {
            explanation = format!(
                "Como lutador de duelos longos, o **Conquistador** é perfeito para o **{}**. Ele amplifica sua força física cumulativa nas trocas e concede cura integrada vital na linha de frente. A árvore secundária de **{}** garante sustentação na rota.",
                champ_name, secondary_tree
            );
        } else if keystone_name == "Aperto Morto-Vivo" {
            explanation = format!(
                "Para o **{}**, o **Aperto Morto-Vivo** permite vencer trocas curtas desgastando o oponente com segurança na rota, enquanto acumula vida máxima para as fases posteriores do jogo.",
                champ_name
            );
        }
    } else if is_assassin {
        explanation = format!(
            "O foco do **{}** é a eliminação rápida de alvos. **{}** potencializa seu dano explosivo e letalidade inicial, enquanto a árvore secundária de **{}** fornece velocidade para flanquear as rotas laterais.",
            champ_name, keystone_name, secondary_tree
        );
    } else if is_support {
        if keystone_name == "Invocação: Aery" || keystone_name == "Guardião" {
            explanation = format!(
                "Como suporte utilitário, **{}** maximiza a proteção e escudos concedidos ao seu atirador pelo **{}**, garantindo que sua dupla sobreviva a trocas agressivas na rota.",
                keystone_name, champ_name
            );
        }
    }

    explanation
}

/// Comando Tauri oficial invocado pelo Frontend Nuxt (Vue) para obter as runas na tela.
/// 
/// **Para o estudante:**
/// O Tauri gerencia a injeção do banco (`state`). Esta função simplesmente extrai o banco
/// de dados encapsulado em `DbState` e repassa para a função interna de processamento.
/// Isso separa a infraestrutura do framework (Tauri) da lógica de negócios real da nossa aplicação!
#[tauri::command]
pub async fn get_rune_overlay_data_command(
    state: State<'_, crate::db::DbState>,
    champ_id: String,
    role: Option<String>,
    elo: Option<String>,
) -> Result<RuneOverlayData, String> {
    let pool = &state.0;
    get_rune_overlay_data_command_internal(pool, champ_id, role, elo).await
}

/// Função de processamento interno para obter runas (livre de dependência do framework Tauri).
/// 
/// **Para o estudante:**
/// Repare no padrão de **Cascata de Busca (Fallback Cascade)**:
/// 1. Tentamos obter as runas específicas do Blitz.gg no Elo e Role informados.
/// 2. Se falhar, tentamos obter a build genérica recomendada para aquele campeão e role.
/// 3. Se falhar, pegamos qualquer build inicial de fallback registrada para o campeão.
/// Isso garante robustez extrema (o aplicativo nunca quebra ou exibe uma tela preta se faltarem dados).
pub async fn get_rune_overlay_data_command_internal(
    pool: &Pool<Sqlite>,
    champ_id: String,
    role: Option<String>,
    elo: Option<String>,
) -> Result<RuneOverlayData, String> {

    // Resolvendo o ID real e tags do campeão
    let (resolved_id, champ_name, champ_tags): (String, String, String) = sqlx::query_as(
        "SELECT id, name, tags FROM champions WHERE LOWER(id) = LOWER(?) OR LOWER(name) = LOWER(?)"
    )
    .bind(&champ_id)
    .bind(&champ_id)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|_| (champ_id.clone(), champ_id.clone(), "[]".to_string()));

    let resolved_role = role.unwrap_or_else(|| "MID".to_string()).to_uppercase();
    let resolved_elo = elo.unwrap_or_else(|| "GLOBAL".to_string()).to_uppercase();

    // --- CASCATA DE CONSULTAS ---
    // Recomendações vêm do banco local. Prioridade: elo mais alto com dados.
    // Skill order não muda por elo — qualquer high elo serve.
    // Wards e builds: sempre do elo mais alto disponível.

    let mut spells_str: Option<String> = None;

    // Tentativa 1: recommended_builds — role exata, prioridade high elo
    let mut runes_str: Option<String> = {
        let row: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT runes_json, summoner_spells_json FROM recommended_builds
             WHERE champion_id = ? AND role = ?
               AND runes_json IS NOT NULL AND runes_json != ''
             ORDER BY CASE elo
               WHEN 'CHALLENGER'  THEN 1 WHEN 'GRANDMASTER' THEN 2
               WHEN 'MASTER'      THEN 3 WHEN 'DIAMOND'     THEN 4
               ELSE 5 END
             LIMIT 1"
        )
        .bind(&resolved_id)
        .bind(&resolved_role)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
        if let Some((r, s)) = row { spells_str = s; Some(r) } else { None }
    };

    // Tentativa 2: recommended_builds — qualquer role, prioridade high elo
    if runes_str.is_none() {
        let row: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT runes_json, summoner_spells_json FROM recommended_builds
             WHERE champion_id = ?
               AND runes_json IS NOT NULL AND runes_json != ''
             ORDER BY CASE elo
               WHEN 'CHALLENGER'  THEN 1 WHEN 'GRANDMASTER' THEN 2
               WHEN 'MASTER'      THEN 3 WHEN 'DIAMOND'     THEN 4
               ELSE 5 END
             LIMIT 1"
        )
        .bind(&resolved_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
        if let Some((r, s)) = row { spells_str = s; runes_str = Some(r); }
    }

    // Tentativa 3: blitz_builds — fallback com dados parciais (keystone + árvore)
    if runes_str.is_none() {
        runes_str = sqlx::query_scalar(
            "SELECT runes_json FROM blitz_builds
             WHERE champion_id = ? AND role = ?
               AND runes_json IS NOT NULL AND runes_json != ''
             ORDER BY CASE elo
               WHEN 'CHALLENGER'  THEN 1 WHEN 'GRANDMASTER' THEN 2
               WHEN 'MASTER'      THEN 3 WHEN 'DIAMOND'     THEN 4
               ELSE 5 END
             LIMIT 1"
        )
        .bind(&resolved_id)
        .bind(&resolved_role)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    }

    // Tentativa 4: blitz_builds qualquer role
    if runes_str.is_none() {
        runes_str = sqlx::query_scalar(
            "SELECT runes_json FROM blitz_builds
             WHERE champion_id = ? AND runes_json IS NOT NULL AND runes_json != ''
             ORDER BY CASE elo
               WHEN 'CHALLENGER'  THEN 1 WHEN 'GRANDMASTER' THEN 2
               WHEN 'MASTER'      THEN 3 WHEN 'DIAMOND'     THEN 4
               ELSE 5 END
             LIMIT 1"
        )
        .bind(&resolved_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    }

    let mut primary_tree_id = 8000;
    let mut secondary_tree_id = 8100;
    let mut runes = vec![];
    let mut shards = vec![];

    // Se encontramos dados válidos, fazemos o parsing da String JSON usando a biblioteca 'serde_json'
    if let Some(json_str) = runes_str {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
            primary_tree_id = parsed["primary_tree"].as_i64().unwrap_or(8000);
            secondary_tree_id = parsed["secondary_tree"].as_i64().unwrap_or(0);
            if let Some(arr) = parsed["runes"].as_array() {
                runes = arr.iter().map(|v| v.as_i64().unwrap_or(0)).collect();
            }
            if let Some(arr) = parsed["shards"].as_array() {
                shards = arr.iter().map(|v| v.as_i64().unwrap_or(0)).collect();
            }
        }
    }

    // --- CURA AUTOMÁTICA (SELF-HEALING) ---
    // Invocamos a função unificada 'fix_rune_page' do nosso serviço de runas.
    // Se por acaso as runas estiverem vazias ou corrompidas no banco de dados, 
    // ela injeta dinamicamente um conjunto de runas 100% funcional baseado nas tags do campeão.
    // O aplicativo nunca deixa o jogador com a página de runas corrompida!
    let (primary_tree_id, secondary_tree_id, runes, shards) =
        crate::db::rune_sync_service::fix_rune_page(
            primary_tree_id,
            secondary_tree_id,
            &runes,
            &shards,
            &champ_tags,
        );

    // Tradução e enriquecimento visual para a interface gráfica
    let primary_tree_name = get_tree_name(primary_tree_id).to_string();
    let secondary_tree_name = get_tree_name(secondary_tree_id).to_string();
    let keystone_id = runes.first().copied().unwrap_or(0);
    let (keystone_name, keystone_desc) = get_keystone_details(keystone_id);

    let explanation = generate_coaching_explanation(
        &champ_name,
        &champ_tags,
        keystone_name,
        &primary_tree_name,
        &secondary_tree_name,
    );

    let summoner_spells = spells_str
        .as_deref()
        .and_then(|s| serde_json::from_str::<Vec<i64>>(s).ok())
        .filter(|v| !v.is_empty())
        .map(|ids| ids.iter().map(|&id| match id {
            1 => "Cleanse", 3 => "Exhaust", 4 => "Flash", 6 => "Ghost",
            7 => "Heal", 11 => "Smite", 12 => "Teleport", 13 => "Clarity",
            14 => "Ignite", 21 => "Barrier", 32 => "Mark", _ => "Flash",
        }.to_string()).collect::<Vec<_>>())
        .unwrap_or_else(|| get_recommended_summoner_spells(&resolved_role, &champ_name));

    // Montando a resposta de retorno serializada em JSON para o Frontend
    Ok(RuneOverlayData {
        champion_id: resolved_id,
        champion_name: champ_name,
        role: resolved_role,
        elo: resolved_elo,
        primary_tree_id,
        primary_tree_name,
        keystone_id,
        keystone_name: keystone_name.to_string(),
        keystone_desc: keystone_desc.to_string(),
        secondary_tree_id,
        secondary_tree_name,
        runes,
        shards,
        explanation,
        summoner_spells,
    })
}
