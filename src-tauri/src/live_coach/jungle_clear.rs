use super::state::CoachState;

/// Determina se um campeão de selva deve começar no Buff Azul.
/// AP/mana-hungry champions preferem Blue; AD/fighters preferem Red.
fn should_start_blue(champ: &str) -> bool {
    matches!(champ.to_lowercase().as_str(),
        "amumu" | "lillia" | "karthus" | "fiddlesticks" | "diana" | "evelynn" |
        "nidalee" | "gragas" | "ivern" | "elise" | "ekko" | "taliyah" | "zac" |
        "nunu" | "nunu & willump" | "sejuani" | "sylas" | "brand" | "morgana" |
        "veigar" | "malzahar" | "swain" | "twisted fate" | "twistedfate" |
        "neeko" | "zyra" | "vel'koz" | "velkoz" | "xerath" | "lux" |
        "orianna" | "lissandra" | "cassiopeia" | "fizz" | "leblanc" |
        "singed" | "aurelion sol" | "aurelionsol" | "seraphine" |
        "teemo" | "rumble" | "heimerdinger" | "kennen" | "zoe"
    )
}

/// Dica mecânica específica do campeão para o primeiro clear — em português, para o Kokoro falar corretamente.
fn get_champ_clear_tip(champ: &str) -> &'static str {
    match champ.to_lowercase().as_str() {
        "hecarim" => "E arrasta o grande — economiza tempo na transição.",
        "lee sin" | "leesin" => "Ward atrás do grande → W na ward após matá-lo.",
        "warwick" => "Passiva cura constantemente — avance sem parar.",
        "karthus" => "Q sem economizar. Recue as Acuâminas ao grupo.",
        "amumu" => "Recue manualmente afastando o grande do grupo.",
        "lillia" => "Q na borda — fogo externo causa dano extra.",
        "diana" => "W no início para o escudo. Q nos grupos pequenos.",
        "vi" => "Q atordoa o grande e interrompe o ataque dele.",
        "elise" => "Forma humana Q no grande, forma aranha Q cura.",
        "nidalee" => "Forma felina entre campos, humana Q para curar.",
        "fiddlesticks" => "W sempre ativo. E nos grupos de pequenos.",
        "evelynn" => "Passiva regenera tudo — limpe e ganke invisível.",
        "graves" => "Q derruba grupos. E entre grandes recarrega balas.",
        "rengar" => "Arbusto a cada campo — Q fortalecido é mais rápido.",
        "zac" => "Colete os pedaços caídos — cada um restaura 4% de vida.",
        "shaco" => "Caixa 3s antes de atacar o grande — medo anula dano.",
        "nunu" | "nunu & willump" => "Q consome o grande com cura massiva.",
        "sejuani" => "E atordoa o grande. W nos grupos para dano passivo.",
        "rammus" => "W ativo em cada campo, desative ao se mover.",
        "ivern" => "Q e E libertam campos sem matar — XP e ouro imediatos.",
        "master yi" | "masteryi" => "Q no começo do campo reseta o ataque básico.",
        "olaf" => "Busque o machado do Q — cada catada reduz o recarga.",
        "xin zhao" | "xinzhao" => "E arremessa o grande. Três ataques do Q depois.",
        "udyr" => "Tigre em individuais, Fênix nos grupos, Urso para mover.",
        "volibear" => "Q atordoa o grande. Passiva abaixo de 30% cura sozinha.",
        "briar" => "Passiva cura ao atacar — nunca pare sem um alvo.",
        "kayn" => "Passe pelos monstros ao matar para acumular fragmentos.",
        "trundle" => "Q reseta o ataque. Coluna de Gelo atrás do grande.",
        "nocturne" => "E ANTES do grande atacar — absorve o feitiço especial.",
        "viego" => "Q rouba vida. Guarde o W para ganks.",
        "bel'veth" | "belveth" => "Varie direções de ataque — cada direção única empilha carga.",
        "pantheon" => "Escudo passivo bloqueia o primeiro acerto do grande.",
        "shyvana" => "E em área em cada campo. Foque o Lagarto.",
        "jarvan iv" | "jarvaniv" => "E + Q arremessa o grande. R só em ganks.",
        "wukong" => "Q reseta o ataque. Clone W para escapar com vida baixa.",
        "skarner" => "E no grande para dano máximo. Q nos grupos pequenos.",
        "naafiri" => "Elimine os cachorros caídos para recuperar vida.",
        _ => "Recue o grande em direção ao próximo campo enquanto ataca.",
    }
}

/// Dica mecânica específica para cada acampamento — em português, sem termos em inglês.
fn get_camp_mechanic_tip(camp_name: &str) -> &'static str {
    let lower = camp_name.to_lowercase();
    if lower.contains("golem") || lower.contains("blue") || lower.contains("azul") {
        "Feitiço de selva no Golem a ~800 de vida."
    } else if lower.contains("lagarto") || lower.contains("red") || lower.contains("verm") || lower.contains("ancioso") {
        "Feitiço de selva no Lagarto a ~700 de vida."
    } else if lower.contains("gromp") {
        "Gromp envenena — recue atacando para manter distância."
    } else if lower.contains("lobo") || lower.contains("wolf") {
        "Mate os lobinhos primeiro, o Lobo Ancião cai sozinho."
    } else if lower.contains("acuâminas") || lower.contains("raptor") || lower.contains("galinha") {
        "Foque a Galinha Grande, habilidades em área nos pequenos."
    } else if lower.contains("pedregulho") || lower.contains("krug") || lower.contains("pedra") {
        "Pedregulho Ancião primeiro — divide em menores ao morrer."
    } else {
        "Foque o grande e recue em direção ao próximo campo."
    }
}

