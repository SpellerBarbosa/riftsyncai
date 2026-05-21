use crate::lcu;

pub(super) async fn get_current_soloq_rank(conn: &lcu::LcuConnection) -> String {
    let mut elo = "GOLD".to_string();

    // EP0: /lol-ranked/v1/current-ranked-stats (Geralmente o mais direto)
    if let Ok(stats) = conn.get("/lol-ranked/v1/current-ranked-stats").await {
        if let Some(queues) = stats["queues"].as_array() {
            for q in queues {
                let q_type = q["queueType"].as_str().unwrap_or("");
                let q_tier = q["tier"].as_str().unwrap_or("");
                if q_type == "RANKED_SOLO_5x5" && !q_tier.is_empty() && q_tier != "NONE" && q_tier != "UNRANKED" {
                    let final_elo = q_tier.to_uppercase();
                    return final_elo;
                }
            }
        }
    }

    // EP1: Summoner ID + Ranked Stats
    if let Ok(summoner) = conn.get("/lol-summoner/v1/current-summoner").await {
        if let Some(sid) = summoner["summonerId"].as_i64() {
            let url = format!("/lol-ranked/v1/ranked-stats/{}", sid);
            if let Ok(stats) = conn.get(&url).await {
                if let Some(queues) = stats["queues"].as_array() {
                    for q in queues {
                        // Tenta queueType primeiro, fallback para queue
                        let q_type = q["queueType"].as_str().unwrap_or(q["queue"].as_str().unwrap_or("?"));
                        let q_tier = q["tier"].as_str().unwrap_or("UNRANKED");

                        if q_type == "RANKED_SOLO_5x5" && q_tier != "NONE" && q_tier != "UNRANKED" {
                            let final_elo = q_tier.to_uppercase();
                            return final_elo;
                        }
                    }
                }
            }
        }
    }

    // EP2: Fallback (LP stats)
    if let Ok(ranked) = conn.get("/lol-ranked/v1/current-lp-stats-by-queue").await {
        if let Some(queues) = ranked.as_array() {
            for q in queues {
                let q_type = q["queueType"].as_str().unwrap_or("");
                let q_tier = q["tier"].as_str().unwrap_or("");
                if q_type == "RANKED_SOLO_5x5" && !q_tier.is_empty() && q_tier != "NONE" && q_tier != "UNRANKED" {
                    elo = q_tier.to_uppercase();
                }
            }
        } else if let Some(q) = ranked.get("RANKED_SOLO_5x5") {
            if let Some(tier) = q["tier"].as_str() {
                if tier != "NONE" && tier != "UNRANKED" {
                    elo = tier.to_uppercase();
                }
            }
        }
    }

    elo
}

pub fn format_champion_display_name(n: &str) -> String {
    match n.to_lowercase().as_str() {
        "missfortune" => "Miss Fortune".to_string(),
        "masteryi" => "Master Yi".to_string(),
        "aurelionsol" => "Aurelion Sol".to_string(),
        "drmundo" => "Dr. Mundo".to_string(),
        "tahmkench" => "Tahm Kench".to_string(),
        "twistedfate" => "Twisted Fate".to_string(),
        "xinzhao" => "Xin Zhao".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
    }
}
