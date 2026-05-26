use futures_util::StreamExt;
use tauri::{Emitter, Manager};

// ── Shared HTTP client (reuses TLS connections) ───────────────────────────────
static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .expect("failed to build HTTP client")
    })
}

// ── Audio source types ────────────────────────────────────────────────────────
pub enum PlaySource {
    Samples(Vec<f32>),
    File(String),
    Bytes(Vec<u8>),
    /// Streaming: receive f32 sample chunks via channel. Send None to signal end.
    SampleStream(std::sync::mpsc::Receiver<Option<Vec<f32>>>),
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
            let mut stream_handle: Option<(rodio::OutputStream, rodio::OutputStreamHandle)> =
                match rodio::OutputStream::try_default() {
                    Ok(h) => { set_status("ok"); Some(h) }
                    Err(e) => { set_status(&format!("erro ao abrir dispositivo: {}", e)); None }
                };

            while let Ok(req) = rx.recv() {
                // Stop any currently playing audio
                if let Some(s) = sink_clone.lock().unwrap().take() { s.stop(); }

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
                let new_sink = match rodio::Sink::try_new(handle) {
                    Err(e) => {
                        set_status(&format!("erro ao criar sink: {}", e));
                        stream_handle = None;
                        let _ = req.done_tx.send(());
                        continue;
                    }
                    Ok(s) => s,
                };

                new_sink.set_volume(req.volume);
                new_sink.set_speed(req.speed);

                match req.source {
                    // ── Streaming: feed chunks as they arrive ─────────────────
                    PlaySource::SampleStream(chunk_rx) => {
                        // Store sink immediately so stop() can interrupt streaming
                        *sink_clone.lock().unwrap() = Some(new_sink);

                        while let Ok(maybe_samples) = chunk_rx.recv() {
                            let samples = match maybe_samples {
                                Some(s) if !s.is_empty() => s,
                                _ => break,
                            };
                            match sink_clone.lock().unwrap().as_ref() {
                                Some(sink) => sink.append(
                                    rodio::buffer::SamplesBuffer::new(1, 24000, samples)
                                ),
                                None => break, // stopped externally
                            }
                        }

                        // Drain remaining buffered audio
                        loop {
                            std::thread::sleep(std::time::Duration::from_millis(50));
                            let done = sink_clone.lock().unwrap()
                                .as_ref().map(|s| s.empty()).unwrap_or(true);
                            if done { break; }
                        }
                        sink_clone.lock().unwrap().take();
                        let _ = req.done_tx.send(());
                    }

                    // ── Static sources ────────────────────────────────────────
                    other => {
                        let appended = match other {
                            PlaySource::Samples(samples) => {
                                new_sink.append(rodio::buffer::SamplesBuffer::new(1, 24000, samples));
                                true
                            }
                            PlaySource::File(path) => {
                                match std::fs::File::open(&path) {
                                    Ok(f) => match rodio::Decoder::new(std::io::BufReader::new(f)) {
                                        Ok(src) => { new_sink.append(src); true }
                                        Err(e) => { set_status(&format!("erro ao decodificar WAV: {}", e)); false }
                                    }
                                    Err(e) => { set_status(&format!("erro ao abrir WAV: {}", e)); false }
                                }
                            }
                            PlaySource::Bytes(bytes) => {
                                match rodio::Decoder::new(std::io::Cursor::new(bytes)) {
                                    Ok(src) => { new_sink.append(src); true }
                                    Err(e) => { set_status(&format!("erro ao decodificar bytes: {}", e)); false }
                                }
                            }
                            PlaySource::SampleStream(_) => unreachable!(),
                        };

                        if appended {
                            *sink_clone.lock().unwrap() = Some(new_sink);
                            loop {
                                std::thread::sleep(std::time::Duration::from_millis(50));
                                let done = sink_clone.lock().unwrap()
                                    .as_ref().map(|s| s.empty()).unwrap_or(true);
                                if done { break; }
                            }
                            sink_clone.lock().unwrap().take();
                        }
                        let _ = req.done_tx.send(());
                    }
                }
            }
        });

        Self { sender: std::sync::Mutex::new(tx), sink, audio_status }
    }

    pub fn play_source(
        &self,
        source: PlaySource,
        volume: f32,
        speed: f32,
    ) -> Result<std::sync::mpsc::Receiver<()>, String> {
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(1);
        self.sender.lock().unwrap()
            .send(PlayRequest { source, volume, speed, done_tx })
            .map_err(|e| format!("Falha ao despachar áudio: {}", e))?;
        Ok(done_rx)
    }

    /// Starts a streaming playback session. Returns a sender for f32 sample
    /// chunks and a receiver that signals when playback is complete.
    /// Send `None` through the sender to signal end of stream.
    pub fn start_sample_stream(
        &self,
        volume: f32,
        speed: f32,
    ) -> Result<(std::sync::mpsc::Sender<Option<Vec<f32>>>, std::sync::mpsc::Receiver<()>), String> {
        let (chunk_tx, chunk_rx) = std::sync::mpsc::channel::<Option<Vec<f32>>>();
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(1);
        self.sender.lock().unwrap()
            .send(PlayRequest {
                source: PlaySource::SampleStream(chunk_rx),
                volume,
                speed,
                done_tx,
            })
            .map_err(|e| format!("Falha ao iniciar stream de áudio: {}", e))?;
        Ok((chunk_tx, done_rx))
    }

    pub fn stop(&self) {
        if let Some(sink) = self.sink.lock().unwrap().take() { sink.stop(); }
    }
}

