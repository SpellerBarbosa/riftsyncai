use tauri::Manager;

pub enum PlaySource {
    Samples(Vec<f32>),
    File(String),
}

struct PlayRequest {
    source: PlaySource,
    volume: f32,
    speed: f32,
    done_tx: std::sync::mpsc::SyncSender<()>,
}

pub struct VoicePlayer {
    sender: std::sync::Mutex<std::sync::mpsc::Sender<PlayRequest>>,
    sink: std::sync::Arc<std::sync::Mutex<Option<rodio::Sink>>>,
    pub audio_status: std::sync::Arc<std::sync::Mutex<String>>,
}

impl VoicePlayer {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<PlayRequest>();
        let sink = std::sync::Arc::new(std::sync::Mutex::new(None::<rodio::Sink>));
        let sink_clone = sink.clone();
        let audio_status = std::sync::Arc::new(std::sync::Mutex::new("inicializando".to_string()));
        let status_clone = audio_status.clone();

        std::thread::spawn(move || {
            let set_status = |msg: &str| {
                if let Ok(mut s) = status_clone.lock() { *s = msg.to_string(); }
            };

            set_status("abrindo dispositivo...");
            let mut stream_handle: Option<(rodio::OutputStream, rodio::OutputStreamHandle)> = match rodio::OutputStream::try_default() {
                Ok(h) => { set_status("ok"); Some(h) }
                Err(e) => { set_status(&format!("erro ao abrir dispositivo: {}", e)); None }
            };

            while let Ok(req) = rx.recv() {
                {
                    let mut active_sink = sink_clone.lock().unwrap();
                    if let Some(s) = active_sink.take() { s.stop(); }
                }

                if stream_handle.is_none() {
                    stream_handle = match rodio::OutputStream::try_default() {
                        Ok(h) => { set_status("ok"); Some(h) }
                        Err(e) => {
                            set_status(&format!("erro ao reinicializar dispositivo: {}", e));
                            let _ = req.done_tx.send(());
                            continue;
                        }
                    };
                }

                let handle = &stream_handle.as_ref().unwrap().1;
                match rodio::Sink::try_new(handle) {
                    Err(e) => {
                        set_status(&format!("erro ao criar sink: {}", e));
                        stream_handle = None;
                        let _ = req.done_tx.send(());
                    }
                    Ok(new_sink) => {
                        new_sink.set_volume(req.volume);
                        new_sink.set_speed(req.speed);

                        let appended = match req.source {
                            PlaySource::Samples(samples) => {
                                let source = rodio::buffer::SamplesBuffer::new(1, 24000, samples);
                                new_sink.append(source);
                                true
                            }
                            PlaySource::File(path_str) => {
                                match std::fs::File::open(&path_str) {
                                    Ok(file) => match rodio::Decoder::new(std::io::BufReader::new(file)) {
                                        Ok(source) => { new_sink.append(source); true }
                                        Err(e) => { set_status(&format!("erro ao decodificar WAV: {}", e)); false }
                                    }
                                    Err(e) => { set_status(&format!("erro ao abrir WAV: {}", e)); false }
                                }
                            }
                        };

                        if appended {
                            { *sink_clone.lock().unwrap() = Some(new_sink); }
                            loop {
                                std::thread::sleep(std::time::Duration::from_millis(50));
                                let is_done = {
                                    let guard = sink_clone.lock().unwrap();
                                    guard.as_ref().map(|s| s.empty()).unwrap_or(true)
                                };
                                if is_done { break; }
                            }
                            sink_clone.lock().unwrap().take();
                        }

                        let _ = req.done_tx.send(());
                    }
                }
            }
        });

        Self {
            sender: std::sync::Mutex::new(tx),
            sink,
            audio_status,
        }
    }

    pub fn play_source(&self, source: PlaySource, volume: f32, speed: f32) -> Result<std::sync::mpsc::Receiver<()>, String> {
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(1);
        let req = PlayRequest { source, volume, speed, done_tx };
        let sender = self.sender.lock().unwrap();
        sender.send(req).map_err(|e| format!("Falha ao despachar áudio: {}", e))?;
        Ok(done_rx)
    }

    pub fn stop(&self) {
        let mut active_sink = self.sink.lock().unwrap();
        if let Some(sink) = active_sink.take() {
            sink.stop();
        }
    }
}

