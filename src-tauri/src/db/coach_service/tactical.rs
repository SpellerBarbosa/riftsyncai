use sqlx::{Pool, Sqlite};
use tauri::State;

/// Estrutura contendo os conselhos táticos exibidos nos flashcards.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TacticalTipsResponse {
    pub matchup_front: Option<String>,
    pub matchup_back: Option<String>,
    pub item_front: Option<String>,
    pub item_back: Option<String>,
    /// true quando o Groq falhou por tokens esgotados ou rate-limit.
    /// O frontend usa este campo para decidir se deve aceitar dicas procedurais do bridge.
    #[serde(default)]
    pub groq_exhausted: bool,
}



/// Fornece o perfil tático do campeão adversário para entender como enfrentá-lo na rota.
fn get_opponent_skill_profile(opp_id: &str) -> &'static str {
    match opp_id.to_uppercase().as_str() {
        "KLED" => "Kled é extremamente agressivo em duelos corpo a corpo. Se ele te prender na corda do Q (Armadilha de Urso), você deve usar seu E (Avanço Umbral) para quebrar o cabo e fugir do puxão. NUNCA use o Q do Aatrox para fugir dele.",
        "CAITLYN" => "Caitlyn é uma atiradora ADC de longo alcance com forte fase de rotas. Ela coloca armadilhas (W), dispara tiros perfurantes (Q), usa a rede (E) para recuar e desacelerar, e usa a ultimate (R) para executar alvos isolados. Ela é frágil; se Lux enraizá-la com o Q, Caitlyn morre facilmente.",
        "JINX" => "Jinx é uma atiradora ADC extremamente frágil e sem mobilidade ou saltos rápidos. Se ela for presa pela gaiola (E) do Veigar ou explodida por magos, morre instantaneamente.",
        "HECARIM" => "Hecarim é um lutador super rápido que tenta correr para cima com o E e dar medo com o R. Ele sofre se for contido por stuns ou controle de grupo antes de colar em você.",
        "AMUMU" => "Amumu é um tanque que inicia lutas com o Q e o R. Evite se agrupar demais para não ser pego pela ultimate dele.",
        "DARIUS" => "Darius é letal em duelos corpo a corpo. Evite lutas longas que permitam que ele acumule 5 marcas de sangramento.",
        "AATROX" => "Aatrox depende totalmente de acertar as lâminas do Q para causar controle de grupo e curar-se. Movimentar-se rápido de forma imprevisível quebra a mecânica dele.",
        "TEEMO" => "Teemo tenta cegar com o Q. Compre Lente Reveladora para desarmar seus cogumelos invisíveis do R.",
        _ => "Desvie de suas habilidades principais, mantenha distância segura se for frágil e explore a falta de mobilidade do inimigo."
    }
}