/// Guia de limpeza da selva em tempo real — roda independente do cooldown global (8s próprio).
/// Usa vida%, mana%, CS e posição inimiga para coaching contextual a cada etapa do clear.
pub fn get_jungle_clear_tip(
    state: &mut CoachState,
    lca_data: &serde_json::Value,
    game_time: f64,
    role: &str,
) -> Option<(String, String)> {
    if role.to_uppercase() != "JUNGLE" { return None; }
    if game_time > 270.0 { return None; }

    // Sem cooldown global: steps por CS disparam imediatamente quando o limiar é atingido.
    // O alerta de vida baixa tem cooldown próprio dentro do seu bloco.

    let active_summ = lca_data["activePlayer"]["summonerName"].as_str().unwrap_or("");
    let active_base = active_summ.split('#').next().unwrap_or(active_summ).to_lowercase();

    // activePlayer doesn't have championName in the LCA API — look it up from allPlayers.
    let champ_owned = lca_data["activePlayer"]["championName"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            lca_data["allPlayers"].as_array()?.iter()
                .find(|p| {
                    let p_name = p["summonerName"].as_str().unwrap_or("");
                    let p_base = p_name.split('#').next().unwrap_or(p_name).to_lowercase();
                    p_name == active_summ || (!active_base.is_empty() && p_base == active_base)
                })
                .and_then(|p| p["championName"].as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();
    let champ = champ_owned.as_str();
    if champ.is_empty() { return None; }

    // Coleta CS e lado do mapa
    let mut my_cs = 0i64;
    let mut team_side = "ORDER";
    if let Some(players) = lca_data["allPlayers"].as_array() {
        for p in players {
            if p["summonerName"].as_str().unwrap_or("") == active_summ {
                my_cs   = p["scores"]["creepScore"].as_i64().unwrap_or(0);
                team_side = p["team"].as_str().unwrap_or("ORDER");
                break;
            }
        }
    }

    // Vida e mana em tempo real
    let hp_curr   = lca_data["activePlayer"]["championStats"]["currentHealth"].as_f64().unwrap_or(1000.0);
    let hp_max    = lca_data["activePlayer"]["championStats"]["maxHealth"].as_f64().unwrap_or(1000.0);
    let hp_pct    = if hp_max > 0.0 { hp_curr / hp_max } else { 1.0 };
    let hp_int    = (hp_pct * 100.0) as i32;

    let mana_curr = lca_data["activePlayer"]["championStats"]["resourceValue"].as_f64().unwrap_or(100.0);
    let mana_max  = lca_data["activePlayer"]["championStats"]["resourceMax"].as_f64().unwrap_or(100.0);
    let mana_pct  = if mana_max > 0.0 { mana_curr / mana_max } else { 1.0 };
    let is_mana   = mana_max > 200.0; // energia e campeões sem recurso ficam abaixo de 200

    let is_blue   = team_side == "ORDER";
    let start_blue = should_start_blue(champ);

    // Rota de limpeza em nomes portugueses
    let path: &[&str] = if is_blue {
        if start_blue {
            &["Golem Antigo", "Gromp", "Lobos", "Acuâminas", "Lagarto Ancioso", "Pedregulhos"]
        } else {
            &["Lagarto Ancioso", "Pedregulhos", "Acuâminas", "Lobos", "Golem Antigo", "Gromp"]
        }
    } else if start_blue {
        &["Golem Antigo", "Gromp", "Lobos", "Acuâminas", "Lagarto Ancioso", "Pedregulhos"]
    } else {
        &["Lagarto Ancioso", "Pedregulhos", "Acuâminas", "Lobos", "Golem Antigo", "Gromp"]
    };

    // Aviso de mana baixa para campeões dependentes
    let mana_aviso = if is_mana && mana_pct < 0.25 {
        " Mana baixa — use as habilidades com moderação."
    } else {
        ""
    };

    // Situação de vida para mensagem contextual
    let situacao_vida = |hp: i32| -> String {
        if hp >= 70 {
            format!("Vida em {}% — ótimo ritmo.", hp)
        } else if hp >= 50 {
            format!("Vida em {}% — continue, mas fique atento.", hp)
        } else if hp >= 35 {
            format!("Vida em {}% — termine a limpeza e volte à base antes de gankar.", hp)
        } else {
            format!("Vida em {}% — cuidado! Considere voltar à base após o próximo acampamento.", hp)
        }
    };

    // Alerta de vida crítica durante a limpeza — cooldown de 10s para não repetir
    if hp_pct < 0.28 && state.first_clear_step >= 2 && state.first_clear_step < 8
        && game_time - state.last_clear_step_time >= 10.0
    {
        state.last_clear_step_time = game_time;
        return Some((
            "❤️ Vida Baixa".to_string(),
            format!("{}% de vida — termine o campo e volte à base.", hp_int)
        ));
    }

    // Etapa 0: Plano antes dos campos nascerem — dispara o mais cedo possível.
    // Usa rota do banco (db_jungle_path) se disponível; fallback para heurística local.
    if game_time >= 0.5 && state.first_clear_step == 0 {
        state.first_clear_step = 1;
        state.last_clear_step_time = game_time;

        // Prioridade 1: rota do banco (meta real extraído das partidas)
        if let Some(ref db_path) = state.db_jungle_path.clone() {
            if !db_path.is_empty() {
                let dica_champ = get_champ_clear_tip(champ);
                return Some((
                    format!("🗺️ Rota Meta — {}", champ),
                    format!("Rota: {}. {}", db_path, dica_champ)
                ));
            }
        }

        // Fallback: rota calculada localmente
        let rota_lane = if (is_blue && !start_blue) || (!is_blue && start_blue) { "Bot" } else { "Top" };
        let dica_champ = get_champ_clear_tip(champ);
        return Some((
            format!("🗺️ Rota — {}", champ),
            format!("Começa no {}. Pede leash da {}. {}", path[0], rota_lane, dica_champ)
        ));
    }

    // Etapa 1: Campos nascem 1:25
    if game_time >= 85.0 && my_cs < 4 && state.first_clear_step == 1 {
        state.first_clear_step = 2;
        state.last_clear_step_time = game_time;
        let dica = get_camp_mechanic_tip(path[0]);
        return Some((
            format!("1:25 — {}!", path[0]),
            format!("{}{}", dica, mana_aviso)
        ));
    }

    // Etapa 2: 4+ abates → segundo acampamento
    if my_cs >= 4 && my_cs < 8 && state.first_clear_step < 3 {
        state.first_clear_step = 3;
        state.last_clear_step_time = game_time;
        let dica = get_camp_mechanic_tip(path[1]);
        return Some((
            format!("4 CS de selva — {}!", path[1]),
            format!("{} {}{}", dica, situacao_vida(hp_int), mana_aviso)
        ));
    }

    // Etapa 3: 8+ abates → terceiro acampamento
    if my_cs >= 8 && my_cs < 12 && state.first_clear_step < 4 {
        state.first_clear_step = 4;
        state.last_clear_step_time = game_time;
        let dica = get_camp_mechanic_tip(path[2]);
        return Some((
            format!("8 CS de selva — {}!", path[2]),
            format!("{} {}{}", dica, situacao_vida(hp_int), mana_aviso)
        ));
    }

    // Etapa 4: 12+ abates → quarto acampamento (travessa de lado)
    if my_cs >= 12 && my_cs < 16 && state.first_clear_step < 5 {
        state.first_clear_step = 5;
        state.last_clear_step_time = game_time;
        let dica = get_camp_mechanic_tip(path[3]);
        return Some((
            format!("12 CS de selva — Travessa para o {}!", path[3]),
            format!("{} Guarde o feitiço para o próximo bufo. {}{}", dica, situacao_vida(hp_int), mana_aviso)
        ));
    }

    // Etapa 5: 16+ abates → quinto acampamento
    if my_cs >= 16 && my_cs < 20 && state.first_clear_step < 6 {
        state.first_clear_step = 6;
        state.last_clear_step_time = game_time;
        let dica = get_camp_mechanic_tip(path[4]);
        return Some((
            format!("16 CS de selva — {} — Quase Lá!", path[4]),
            format!("{} {}{}", dica, situacao_vida(hp_int), mana_aviso)
        ));
    }

    // Etapa 6: 20+ abates → último acampamento
    if my_cs >= 20 && my_cs < 24 && state.first_clear_step < 7 {
        state.first_clear_step = 7;
        state.last_clear_step_time = game_time;
        let dica = get_camp_mechanic_tip(path[5]);
        let vida_msg = if hp_pct < 0.5 {
            format!("{}% de vida — volte à base logo após terminar esse acampamento.", hp_int)
        } else {
            format!("Último acampamento! {}", situacao_vida(hp_int))
        };
        return Some((
            format!("20 CS de selva — {} — Último!", path[5]),
            format!("{} {}", dica, vida_msg)
        ));
    }

    // Etapa 7: 24+ abates → limpeza completa
    if my_cs >= 24 && state.first_clear_step < 8 {
        state.first_clear_step = 8;
        state.last_clear_step_time = game_time;
        let proxima_acao = if hp_pct < 0.45 {
            format!("{}% de vida — base antes de qualquer emboscada.", hp_int)
        } else if hp_pct < 0.65 {
            format!("{}% de vida — emboscada rápida ou base para recuperar.", hp_int)
        } else {
            format!("{}% — Aronguejo às 3:30. Posicione no rio.", hp_int)
        };
        return Some((
            "✅ Limpeza Completa — Nível 4!".to_string(),
            proxima_acao
        ));
    }

    None
}