// ── PCM conversion ────────────────────────────────────────────────────────────

fn s16le_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes.chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32_767.0)
        .collect()
}

// ── Voice parameter helpers ───────────────────────────────────────────────────

struct VoiceParams {
    phrase_key: String,
    api_voice: String,
    lang: String,
    text: String,
}

fn resolve_voice_params(raw_text: &str, voice: &str, speed: f32) -> VoiceParams {
    let text = if raw_text.chars().count() > 65 {
        let t: String = raw_text.chars().take(65).collect();
        t.rfind(' ').map(|p| t[..p].to_string()).unwrap_or(t)
    } else {
        raw_text.to_string()
    };

    let api_voice = match voice.to_lowercase().as_str() {
        "francisca" => "pf_dora".to_string(),
        "antonio"   => "pm_alex".to_string(),
        _           => voice.to_string(),
    };

    let lang = match api_voice.chars().next() {
        Some('p') => "pt-br".to_string(),
        _         => String::new(),
    };

    let phrase_key = if lang.is_empty() {
        format!("{}:{:.2}:{}", api_voice, speed, text)
    } else {
        format!("{}:{}:{:.2}:{}", api_voice, lang, speed, text)
    };

    VoiceParams { phrase_key, api_voice, lang, text }
}

fn build_tts_payload(text: &str, api_voice: &str, speed: f32, lang: &str) -> serde_json::Value {
    let mut payload = serde_json::json!({ "text": text, "voice": api_voice, "speed": speed });
    if !lang.is_empty() { payload["lang"] = serde_json::json!(lang); }
    payload
}

async fn fetch_tts_bytes(text: &str, api_voice: &str, speed: f32, lang: &str) -> Result<Vec<u8>, String> {
    let response = http_client()
        .post("https://spell2014-riftsyncai.hf.space/tts")
        .json(&build_tts_payload(text, api_voice, speed, lang))
        .send()
        .await
        .map_err(|e| format!("Erro na chamada à API de voz: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API de voz retornou erro HTTP {}", response.status()));
    }

    response.bytes().await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Erro ao ler resposta da API: {}", e))
}