/// Comando Tauri invocado pelo frontend para atualizar os cartões de dicas na tela.
/// 
/// **Para o estudante (Fluxo do Programa):**
/// 1. Resolve o ID do campeão do jogador.
/// 2. Busca a pior matchup estatística dele (o maior counter registrado na base de dados).
/// 3. Consolida todas as habilidades e informações da nossa base de dados SQL local.
/// 4. Se a IA na Nuvem estiver configurada, tenta gerar um prompt customizado e consultar o OpenRouter.
/// 5. Se o OpenRouter falhar ou estiver offline, roda a inteligência local assíncrona baseada na 
///    telemetria ao vivo da partida (Riot LCA).
/// 6. Se o jogo não estiver aberto, cai no último fallback de dados estatísticos do banco de dados local.
/// Isso é o que chamamos de **Arquitetura Resiliente de Três Camadas (Cloud AI -> Telemetria LCA -> Banco de Dados Local)**.
#[tauri::command]
pub async fn get_tactical_tips_command(
    state: State<'_, crate::db::DbState>,
    champ_id: String,
) -> Result<TacticalTipsResponse, String> {
    let pool = &state.0;

    // 1. Resolve o nome ou ID do campeão
    let resolved_id: String = sqlx::query_scalar("SELECT id FROM champions WHERE LOWER(id) = LOWER(?) OR LOWER(name) = LOWER(?)")
        .bind(&champ_id)
        .bind(&champ_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| champ_id.clone());

    // 2. Busca a pior matchup dele no banco
    let worst_matchup: Option<(String, f64)> = sqlx::query_as(
        "SELECT opponent_id, win_rate FROM matchups WHERE champion_id = ? AND win_rate IS NOT NULL ORDER BY win_rate ASC LIMIT 1"
    )
    .bind(&resolved_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    // 3. Obtém metadados do campeão
    let champ_name: String = sqlx::query_scalar("SELECT name FROM champions WHERE id = ?")
        .bind(&resolved_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| resolved_id.clone());
    let champ_tags: String = sqlx::query_scalar("SELECT tags FROM champions WHERE id = ?")
        .bind(&resolved_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .unwrap_or_default();

    let clean_tags = if champ_tags.trim().starts_with('[') {
        serde_json::from_str::<Vec<String>>(&champ_tags).unwrap_or_default().join(", ")
    } else {
        champ_tags
    };

    // 4. Busca os itens recomendados
    let recommended_items_json: Option<String> = sqlx::query_scalar(
        "SELECT items_json FROM recommended_builds WHERE champion_id = ? AND elo = 'CHALLENGER' AND items_json IS NOT NULL LIMIT 1"
    )
    .bind(&resolved_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let mut item_names = Vec::new();
    if let Some(json_str) = recommended_items_json {
        if let Ok(item_ids) = serde_json::from_str::<Vec<i64>>(&json_str) {
            for id in item_ids.iter().take(4) {
                let name: Option<String> = sqlx::query_scalar("SELECT name FROM items WHERE id = ?")
                    .bind(&id.to_string())
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None);
                if let Some(n) = name {
                    item_names.push(n);
                }
            }
        }
    }

    let first_item = item_names.first().cloned().unwrap_or_else(|| {
        if clean_tags.to_lowercase().contains("mage") {
            "Companheiro de Luden".to_string()
        } else if clean_tags.to_lowercase().contains("tank") {
            "Égide de Fogo Solar".to_string()
        } else if clean_tags.to_lowercase().contains("marksman") {
            "Gume do Infinito".to_string()
        } else {
            "Quebra-passos".to_string()
        }
    });

    let opp_name = worst_matchup.as_ref().map(|(o, _)| o.clone()).unwrap_or_else(|| "Inimigo".to_string());

    let opp_spells_opt: Option<String> = sqlx::query_scalar("SELECT spells_description FROM champions WHERE id = ?")
        .bind(&opp_name)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

    let opp_skills = opp_spells_opt
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| get_opponent_skill_profile(&opp_name).to_string());

    // Habilidades do próprio campeão — âncora para o Groq não alucinar mecânicas
    let own_spells_opt: Option<String> = sqlx::query_scalar("SELECT spells_description FROM champions WHERE id = ?")
        .bind(&resolved_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    let own_skills = own_spells_opt
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{} ({})", champ_name, clean_tags));

    // Regras locais baseadas em estatísticas estáticas (Último Fallback)
    let (matchup_front, matchup_back) = match worst_matchup.as_ref() {
        Some((opp, wr)) => {
            let opp_wr_percent = ((1.0 - wr) * 100.0).round();
            let mut threat_tip = " Controle a onda e evite trocas longas.".to_string();
            let o_skills_lower = opp_skills.to_lowercase();
            if o_skills_lower.contains("atordoa") || o_skills_lower.contains("stun") || o_skills_lower.contains("medo") || o_skills_lower.contains("fear") {
                threat_tip = " Desvie do CC dele antes de iniciar trocas.".to_string();
            } else if o_skills_lower.contains("avanço") || o_skills_lower.contains("dash") || o_skills_lower.contains("pula") {
                threat_tip = " Alta mobilidade — jogue recuado e espere erros.".to_string();
            } else if o_skills_lower.contains("invisível") || o_skills_lower.contains("furtivo") {
                threat_tip = " Compre ward de controle para revelar emboscadas.".to_string();
            }

            (
                format!("Como {} enfrenta {}?", champ_name, opp),
                format!("{} counter forte ({}% vitória inimiga).{}", opp, opp_wr_percent, threat_tip)
            )
        }
        None => {
            (
                format!("{} — posicionamento na rota?", champ_name),
                "Controle a onda e ward no rio contra ganks.".to_string()
            )
        }
    };

    let item_front = format!("Qual a força de {} em {}?", first_item, champ_name);
    
    let mut item_purpose = "pico de poder para as primeiras lutas.".to_string();
    let item_lower = first_item.to_lowercase();
    if item_lower.contains("luden") {
        item_purpose = "burst e mana para limpar ondas rápido.".to_string();
    } else if item_lower.contains("zhonya") || item_lower.contains("ampulheta") {
        item_purpose = "estase e armadura contra ultimates inimigos.".to_string();
    } else if item_lower.contains("gume") || item_lower.contains("infinity") {
        item_purpose = "dano crítico máximo para carregar como atirador.".to_string();
    } else if item_lower.contains("fogo solar") || item_lower.contains("sunfire") {
        item_purpose = "vida, armadura e queima passiva na selva.".to_string();
    } else if item_lower.contains("youmuu") {
        item_purpose = "letalidade e velocidade para rotacionar e emboscar.".to_string();
    }

    let item_back = format!("Feche '{}' cedo — {}.", first_item, item_purpose);

    // --- CAMADA 1: GROQ (INFERÊNCIA RÁPIDA NA NUVEM) ---
    let groq_exhausted;

    let opp_percent_str = worst_matchup.as_ref()
        .map(|(_, wr)| format!("{:.0}", (1.0 - wr) * 100.0))
        .unwrap_or_default();

    let own_ctx  = own_skills.chars().take(150).collect::<String>();
    let opp_ctx  = opp_skills.chars().take(150).collect::<String>();

    // Cache key: champion|opponent|item — mesmo cenário = mesma dica, TTL 14 dias
    let opp_key = worst_matchup.as_ref().map(|(o, _)| o.as_str()).unwrap_or("none");
    let groq_cache_key = format!("groq:{}:{}:{}", resolved_id, opp_key, first_item);

    // Tenta recuperar do cache SQLite antes de chamar o Groq
    let cached: Option<String> = sqlx::query_scalar(
        "SELECT response_json FROM groq_cache WHERE cache_key = ? AND expires_at > datetime('now')"
    ).bind(&groq_cache_key).fetch_optional(pool).await.unwrap_or(None);

    if let Some(cached_json) = cached {
        if let Ok(mut parsed) = serde_json::from_str::<TacticalTipsResponse>(&cached_json) {
            let has_content = parsed.matchup_back.as_deref().map(|s| !s.is_empty()).unwrap_or(false)
                && parsed.item_back.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
            if has_content {
                parsed.groq_exhausted = false;
                println!("[Coach:Groq] Cache HIT — sem chamada à API ({}).", groq_cache_key);
                return Ok(parsed);
            }
        }
    }

    let groq_prompt = if let Some((opp, _)) = worst_matchup.as_ref() {
        format!(
            "Campeão: {champ} ({tags}). Mecânicas: {own_ctx}\n\
             Counter: {opp} ({opp_wr}% win rate). Mecânicas do {opp}: {opp_ctx}\n\
             Item core: {item} ({item_purpose})\n\n\
             Preencha o JSON abaixo em PT-BR, máx 10 palavras por campo, ação concreta, sem nome de jogador:\n\
             {{\"matchup_front\":\"comportamento vs {opp} na rota\",\"matchup_back\":\"ameaça principal do {opp} e counter\",\"item_front\":\"{item} vs {opp}: vantagem principal\",\"item_back\":\"ative {item} após [gatilho específico do {opp}]\"}}",
            champ = champ_name, tags = clean_tags, item = first_item,
            item_purpose = item_purpose,
            opp = opp, opp_wr = opp_percent_str,
            own_ctx = own_ctx, opp_ctx = opp_ctx,
        )
    } else {
        format!(
            "Campeão: {champ} ({tags}). Mecânicas: {own_ctx}\n\
             Item core: {item} ({item_purpose})\n\n\
             Preencha o JSON abaixo em PT-BR, máx 10 palavras por campo, ação concreta, sem nome de jogador:\n\
             {{\"matchup_front\":\"{champ}: posicionamento chave na rota\",\"matchup_back\":\"quando {champ} inicia vs quando recua\",\"item_front\":\"{item}: por que é o core ideal\",\"item_back\":\"ative {item} após [gatilho de combate]\"}}",
            champ = champ_name, tags = clean_tags, item = first_item,
            item_purpose = item_purpose, own_ctx = own_ctx,
        )
    };

    match crate::groq::get_groq_tip_internal(pool, groq_prompt).await {

        Ok(json_res) => {
            let mut cleaned = json_res.clone();
            if let Some(start) = cleaned.find('{') { cleaned = cleaned[start..].to_string(); }
            if let Some(end) = cleaned.rfind('}') { cleaned = cleaned[..=end].to_string(); }

            if let Ok(mut parsed) = serde_json::from_str::<TacticalTipsResponse>(&cleaned) {
                let has_content = parsed.matchup_back.as_deref().map(|s| !s.is_empty()).unwrap_or(false)
                    && parsed.item_back.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
                if has_content {
                    parsed.groq_exhausted = false;
                    // Salva no cache por 14 dias
                    let _ = sqlx::query(
                        "INSERT OR REPLACE INTO groq_cache (cache_key, response_json, expires_at)
                         VALUES (?, ?, datetime('now', '+14 days'))"
                    ).bind(&groq_cache_key).bind(&cleaned).execute(pool).await;
                    // Limpa expirados + mantém máx 300 entradas (~150KB de texto)
                    let _ = sqlx::query("DELETE FROM groq_cache WHERE expires_at <= datetime('now')")
                        .execute(pool).await;
                    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM groq_cache")
                        .fetch_one(pool).await.unwrap_or(0);
                    if count > 2000 {
                        let _ = sqlx::query(
                            "DELETE FROM groq_cache WHERE cache_key IN
                             (SELECT cache_key FROM groq_cache ORDER BY created_at ASC LIMIT ?)"
                        ).bind(count - 2000).execute(pool).await;
                    }
                    println!("[Coach:Groq] Cacheado 14 dias — {} entradas no cache.", count.min(2000));
                    return Ok(parsed);
                }
            }
            // JSON mal formado ou campos vazios: trata como falha e libera procedural
            groq_exhausted = true;
            println!("[Coach:Groq] Resposta inválida/vazia — liberando procedural.");
        }
        Err(e) => {
            // Qualquer erro do Groq (tokens, rate-limit, rede, sem chave) libera o procedural
            groq_exhausted = true;
            println!("[Coach:Groq] Falhou ({}). Procedural liberado como fallback.", e);
        }
    }

    // --- CAMADA 2: TELEMETRIA EM TEMPO REAL (RIOT LCA) ---
    if let Some(mut live_tips) = analyze_live_game_state(pool, &champ_name, &first_item, &item_purpose).await {
        live_tips.groq_exhausted = groq_exhausted;
        println!("[Coach:LCA] Usando telemetria do jogo ativo.");
        return Ok(live_tips);
    }

    // --- CAMADA 3: BANCO DE DADOS LOCAL (ESTÁTICO FALLBACK) ---
    println!("[Coach:Static] Usando dicas estáticas locais.");
    Ok(TacticalTipsResponse {
        matchup_front: Some(matchup_front),
        matchup_back: Some(matchup_back),
        item_front: Some(item_front),
        item_back: Some(item_back),
        groq_exhausted,
    })
}

/// Analisa os dados dinâmicos da partida ativa (LCA) e constrói dicas táticas em tempo real.
/// 
/// **Para o estudante:**
/// Aqui lemos o endpoint `/liveclientdata/allgamedata` da Riot Games.
/// Se o jogador estiver sofrendo na rota (mais mortes que abates), ativamos dicas de congelamento de onda.
/// Se estiver dominando, incentivamos o "Slow Push" ou pressão de roaming.
/// Também avaliamos a composição inimiga (ex: alertar para comprar resistência mágica se houver 3+ ameaças AP).
pub async fn analyze_live_game_state(
    _pool: &Pool<Sqlite>,
    champ_name: &str,
    first_item: &str,
    item_purpose: &str,
) -> Option<TacticalTipsResponse> {
    let lca = crate::lca::LcaConnection::new();
    let game_data = lca.get("allgamedata").await.ok()?;
    
    let game_time_secs = game_data["gameData"]["gameTime"].as_f64()?;
    let game_time_mins = game_time_secs / 60.0;
    
    let active_name = game_data["activePlayer"]["summonerName"].as_str()?;
    
    let players = game_data["allPlayers"].as_array()?;
    let active_player_opt = players.iter().find(|p| p["summonerName"].as_str() == Some(active_name));
    
    let mut kills = 0;
    let mut deaths = 0;
    let mut cs = 0;
    let mut team = "ORDER".to_string();
    let mut vision_score = 0.0;
    
    if let Some(ap) = active_player_opt {
        kills = ap["scores"]["kills"].as_i64().unwrap_or(0);
        deaths = ap["scores"]["deaths"].as_i64().unwrap_or(0);
        cs = ap["scores"]["creepScore"].as_i64().unwrap_or(0);
        team = ap["team"].as_str().unwrap_or("ORDER").to_string();
        vision_score = ap["scores"]["wardScore"].as_f64().unwrap_or(0.0);
    }
    
    let mut enemy_ap_count = 0;
    let mut enemy_ad_count = 0;
    let mut worst_enemy_name = "Inimigo".to_string();
    let mut max_enemy_kills = -1;
    
    for p in players {
        if p["team"].as_str().unwrap_or("ORDER") != team {
            let champ = p["championName"].as_str().unwrap_or("?");
            let enemy_kills = p["scores"]["kills"].as_i64().unwrap_or(0);
            
            if enemy_kills > max_enemy_kills {
                max_enemy_kills = enemy_kills;
                worst_enemy_name = champ.to_string();
            }
            
            // Heurística básica de ameaça mágica (AP)
            let is_ap = ["Ahri", "Akali", "Anivia", "Annie", "AurelionSol", "Azir", "Brand", "Cassiopeia", "Diana", 
                         "Evelynn", "Fiddlesticks", "Fizz", "Galio", "Gragas", "Heimerdinger", "Karthus", "Kassadin", 
                         "Katarina", "Kayle", "Leblanc", "Lillia", "Lissandra", "Lux", "Malzahar", "Morderkaiser", 
                         "Morgana", "Neeko", "Nidalee", "Nunu", "Orianna", "Rumble", "Ryze", "Singed", "Swain", 
                         "Sylas", "Syndra", "Taliyah", "Teemo", "TwistedFate", "Veigar", "Velkoz", "Viktor", 
                         "Vladmir", "Xerath", "Ziggas", "Zoe", "Zyra"].contains(&champ);
            if is_ap {
                enemy_ap_count += 1;
            } else {
                enemy_ad_count += 1;
            }
        }
    }
    
    // DETECÇÃO AUTOMÁTICA DA ROTA (ROLE)
    let mut detected_role = "MID".to_string();
    let raw_position = active_player_opt
        .and_then(|p| p["position"].as_str())
        .unwrap_or("MIDDLE")
        .to_uppercase();
        
    if raw_position == "MIDDLE" {
        detected_role = "MID".to_string();
    } else if raw_position == "BOTTOM" {
        detected_role = "ADC".to_string();
    } else if raw_position == "UTILITY" {
        detected_role = "SUPPORT".to_string();
    } else if raw_position == "JUNGLE" {
        detected_role = "JUNGLE".to_string();
    } else if raw_position == "TOP" {
        detected_role = "TOP".to_string();
    }
    
    // Validação extra: Se tem feitiço Smite/Golpear, é caçador (Jungle)
    let mut has_smite = false;
    let mut has_support_item = false;
    
    if let Some(ap) = active_player_opt {
        if let Some(spells) = ap.get("summonerSpells") {
            let s1 = spells["summonerSpellOne"]["displayName"].as_str().unwrap_or("").to_lowercase();
            let s2 = spells["summonerSpellTwo"]["displayName"].as_str().unwrap_or("").to_lowercase();
            if s1.contains("smite") || s1.contains("golpear") || s2.contains("smite") || s2.contains("golpear") {
                has_smite = true;
            }
        }
        if let Some(items) = ap.get("items").and_then(|i| i.as_array()) {
            for item in items {
                let item_id = item["itemId"].as_i64().unwrap_or(0);
                if item_id >= 3858 && item_id <= 3864 {
                    has_support_item = true;
                }
            }
        }
    }
    
    if has_smite {
        detected_role = "JUNGLE".to_string();
    } else if has_support_item {
        detected_role = "SUPPORT".to_string();
    }
    
    let matchup_front = format!("Live [{}] | {} vs {}", detected_role, champ_name, worst_enemy_name);
    let mut matchup_back = String::new();
    
    // --- COMPUTAÇÃO DE DICAS DE MICRO-GAME (TÁTICA E FARM) ---
    match detected_role.as_str() {
        "TOP" => {
            if deaths > kills + 2 {
                matchup_back.push_str("Congele a onda na torre para farmar seguro.");
            } else if kills > deaths + 2 {
                matchup_back.push_str("Slow push no rio para forçar o inimigo a avançar.");
            } else {
                matchup_back.push_str("Use o arbusto para resetar agro após trocas.");
            }
            if game_time_mins > 2.0 {
                let cs_per_min = (cs as f64) / game_time_mins;
                if cs_per_min < 6.0 {
                    matchup_back.push_str(&format!(" {:.1} CS/min — foque em last hits.", cs_per_min));
                } else {
                    matchup_back.push_str(&format!(" {:.1} CS/min — continue acumulando vantagem.", cs_per_min));
                }
            } else {
                matchup_back.push_str(" Nível 2 antes do adversário dita o ritmo.");
            }
        },
        "JUNGLE" => {
            let cs_per_min = if game_time_mins > 2.0 { (cs as f64) / game_time_mins } else { 0.0 };
            if cs_per_min < 4.0 {
                matchup_back.push_str(&format!("{:.1} CS/min — limpe campos antes de gankar.", cs_per_min));
            } else {
                matchup_back.push_str(&format!("{:.1} CS/min — gank pela rota com onda avançada.", cs_per_min));
            }
            matchup_back.push_str(" Recue o monstro grande em direção ao próximo campo.");
        },
        "MID" => {
            if deaths > kills + 2 {
                matchup_back.push_str("Guarde mobilidade para fugir, jogue defensivo.");
            } else if kills > deaths + 2 {
                matchup_back.push_str("Domine — limpe ondas e some do mapa.");
            } else {
                matchup_back.push_str("Ataque após ele gastar habilidade de controle.");
            }
            if game_time_mins > 2.0 {
                let cs_per_min = (cs as f64) / game_time_mins;
                if cs_per_min < 6.5 {
                    matchup_back.push_str(&format!(" {:.1} CS/min — priorize last hits.", cs_per_min));
                } else {
                    matchup_back.push_str(&format!(" {:.1} CS/min — avance antes de rotacionar.", cs_per_min));
                }
            } else {
                matchup_back.push_str(" Ajude o caçador em invasões de rio nível 2.");
            }
        },
        "ADC" => {
            if game_time_mins > 2.0 {
                let cs_per_min = (cs as f64) / game_time_mins;
                if cs_per_min < 6.8 {
                    matchup_back.push_str(&format!("{:.1} CS/min — recue para ondas seguras.", cs_per_min));
                } else {
                    matchup_back.push_str(&format!("{:.1} CS/min — priorize 3 itens no pico.", cs_per_min));
                }
            }
            if max_enemy_kills >= 3 {
                matchup_back.push_str(&format!(" {} forte — não inicie na linha de frente.", worst_enemy_name));
            } else {
                matchup_back.push_str(" Ataque atrás dos tanques, mantenha alcance máximo.");
            }
        },
        "SUPPORT" => {
            if game_time_mins > 2.0 {
                let vis_score_per_min = vision_score / game_time_mins;
                if vis_score_per_min < 1.0 {
                    matchup_back.push_str(&format!("{:.1} vis/min — ward rio e selva inimiga.", vis_score_per_min));
                } else {
                    matchup_back.push_str(&format!("{:.1} vis/min — continue limpando sentinelas inimigas.", vis_score_per_min));
                }
            }
            matchup_back.push_str(" Fique ao lado do atirador e proteja nas trocas.");
        },
        _ => {}
    }
    
    // --- COMPUTAÇÃO DE DICAS DE MACRO-GAME (FASE DO JOGO E OBJETIVOS) ---
    let item_front = format!("Macro [{}] & Visão Geral ({:.0} min)", detected_role, game_time_mins);
    let mut item_back = String::new();
    
    // Determinando a fase do jogo (Early, Mid, Late) baseado no tempo
    let phase = if game_time_mins < 14.0 { "Early" } else if game_time_mins < 25.0 { "Mid" } else { "Late" };
    
    match detected_role.as_str() {
        "TOP" => {
            if phase == "Early" {
                item_back.push_str("Barricadas e Vastilarvas com prioridade de rota.");
            } else if phase == "Mid" {
                item_back.push_str("Split push oposto ao objetivo e TP nas lutas.");
            } else {
                item_back.push_str("Líder de linha de frente nas lutas por objetivos.");
            }
        },
        "JUNGLE" => {
            if phase == "Early" {
                if game_time_mins < 8.0 {
                    item_back.push_str("5:00 — Vastilarvas ou Dragão, decida com o time.");
                } else {
                    item_back.push_str("Arauto às 8:00 — visão na fenda e prioridade.");
                }
            } else if phase == "Mid" {
                item_back.push_str("Ward Dragão e Baron. Smite só para objetivos.");
            } else {
                item_back.push_str("Dragão Ancião e Barão — Smite define a vitória.");
            }
        },
        "MID" => {
            if phase == "Early" {
                item_back.push_str("Ward na selva inimiga e rotacione com o caçador.");
            } else if phase == "Mid" {
                item_back.push_str("Empurre laterais e rotacione para objetivos do rio.");
            } else {
                item_back.push_str("Espere o timing perfeito para explodir ADC ou Mid.");
            }
        },
        "ADC" => {
            if phase == "Early" {
                item_back.push_str("Farm seguro — avance só com visão do caçador.");
            } else if phase == "Mid" {
                item_back.push_str("Rotas encerradas — vá para o meio farmar seguro.");
            } else {
                item_back.push_str("Nunca ande sozinho — colado com o suporte.");
            }
        },
        "SUPPORT" => {
            if phase == "Early" {
                item_back.push_str("Ward nas entradas, ajude o atirador na onda.");
            } else if phase == "Mid" {
                item_back.push_str("Atirador seguro — rotacione com o caçador.");
            } else {
                item_back.push_str("Ward absoluto em Barão e Dragão Ancião.");
            }
        },
        _ => {}
    }

    item_back.push_str(&format!(" '{}' {}.", first_item, item_purpose));

    if enemy_ap_count >= 3 {
        item_back.push_str(" 3+ AP inimigos — compre resistência mágica.");
    } else if enemy_ad_count >= 3 {
        item_back.push_str(" 3+ AD inimigos — priorize armadura.");
    }
    
    Some(TacticalTipsResponse {
        matchup_front: Some(matchup_front),
        matchup_back: Some(matchup_back),
        item_front: Some(item_front),
        item_back: Some(item_back),
        groq_exhausted: false,
    })
}

/// Alerta inteligente de compras no inventário ( threshold > 700g ).
/// 
/// **Para o estudante:**
/// Esta função resolve um problema de usabilidade clássico: o jogador acumula ouro e esquece de voltar base 
/// para comprar seus componentes de itens core.
/// 
/// **Fluxo e Lógica do Algoritmo:**
/// 1. Busca os itens recomendados (Core Build) do campeão no banco SQLite.
/// 2. Converte a lista de itens e o inventário atual do jogador em strings.
/// 3. Varre a lista sequencial de itens-alvo. O primeiro item que o jogador ainda não fechou é o nosso alvo atual.
/// 4. Carrega a receita deste item (seus componentes/ingredientes) do banco de dados.
/// 5. Remove do inventário temporário os componentes que o jogador já possui para descobrir quais faltam.
/// 6. Se o jogador tiver ouro suficiente para comprar o primeiro ingrediente que falta, E esse ingrediente 
///    custar 700 de ouro ou mais, emitimos um alerta na tela: "Volte base! Você pode comprar [Ingrediente]!".
/// Isso evita o spam de alertas irritantes para itens baratos de 50 de ouro (como poções de vida).
pub async fn get_next_item_purchase_alert(
    pool: &Pool<Sqlite>,
    champion_id: &str,
    role: &str,
    current_gold: f64,
    inventory_items: Vec<i64>
) -> Result<Option<String>, String> {
    // 1. Resolve o nome ou ID do campeão para bater com o slug do banco (ex: "Lee Sin" -> "LeeSin")
    let resolved_id: String = sqlx::query_scalar(
        "SELECT id FROM champions WHERE LOWER(id) = LOWER(?) OR LOWER(name) = LOWER(?)"
    )
    .bind(champion_id)
    .bind(champion_id)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|_| champion_id.to_string());

    // Busca a core build recomendada
    let recommended_items_json: Option<String> = sqlx::query_scalar(
        "SELECT items_json FROM recommended_builds WHERE champion_id = ? AND role = ? AND elo = 'CHALLENGER' AND is_core = 1 LIMIT 1"
    )
    .bind(&resolved_id).bind(role).fetch_optional(pool).await.unwrap_or(None).flatten();

    let recommended_items_json = if recommended_items_json.is_none() {
        sqlx::query_scalar(
            "SELECT items_json FROM recommended_builds WHERE champion_id = ? AND elo = 'CHALLENGER' AND is_core = 1 LIMIT 1"
        )
        .bind(&resolved_id).fetch_optional(pool).await.unwrap_or(None).flatten()
    } else {
        recommended_items_json
    };

    let build_str = recommended_items_json.unwrap_or_else(|| "[]".to_string());
    let raw_target_items: Vec<String> = serde_json::from_str(&build_str).unwrap_or_default();
    
    if raw_target_items.is_empty() { return Ok(None); }

    // Filtra itens iniciais e consumíveis (gold_total < 1000) para evitar travamentos
    let mut target_items = Vec::new();
    if !raw_target_items.is_empty() {
        let placeholders = raw_target_items.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query_str = format!(
            "SELECT id FROM items WHERE id IN ({}) AND gold_total >= 1000",
            placeholders
        );
        let mut query = sqlx::query_as::<_, (String,)>(&query_str);
        for item_id in &raw_target_items {
            query = query.bind(item_id);
        }
        if let Ok(rows) = query.fetch_all(pool).await {
            let valid_ids: std::collections::HashSet<String> = rows.into_iter().map(|(id,)| id).collect();
            for item_id in raw_target_items {
                if valid_ids.contains(&item_id) {
                    target_items.push(item_id);
                }
            }
        }
    }

    if target_items.is_empty() { return Ok(None); }

    let mut inv: Vec<String> = inventory_items.iter().map(|i| i.to_string()).collect();

    // Identifica o primeiro item alvo incompleto
    let mut target_item_id = String::new();
    for t_item in target_items {
        if !inv.contains(&t_item) {
            target_item_id = t_item;
            break;
        }
    }
    
    if target_item_id.is_empty() {
        return Ok(None);
    }

    // Carrega dados do item e seus componentes
    let row: Option<(String, i32, i32, String)> = sqlx::query_as(
        "SELECT name, gold_total, gold_base, components_json FROM items WHERE id = ?"
    )
    .bind(&target_item_id)
    .fetch_optional(pool).await.unwrap_or(None);

    if let Some((name, total, base, components_json_str)) = row {
        let components: Vec<String> = serde_json::from_str(&components_json_str).unwrap_or_default();
        
        // Simulação do inventário: remove o que o jogador já comprou
        let mut missing_components = Vec::new();
        for comp in components {
            if let Some(pos) = inv.iter().position(|x| *x == comp) {
                inv.remove(pos);
            } else {
                missing_components.push(comp);
            }
        }

        if !missing_components.is_empty() {
            // Existe um ingrediente faltando
            let next_comp_id = &missing_components[0];
            let comp_row: Option<(String, i32)> = sqlx::query_as(
                "SELECT name, gold_total FROM items WHERE id = ?"
            )
            .bind(next_comp_id).fetch_optional(pool).await.unwrap_or(None);
            
            if let Some((comp_name, comp_cost)) = comp_row {
                // Só avisa se o ouro acumulado for >= 700 E pudermos pagar pelo ingrediente
                if current_gold >= 700.0 && current_gold >= (comp_cost as f64) {
                    return Ok(Some(format!("Volte base [B]! Você tem {}g e pode comprar {} (para {}).", current_gold as i32, comp_name, name)));
                } else if current_gold >= (total as f64) {
                    return Ok(Some(format!("Volte base [B]! Você tem ouro para fechar o item inteiro: {}!", name)));
                }
            }
        } else {
            // Todos os ingredientes estão no inventário, falta apenas o custo da receita (upgrade_cost)
            let upgrade_cost = if components_json_str == "[]" { total } else { base };
            if current_gold >= 700.0 && current_gold >= (upgrade_cost as f64) {
                return Ok(Some(format!("Volte base [B]! Você tem {}g e pode fechar o item completo: {}!", current_gold as i32, name)));
            } else if current_gold >= (upgrade_cost as f64) {
                return Ok(Some(format!("Volte base [B]! Você tem ouro para fechar o item completo: {}!", name)));
            }
        }
    }
    
    Ok(None)
}