/// Verifica se a API remota de TTS está disponível e com modelo carregado.
#[tauri::command]
pub(crate) async fn get_kokoro_status(_app: tauri::AppHandle) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Erro ao criar cliente HTTP: {}", e))?;

    match client.get("https://spell2014-riftsyncai.hf.space/health").send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(body) if body["status"] == "ok" && body["model_loaded"] == true => Ok("ready".to_string()),
                Ok(body) => Err(format!("API indisponível: {:?}", body)),
                Err(e) => Err(format!("Resposta inválida do health check: {}", e)),
            }
        }
        Ok(resp) => Err(format!("Health check retornou HTTP {}", resp.status())),
        Err(e) => Err(format!("Falha ao conectar na API de voz: {}", e)),
    }
}

async fn cleanup_voice_cache(db: &sqlx::Pool<sqlx::Sqlite>, max_files: i64) {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM voice_cache")
        .fetch_one(db).await.unwrap_or(0);
    if count <= max_files { return; }

    let to_delete = count - max_files;
    let old_files: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, audio_path FROM voice_cache ORDER BY created_at ASC LIMIT ?"
    ).bind(to_delete).fetch_all(db).await.unwrap_or_default();

    for (id, path) in old_files {
        let _ = std::fs::remove_file(&path);
        let _ = sqlx::query("DELETE FROM voice_cache WHERE id = ?")
            .bind(&id).execute(db).await;
    }
    if to_delete > 0 {
        println!("[VoiceCache] Limpeza: {} arquivo(s) antigo(s) removido(s).", to_delete);
    }
}

