use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Pool, Sqlite};
use tauri::Manager;

const DEFAULT_GROQ_MODEL: &str = "meta-llama/llama-4-scout-17b-16e-instruct";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroqSettings {
    pub enabled: bool,
    pub api_key: String,
    pub model: String,
}

#[tauri::command]
pub async fn get_groq_settings(app: tauri::AppHandle) -> Result<GroqSettings, String> {
    let state = match app.try_state::<crate::db::DbState>() {
        Some(s) => s,
        None => return Ok(GroqSettings {
            enabled: false,
            api_key: String::new(),
            model: DEFAULT_GROQ_MODEL.to_string(),
        }),
    };
    let pool = &state.0;

    let enabled_str: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT value FROM sync_metadata WHERE key = 'enable_groq'"
    ).fetch_optional(pool).await.unwrap_or(None);
    let enabled = enabled_str.unwrap_or_else(|| "false".to_string()) == "true";

    let api_key_opt: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT value FROM sync_metadata WHERE key = 'groq_key'"
    ).fetch_optional(pool).await.unwrap_or(None);
    let mut api_key = api_key_opt.unwrap_or_default();
    if api_key.trim().is_empty() {
        if let Ok(env_key) = std::env::var("GROQ_API_KEY") {
            api_key = env_key;
        }
    }

    let model_opt: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT value FROM sync_metadata WHERE key = 'groq_model'"
    ).fetch_optional(pool).await.unwrap_or(None);
    let model = model_opt.unwrap_or_else(|| DEFAULT_GROQ_MODEL.to_string());

    Ok(GroqSettings { enabled, api_key, model })
}

#[tauri::command]
pub async fn set_groq_settings(
    app: tauri::AppHandle,
    enabled: bool,
    api_key: String,
    model: String,
) -> Result<(), String> {
    let state = match app.try_state::<crate::db::DbState>() {
        Some(s) => s,
        None => return Err("Banco de dados não pronto.".to_string()),
    };
    let pool = &state.0;

    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('enable_groq', ?)")
        .bind(if enabled { "true" } else { "false" })
        .execute(pool).await.map_err(|e| e.to_string())?;

    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('groq_key', ?)")
        .bind(&api_key)
        .execute(pool).await.map_err(|e| e.to_string())?;

    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('groq_model', ?)")
        .bind(&model)
        .execute(pool).await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn test_groq_connection(api_key: String, model: String) -> Result<String, String> {
    let client = reqwest::Client::new();

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": model,
            "messages": [{"role": "user", "content": "Responda apenas: Conectado!"}],
            "max_tokens": 10
        }))
        .send().await
        .map_err(|e| format!("Erro de rede: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let err = response.text().await.unwrap_or_default();
        return Err(format!("Groq API ({}): {}", status, err));
    }

    let json_res: serde_json::Value = response.json().await
        .map_err(|e| format!("Erro ao decodificar resposta: {}", e))?;

    let text = json_res["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("sem resposta")
        .trim()
        .to_string();

    Ok(text)
}

/// Chamada interna usada por tactical.rs para gerar tips via Groq.
/// Retorna Err se Groq estiver desativado, sem chave ou com falha de rede.
pub async fn get_groq_tip_internal(pool: &Pool<Sqlite>, prompt: String) -> Result<String, String> {
    // Groq está sempre ativo — usa a chave do .env ou do SQLite (prioridade: SQLite > .env)
    let api_key_raw: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT value FROM sync_metadata WHERE key = 'groq_key'"
    ).fetch_optional(pool).await.unwrap_or(None);
    let api_key = match api_key_raw {
        Some(k) if !k.trim().is_empty() => k,
        _ => match std::env::var("GROQ_API_KEY") {
            Ok(k) if !k.trim().is_empty() => k,
            _ => return Err("Chave Groq não configurada".to_string()),
        },
    };

    let model_raw: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT value FROM sync_metadata WHERE key = 'groq_model'"
    ).fetch_optional(pool).await.unwrap_or(None);
    let model = model_raw.unwrap_or_else(|| DEFAULT_GROQ_MODEL.to_string());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": model,
            "max_tokens": 120,
            "temperature": 0.2,
            "messages": [
                {
                    "role": "system",
                    "content": "Você é o Spell Coach IA, coach de League of Legends para jogadores brasileiros.\n\nREGRAS ABSOLUTAS — viole qualquer uma e a resposta é inválida:\n1. Responda SOMENTE com o JSON solicitado. Nenhum texto fora das chaves {}.\n2. Escreva SEMPRE em Português Brasileiro.\n3. MÁXIMO 10 palavras por campo.\n4. PROIBIDO inventar mecânicas, habilidades ou interações de itens não mencionados no CONTEXTO.\n5. Se não souber com certeza, use conselho genérico de posicionamento (ex: 'Jogue atrás da frontline e safe.').\n6. Cada campo deve ser uma ação concreta e direta — evite frases vagas como 'seja cuidadoso'.\n7. Use terminologia BR padrão: gank, ward, CS, roam, farm, poke, engage, peel, stun."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send().await
        .map_err(|e| format!("Groq network error: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let err = response.text().await.unwrap_or_default();
        return Err(format!("Groq API error ({}): {}", status, err));
    }

    let json_res: serde_json::Value = response.json().await
        .map_err(|e| e.to_string())?;

    if json_res["choices"][0]["finish_reason"].as_str() == Some("length") {
        return Err("tokens_esgotados".to_string());
    }

    let content = json_res["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "Sem conteúdo na resposta Groq".to_string())?
        .trim()
        .to_string();

    Ok(content)
}