async fn save_to_disk_and_cache(
    wav_bytes: &[u8],
    phrase_key: &str,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    let pool = app.try_state::<crate::db::DbState>()
        .ok_or_else(|| "Banco de dados não inicializado.".to_string())?;
    let db = &pool.0;

    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let cache_dir = app_dir.join("voice_cache");
    if !cache_dir.exists() { let _ = std::fs::create_dir_all(&cache_dir); }

    let file_id = format!("voice_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let audio_path = cache_dir.join(format!("{}.wav", file_id));
    let audio_path_str = audio_path.to_string_lossy().to_string();

    std::fs::write(&audio_path, wav_bytes)
        .map_err(|e| format!("Erro ao salvar WAV em disco: {}", e))?;

    let db_res = sqlx::query(
        "INSERT OR REPLACE INTO voice_cache (phrase_key, audio_path, file_size, audio_format, expires_at)
         VALUES (?, ?, ?, 'wav', datetime('now', '+7 days'))"
    )
    .bind(phrase_key)
    .bind(&audio_path_str)
    .bind(wav_bytes.len() as i64)
    .execute(db)
    .await;

    match db_res {
        Ok(_) => {
            println!("[Voice Cache DB] Salvo (7 dias): '{}'", audio_path_str);
            cleanup_voice_cache(db, 80).await;
        }
        Err(e) => eprintln!("[Voice Cache DB] Erro ao salvar: {}", e),
    }

    Ok(audio_path_str)
}

async fn cleanup_voice_cache(db: &sqlx::Pool<sqlx::Sqlite>, max_files: i64) {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM voice_cache")
        .fetch_one(db).await.unwrap_or(0);
    if count <= max_files { return; }

    let to_delete = count - max_files;
    let old_files: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, audio_path FROM voice_cache ORDER BY created_at ASC LIMIT ?"
    ).bind(to_delete).fetch_all(db).await.unwrap_or_default();

    for (id, path) in &old_files {
        let _ = std::fs::remove_file(path);
        let _ = sqlx::query("DELETE FROM voice_cache WHERE id = ?")
            .bind(id).execute(db).await;
    }
    if to_delete > 0 {
        println!("[VoiceCache] Limpeza: {} arquivo(s) antigo(s) removido(s).", to_delete);
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub(crate) async fn get_kokoro_status(_app: tauri::AppHandle) -> Result<String, String> {
    match http_client()
        .get("https://spell2014-riftsyncai.hf.space/health")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
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

/// Pré-gera e armazena em cache o áudio sem reproduzir.
#[tauri::command]
pub(crate) async fn prefetch_voice(
    text: String,
    voice: String,
    speed: f32,
    app: tauri::AppHandle,
) -> Result<(), String> {
    ensure_audio_cached(text, voice, speed, &app).await?;
    Ok(())
}

async fn ensure_audio_cached(
    text: String,
    voice: String,
    speed: f32,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    let p = resolve_voice_params(&text, &voice, speed);

    let pool = app.try_state::<crate::db::DbState>()
        .ok_or_else(|| "Banco de dados não inicializado.".to_string())?;
    let db = &pool.0;

    let cached: Option<(String, String)> = sqlx::query_as(
        "SELECT id, audio_path FROM voice_cache WHERE phrase_key = ? AND expires_at > datetime('now')"
    )
    .bind(&p.phrase_key)
    .fetch_optional(db)
    .await
    .map_err(|e| format!("Erro ao consultar cache: {}", e))?;

    if let Some((_, path)) = cached {
        if std::path::Path::new(&path).exists() {
            println!("[Voice Cache DB] HIT: '{}'", path);
            return Ok(path);
        }
        let _ = sqlx::query("DELETE FROM voice_cache WHERE phrase_key = ?")
            .bind(&p.phrase_key).execute(db).await;
    }

    println!("[Voice] MISS — gerando: '{}'", p.text);
    let start = std::time::Instant::now();
    let bytes = fetch_tts_bytes(&p.text, &p.api_voice, speed, &p.lang).await?;
    println!("[Voice] Gerado em {:?} ({} bytes)", start.elapsed(), bytes.len());

    save_to_disk_and_cache(&bytes, &p.phrase_key, app).await
}

#[tauri::command]
pub(crate) async fn play_voice(
    text: String,
    voice: String,
    volume: f32,
    speed: f32,
    app: tauri::AppHandle,
) -> Result<(), String> {
    println!("[play_voice] voice: '{}', volume: {}, speed: {}", voice, volume, speed);

    let p = resolve_voice_params(&text, &voice, speed);

    let pool = app.try_state::<crate::db::DbState>()
        .ok_or_else(|| "Banco de dados não inicializado.".to_string())?;
    let db = &pool.0;

    let player = app.try_state::<VoicePlayer>()
        .ok_or_else(|| "Motor de áudio não inicializado.".to_string())?;

    // ── Cache hit: play from disk instantly ───────────────────────────────────
    let cached: Option<(String, String)> = sqlx::query_as(
        "SELECT id, audio_path FROM voice_cache WHERE phrase_key = ? AND expires_at > datetime('now')"
    )
    .bind(&p.phrase_key)
    .fetch_optional(db)
    .await
    .unwrap_or(None);

    if let Some((_, path)) = cached.filter(|(_, path)| std::path::Path::new(path).exists()) {
        println!("[Voice Cache DB] HIT: '{}'", path);
        let _ = app.emit("voice-playback-started", serde_json::json!({}));
        let done_rx = player.inner().play_source(PlaySource::File(path), volume, 1.0)?;
        tokio::task::spawn_blocking(move || { let _ = done_rx.recv(); })
            .await.map_err(|e| e.to_string())?;
        return Ok(());
    }

    // ── Cache miss: stream from API, play as chunks arrive ────────────────────
    println!("[Voice] MISS — streaming: '{}'", p.text);
    let start = std::time::Instant::now();

    let response = http_client()
        .post("https://spell2014-riftsyncai.hf.space/tts/stream")
        .json(&build_tts_payload(&p.text, &p.api_voice, speed, &p.lang))
        .send()
        .await
        .map_err(|e| format!("Erro na chamada à API de voz: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API de voz retornou HTTP {}", response.status()));
    }

    // Start audio thread streaming session
    let (chunk_tx, done_rx) = player.inner().start_sample_stream(volume, 1.0)?;
    let _ = app.emit("voice-playback-started", serde_json::json!({}));

    // Read HTTP stream, skip WAV header, convert s16le → f32 → audio thread
    let mut body = response.bytes_stream();
    let mut header_buf = Vec::<u8>::with_capacity(44);
    let mut header_done = false;
    let mut all_bytes = Vec::<u8>::new();
    // Carry-over when an HTTP chunk boundary splits a 2-byte PCM sample.
    // Without this, chunks_exact(2) drops the odd byte and the next chunk
    // starts misaligned, producing clicks and pops on every chunk boundary.
    let mut odd_byte: Option<u8> = None;

    while let Some(result) = body.next().await {
        let chunk = result.map_err(|e| format!("Erro no stream de áudio: {}", e))?;
        all_bytes.extend_from_slice(&chunk);

        let pcm: Option<Vec<u8>> = if !header_done {
            header_buf.extend_from_slice(&chunk);
            if header_buf.len() >= 44 {
                header_done = true;
                if header_buf.len() > 44 {
                    Some(header_buf[44..].to_vec())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            Some(chunk.to_vec())
        };

        if let Some(bytes) = pcm {
            // Prepend carry-over byte from previous chunk to restore alignment
            let mut aligned = if let Some(lo) = odd_byte.take() {
                let mut b = Vec::with_capacity(bytes.len() + 1);
                b.push(lo);
                b.extend_from_slice(&bytes);
                b
            } else {
                bytes
            };

            // If still odd, save last byte for next chunk instead of dropping it
            if aligned.len() % 2 != 0 {
                odd_byte = aligned.pop();
            }

            if !aligned.is_empty() {
                let samples = s16le_to_f32(&aligned);
                if chunk_tx.send(Some(samples)).is_err() {
                    break; // audio thread stopped (e.g. stop_voice called)
                }
            }
        }
    }

    // Signal end of stream (dropping chunk_tx also works, but explicit is clearer)
    let _ = chunk_tx.send(None);
    println!("[Voice] Stream completo em {:?} ({} bytes)", start.elapsed(), all_bytes.len());

    // Save full WAV to disk cache in background
    let phrase_key = p.phrase_key.clone();
    let app_clone = app.clone();
    tokio::spawn(async move {
        if let Err(e) = save_to_disk_and_cache(&all_bytes, &phrase_key, &app_clone).await {
            eprintln!("[Voice Cache] Erro ao salvar em background: {}", e);
        }
    });

    // Wait for audio playback to finish
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
        Ok(player.audio_status.lock().unwrap().clone())
    } else {
        Ok("VoicePlayer não inicializado".to_string())
    }
}

#[tauri::command]
pub(crate) async fn test_audio_output() -> Result<String, String> {
    tokio::task::spawn_blocking(|| -> Result<String, String> {
        let (stream, handle) = rodio::OutputStream::try_default()
            .map_err(|e| format!("Falha ao abrir dispositivo de áudio: {}", e))?;

        let sink = rodio::Sink::try_new(&handle)
            .map_err(|e| format!("Falha ao criar Sink de áudio: {}", e))?;

        let sample_rate = 24000u32;
        let duration_samples = (sample_rate as f32 * 0.5) as usize;
        let samples: Vec<f32> = (0..duration_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.3 * (1.0 - t / 0.5)
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
    .map_err(|e| format!("Erro interno de thread: {}", e))?
}