#[tauri::command]
pub(crate) async fn play_voice(
    text: String,
    voice: String,
    volume: f32,
    speed: f32,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let text = if text.chars().count() > 100 {
        let truncated: String = text.chars().take(100).collect();
        match truncated.rfind(' ') {
            Some(pos) => truncated[..pos].to_string(),
            None => truncated,
        }
    } else {
        text
    };

    println!("[play_voice] text: '{}', voice: '{}', volume: {}, speed: {}", text, voice, volume, speed);

    let api_voice = if voice.to_lowercase() == "francisca" {
        "pf_dora".to_string()
    } else if voice.to_lowercase() == "antonio" {
        "pm_alex".to_string()
    } else {
        voice.clone()
    };

    let phrase_key = format!("{}:{:.2}:{}", api_voice, speed, text);

    let pool = app.try_state::<crate::db::DbState>()
        .ok_or_else(|| "Banco de dados não inicializado.".to_string())?;
    let db = &pool.0;

    let cached_entry: Option<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT id, audio_path FROM voice_cache WHERE phrase_key = ? AND expires_at > datetime('now')"
    )
    .bind(&phrase_key)
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Erro ao consultar cache de voz: {}", e))?;

    let player = app.try_state::<VoicePlayer>()
        .ok_or_else(|| "Motor de áudio não inicializado.".to_string())?;

    if let Some((_id, audio_path_str)) = cached_entry {
        let path = std::path::Path::new(&audio_path_str);
        if path.exists() {
            println!("[Voice Cache DB] HIT: '{}'", audio_path_str);
            let done_rx = player.inner().play_source(PlaySource::File(audio_path_str), volume, 1.0)?;
            tokio::task::spawn_blocking(move || { let _ = done_rx.recv(); })
                .await.map_err(|e| e.to_string())?;
            return Ok(());
        }
        let _ = sqlx::query("DELETE FROM voice_cache WHERE phrase_key = ?").bind(&phrase_key).execute(db).await;
    }

    println!("[Voice] MISS. Chamando API remota spell-tts-api...");
    let start = std::time::Instant::now();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| format!("Erro ao criar cliente HTTP: {}", e))?;

    let lang = if api_voice.starts_with('p') { "pt-br" } else { "" };

    let mut payload = serde_json::json!({
        "text": text,
        "voice": api_voice,
        "speed": speed
    });
    if !lang.is_empty() {
        payload["lang"] = serde_json::json!(lang);
    }

    let response = client
        .post("https://spell2014-riftsyncai.hf.space/tts")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Erro na chamada à API de voz: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API de voz retornou erro HTTP {}", response.status()));
    }

    let wav_bytes = response.bytes()
        .await
        .map_err(|e| format!("Erro ao ler resposta da API: {}", e))?;

    println!("[Voice] API respondeu em {:?} ({} bytes)", start.elapsed(), wav_bytes.len());

    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let cache_dir = app_dir.join("voice_cache");
    if !cache_dir.exists() {
        let _ = std::fs::create_dir_all(&cache_dir);
    }

    let file_id = format!("voice_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let audio_file_path = cache_dir.join(format!("{}.wav", file_id));
    let audio_path_str = audio_file_path.to_string_lossy().to_string();

    std::fs::write(&audio_file_path, &wav_bytes)
        .map_err(|e| format!("Erro ao salvar WAV em disco: {}", e))?;

    let file_size = wav_bytes.len() as i64;
    let db_res = sqlx::query(
        "INSERT OR REPLACE INTO voice_cache (phrase_key, audio_path, file_size, audio_format, expires_at)
         VALUES (?, ?, ?, 'wav', datetime('now', '+7 days'))"
    )
    .bind(&phrase_key)
    .bind(&audio_path_str)
    .bind(file_size)
    .execute(db)
    .await;

    if db_res.is_ok() {
        println!("[Voice Cache DB] Salvo (7 dias): '{}'", audio_path_str);
        cleanup_voice_cache(db, 80).await;
    } else if let Err(e) = db_res {
        eprintln!("[Voice Cache DB] Erro ao registrar no SQLite: {}", e);
    }

    let done_rx = player.inner().play_source(PlaySource::File(audio_path_str), volume, 1.0)?;
    tokio::task::spawn_blocking(move || { let _ = done_rx.recv(); })
        .await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub(crate) async fn stop_voice(app: tauri::AppHandle) -> Result<(), String> {
    println!("[stop_voice] Chamado");
    if let Some(player) = app.try_state::<VoicePlayer>() {
        player.inner().stop();
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_audio_status(app: tauri::AppHandle) -> Result<String, String> {
    if let Some(player) = app.try_state::<VoicePlayer>() {
        let status = player.audio_status.lock().unwrap().clone();
        Ok(status)
    } else {
        Ok("VoicePlayer não inicializado".to_string())
    }
}

#[tauri::command]
pub(crate) async fn test_audio_output() -> Result<String, String> {
    use std::f32::consts::PI;

    let result = tokio::task::spawn_blocking(|| -> Result<String, String> {
        let (stream, handle) = rodio::OutputStream::try_default()
            .map_err(|e| format!("Falha ao abrir dispositivo de áudio: {}", e))?;

        let sink = rodio::Sink::try_new(&handle)
            .map_err(|e| format!("Falha ao criar Sink de áudio: {}", e))?;

        let sample_rate = 24000u32;
        let duration_samples = (sample_rate as f32 * 0.5) as usize;
        let freq = 440.0f32;
        let samples: Vec<f32> = (0..duration_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let env = 1.0 - (t / 0.5);
                (2.0 * PI * freq * t).sin() * 0.3 * env
            })
            .collect();

        let source = rodio::buffer::SamplesBuffer::new(1, sample_rate, samples);
        sink.set_volume(0.8);
        sink.append(source);
        sink.sleep_until_end();

        drop(stream);
        Ok("OK: Dispositivo de áudio padrão funcionando".to_string())
    })
    .await
    .map_err(|e| format!("Erro interno de thread: {}", e))?;

    result
}
