use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::Manager;
use sqlx::{Pool, Sqlite};

const DEFAULT_OPENROUTER_MODEL: &str = "deepseek/deepseek-v4-flash:free";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenRouterSettings {
    pub enabled: bool,
    pub api_key: String,
    pub model: String,
}

#[tauri::command]
pub async fn get_openrouter_settings(
    app: tauri::AppHandle,
) -> Result<OpenRouterSettings, String> {
    let state = match app.try_state::<crate::db::DbState>() {
        Some(s) => s,
        None => {
            return Ok(OpenRouterSettings {
                enabled: true,
                api_key: String::new(),
                model: DEFAULT_OPENROUTER_MODEL.to_string(),
            });
        }
    };
    let pool = &state.0;
    
    let enabled_str: Option<String> = sqlx::query_scalar("SELECT value FROM sync_metadata WHERE key = 'enable_openrouter'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    let enabled = enabled_str.unwrap_or_else(|| "true".to_string()) == "true";

    let api_key: Option<String> = sqlx::query_scalar("SELECT value FROM sync_metadata WHERE key = 'openrouter_key'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    let mut api_key = api_key.unwrap_or_default();
    if api_key.trim().is_empty() {
        if let Ok(env_key) = std::env::var("OPENROUTER_API_KEY") {
            api_key = env_key;
        }
    }

    let model: Option<String> = sqlx::query_scalar("SELECT value FROM sync_metadata WHERE key = 'openrouter_model'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    let model = model.unwrap_or_else(|| DEFAULT_OPENROUTER_MODEL.to_string());

    Ok(OpenRouterSettings { enabled, api_key, model })
}

#[tauri::command]
pub async fn set_openrouter_settings(
    app: tauri::AppHandle,
    enabled: bool,
    api_key: String,
    model: String,
) -> Result<(), String> {
    let state = match app.try_state::<crate::db::DbState>() {
        Some(s) => s,
        None => return Err("Banco de dados ainda não está pronto.".to_string()),
    };
    let pool = &state.0;
    
    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('enable_openrouter', ?)")
        .bind(if enabled { "true" } else { "false" })
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('openrouter_key', ?)")
        .bind(&api_key)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('openrouter_model', ?)")
        .bind(&model)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn test_openrouter_connection(
    api_key: String,
    model: String,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    
    let response = client.post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("HTTP-Referer", "https://github.com/spellcoachia")
        .header("X-Title", "Spell Coach IA")
        .json(&json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": "Olá! Responda apenas com a palavra 'Conectado!'"
                }
            ],
            "max_tokens": 10
        }))
        .send()
        .await
        .map_err(|e| format!("Erro ao enviar requisição: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("Erro da API OpenRouter (Status {}): {}", status, err_text));
    }

    let json_res: serde_json::Value = response.json()
        .await
        .map_err(|e| format!("Erro ao decodificar JSON: {}", e))?;

    let text = json_res["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("Nenhuma resposta recebida.")
        .trim()
        .to_string();

    Ok(text)
}

pub async fn get_openrouter_tip_internal(
    pool: &Pool<Sqlite>,
    prompt: String,
) -> Result<String, String> {
    let enabled_str: Option<String> = sqlx::query_scalar("SELECT value FROM sync_metadata WHERE key = 'enable_openrouter'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    if enabled_str.unwrap_or_else(|| "true".to_string()) != "true" {
        return Err("OpenRouter desativado".to_string());
    }

    let api_key: Option<String> = sqlx::query_scalar("SELECT value FROM sync_metadata WHERE key = 'openrouter_key'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    let api_key = match api_key {
        Some(k) if !k.trim().is_empty() => k,
        _ => {
            if let Ok(env_key) = std::env::var("OPENROUTER_API_KEY") {
                if !env_key.trim().is_empty() {
                    env_key
                } else {
                    return Err("Chave de API OpenRouter vazia".to_string());
                }
            } else {
                return Err("Chave de API OpenRouter vazia".to_string());
            }
        }
    };

    let model: Option<String> = sqlx::query_scalar("SELECT value FROM sync_metadata WHERE key = 'openrouter_model'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    let model = model.unwrap_or_else(|| DEFAULT_OPENROUTER_MODEL.to_string());

    let client = reqwest::Client::new();
    let response = client.post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("HTTP-Referer", "https://github.com/spellcoachia")
        .header("X-Title", "Spell Coach IA")
        .json(&json!({
            "model": model,
            "max_tokens": 120,
            "temperature": 0.2,
            "messages": [
                {
                    "role": "system",
                    "content": "Você é o Spell Coach IA — coach de League of Legends. REGRAS ABSOLUTAS: (1) Responda APENAS com o JSON solicitado, sem nenhum texto antes ou depois. (2) Use EXCLUSIVAMENTE as habilidades listadas no contexto — nunca invente efeitos. (3) Complete todos os campos. (4) MÁXIMO 10 PALAVRAS por valor de campo — seja extremamente conciso."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    if !status.is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("OpenRouter error (Status {}): {}", status, err_text));
    }

    let json_res: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    // Se finish_reason == "length", a resposta foi cortada no meio → fallback local é mais seguro
    let finish_reason = json_res["choices"][0]["finish_reason"].as_str().unwrap_or("stop");
    if finish_reason == "length" {
        return Err("tokens_esgotados".to_string());
    }

    let content = json_res["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "Sem conteúdo na resposta".to_string())?
        .trim()
        .to_string();

    Ok(content)
}
