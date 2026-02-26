// SYNOID Editor API — Full REST backend for the React NLE editor
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response, Sse},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{fs as tfs, process::Command};
use tokio_stream::StreamExt as _;
use tracing::{error, info, warn};
use uuid::Uuid;

// ─── Shared state for session tracking ────────────────────────────────────────
#[derive(Debug, Clone, Serialize)]
pub struct SessionState {
    pub id: String,
    pub created_at: u64,
    pub asset_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetMeta {
    pub id: String,
    pub session_id: String,
    pub filename: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub size: u64,
    pub thumbnail_url: Option<String>,
    pub stream_url: String,
    pub fps: f64,
}

#[derive(Debug, Default)]
pub struct RenderJob {
    pub progress: f32,
    pub status: String,
    pub output_path: Option<PathBuf>,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct EditorStore {
    pub sessions: HashMap<String, SessionState>,
    pub assets: HashMap<String, Vec<AssetMeta>>,   // session_id → assets
    pub jobs: HashMap<String, RenderJob>,           // session_id → render job
}

pub type SharedEditorStore = Arc<Mutex<EditorStore>>;

// ─── Request/Response types ───────────────────────────────────────────────────
#[derive(Deserialize)]
pub struct TranscribeRequest {
    #[serde(rename = "assetId")]
    pub asset_id: String,
}

#[derive(Deserialize)]
pub struct AiChatRequest {
    pub message: String,
}

#[derive(Deserialize)]
pub struct AutoEditRequest {
    pub intent: String,
    #[serde(rename = "assetId")]
    pub asset_id: Option<String>,
    #[serde(rename = "outputPath")]
    pub output_path: Option<String>,
}

#[derive(Deserialize)]
pub struct RenderRequest {
    pub intent: Option<String>,
    #[serde(rename = "assetId")]
    pub asset_id: Option<String>,
    pub clips: Option<Value>,
    #[serde(rename = "captionData")]
    pub caption_data: Option<Value>,
}

// ─── App state ────────────────────────────────────────────────────────────────
#[derive(Clone)]
pub struct EditorState {
    pub store: SharedEditorStore,
    pub core: Arc<crate::agent::core::AgentCore>,
}

// ─── Router Factory ──────────────────────────────────────────────────────────
pub fn router(core: Arc<crate::agent::core::AgentCore>) -> Router {
    let state = EditorState {
        store: Arc::new(Mutex::new(EditorStore::default())),
        core,
    };

    Router::new()
        .route("/sessions", post(create_session))
        .route("/sessions/:id", get(get_session))
        .route("/sessions/:id/assets", post(upload_asset).get(list_assets))
        .route("/sessions/:id/assets/:asset_id", delete(delete_asset))
        .route("/sessions/:id/assets/:asset_id/stream", get(stream_asset))
        .route("/sessions/:id/assets/:asset_id/thumbnail", get(get_thumbnail))
        .route("/sessions/:id/transcribe", post(transcribe_asset))
        .route("/sessions/:id/ai/chat", post(ai_chat))
        .route("/sessions/:id/ai/auto-edit", post(ai_auto_edit))
        .route("/sessions/:id/render", post(start_render))
        .route("/sessions/:id/render/status", get(render_status))
        .route("/sessions/:id/project/save", post(save_project))
        .route("/sessions/:id/project/load", get(load_project))
        .with_state(state)
}

// ─── Session Handlers ─────────────────────────────────────────────────────────
async fn create_session(State(s): State<EditorState>) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let asset_dir = PathBuf::from("cortex_cache").join("editor_sessions").join(&id).join("assets");
    let _ = tfs::create_dir_all(&asset_dir).await;

    let session = SessionState { id: id.clone(), created_at: now, asset_dir };
    {
        let mut store = s.store.lock().unwrap();
        store.sessions.insert(id.clone(), session);
    }

    info!("[EDITOR-API] Created session {}", id);
    Json(json!({ "id": id, "status": "active" }))
}

async fn get_session(
    Path(id): Path<String>,
    State(s): State<EditorState>,
) -> impl IntoResponse {
    let store = s.store.lock().unwrap();
    if store.sessions.contains_key(&id) {
        Json(json!({ "id": id, "status": "active" })).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

// ─── Asset Handlers ───────────────────────────────────────────────────────────
async fn upload_asset(
    Path(session_id): Path<String>,
    State(s): State<EditorState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let asset_dir = {
        let store = s.store.lock().unwrap();
        match store.sessions.get(&session_id) {
            Some(sess) => sess.asset_dir.clone(),
            None => return (StatusCode::NOT_FOUND, "Session not found").into_response(),
        }
    };

    let _ = tfs::create_dir_all(&asset_dir).await;

    while let Ok(Some(field)) = multipart.next_field().await {
        let filename = field.file_name().unwrap_or("upload").to_string();
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(e) => {
                error!("[EDITOR-API] Upload read error: {}", e);
                return (StatusCode::BAD_REQUEST, "Failed to read upload").into_response();
            }
        };

        let asset_id = Uuid::new_v4().to_string();
        let safe_name = sanitize_filename(&filename);
        let file_path = asset_dir.join(format!("{}_{}", asset_id, safe_name));
        let size = data.len() as u64;

        if let Err(e) = tfs::write(&file_path, &data).await {
            error!("[EDITOR-API] Failed to write asset: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        info!("[EDITOR-API] Saved asset {} → {:?}", asset_id, file_path);

        // Probe video metadata with ffprobe
        let (duration, width, height, fps) = probe_video_meta(&file_path).await;

        // Extract thumbnail
        let thumb_path = asset_dir.join(format!("{}_thumb.jpg", asset_id));
        extract_thumbnail(&file_path, &thumb_path, 1.0).await;

        let kind = infer_asset_type(&filename);
        let stream_url = format!("/api/editor/sessions/{}/assets/{}/stream", session_id, asset_id);
        let thumbnail_url = if thumb_path.exists() {
            Some(format!("/api/editor/sessions/{}/assets/{}/thumbnail", session_id, asset_id))
        } else {
            None
        };

        let meta = AssetMeta {
            id: asset_id.clone(),
            session_id: session_id.clone(),
            filename: filename.clone(),
            kind,
            duration,
            width,
            height,
            size,
            fps,
            thumbnail_url,
            stream_url,
        };

        {
            let mut store = s.store.lock().unwrap();
            store.assets.entry(session_id.clone()).or_default().push(meta.clone());
        }

        return Json(json!({
            "id": meta.id,
            "type": meta.kind,
            "filename": meta.filename,
            "duration": meta.duration,
            "width": meta.width,
            "height": meta.height,
            "size": meta.size,
            "fps": meta.fps,
            "thumbnailUrl": meta.thumbnail_url,
            "streamUrl": meta.stream_url,
            "aiGenerated": false,
        })).into_response();
    }

    (StatusCode::BAD_REQUEST, "No file provided").into_response()
}

async fn list_assets(
    Path(session_id): Path<String>,
    State(s): State<EditorState>,
) -> impl IntoResponse {
    let store = s.store.lock().unwrap();
    let assets = store.assets.get(&session_id).cloned().unwrap_or_default();
    let json_assets: Vec<Value> = assets.iter().map(|m| json!({
        "id": m.id,
        "type": m.kind,
        "filename": m.filename,
        "duration": m.duration,
        "width": m.width,
        "height": m.height,
        "size": m.size,
        "thumbnailUrl": m.thumbnail_url,
        "streamUrl": m.stream_url,
        "aiGenerated": false,
    })).collect();
    Json(json_assets)
}

async fn delete_asset(
    Path((session_id, asset_id)): Path<(String, String)>,
    State(s): State<EditorState>,
) -> impl IntoResponse {
    let asset_dir = {
        let store = s.store.lock().unwrap();
        store.sessions.get(&session_id).map(|s| s.asset_dir.clone())
    };
    if let Some(dir) = asset_dir {
        // Try to delete all files with this asset_id prefix
        if let Ok(mut entries) = tfs::read_dir(&dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with(&asset_id) {
                    let _ = tfs::remove_file(entry.path()).await;
                }
            }
        }
        let mut store = s.store.lock().unwrap();
        if let Some(assets) = store.assets.get_mut(&session_id) {
            assets.retain(|a| a.id != asset_id);
        }
    }
    StatusCode::NO_CONTENT
}

async fn stream_asset(
    Path((session_id, asset_id)): Path<(String, String)>,
    State(s): State<EditorState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let file_path = find_asset_path(&s, &session_id, &asset_id).await;
    match file_path {
        Some(path) => {
            let content_type = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            serve_file_with_range(&path, &headers, &content_type).await.into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn get_thumbnail(
    Path((session_id, asset_id)): Path<(String, String)>,
    State(s): State<EditorState>,
) -> impl IntoResponse {
    let dir = {
        let store = s.store.lock().unwrap();
        store.sessions.get(&session_id).map(|sess| sess.asset_dir.clone())
    };
    if let Some(asset_dir) = dir {
        let thumb_path = asset_dir.join(format!("{}_thumb.jpg", asset_id));
        if thumb_path.exists() {
            if let Ok(bytes) = tfs::read(&thumb_path).await {
                return (
                    [(header::CONTENT_TYPE, "image/jpeg")],
                    bytes,
                ).into_response();
            }
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

// ─── Transcription ─────────────────────────────────────────────────────────────
async fn transcribe_asset(
    Path(session_id): Path<String>,
    State(s): State<EditorState>,
    Json(req): Json<TranscribeRequest>,
) -> impl IntoResponse {
    let file_path = find_asset_path(&s, &session_id, &req.asset_id).await;
    let file_path = match file_path {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Asset not found"}))).into_response(),
    };

    info!("[EDITOR-API] Transcribing asset {} in session {}", req.asset_id, session_id);

    // Extract audio to WAV for Whisper
    let wav_path = file_path.with_extension("_transcribe.wav");
    let extract_ok = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(&file_path)
        .args(["-ar", "16000", "-ac", "1", "-f", "wav"])
        .arg(&wav_path)
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);

    if !extract_ok {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Audio extraction failed"}))).into_response();
    }

    let engine = match crate::agent::transcription::TranscriptionEngine::new(None).await {
        Ok(e) => e,
        Err(e) => {
            error!("[EDITOR-API] Transcription engine init failed: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response();
        }
    };

    let segments = match engine.transcribe(&wav_path).await {
        Ok(s) => s,
        Err(e) => {
            let _ = tfs::remove_file(&wav_path).await;
            error!("[EDITOR-API] Transcription failed: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response();
        }
    };
    let _ = tfs::remove_file(&wav_path).await;

    // Build word-level approximation (distribute words evenly within each segment)
    let mut words = Vec::new();
    for seg in &segments {
        let seg_words: Vec<&str> = seg.text.trim().split_whitespace().collect();
        let n = seg_words.len().max(1);
        let dur = (seg.end - seg.start) / n as f64;
        for (i, word) in seg_words.iter().enumerate() {
            words.push(json!({
                "text": word,
                "start": seg.start + i as f64 * dur,
                "end": seg.start + (i + 1) as f64 * dur,
            }));
        }
    }

    let response = json!({
        "segments": segments.iter().map(|s| json!({
            "start": s.start,
            "end": s.end,
            "text": s.text,
        })).collect::<Vec<_>>(),
        "words": words,
    });

    Json(response).into_response()
}

// ─── AI Chat ──────────────────────────────────────────────────────────────────
async fn ai_chat(
    Path(session_id): Path<String>,
    State(s): State<EditorState>,
    Json(req): Json<AiChatRequest>,
) -> impl IntoResponse {
    info!("[EDITOR-API] AI chat in session {}: {}", session_id, req.message);
    let mut brain = s.core.brain.lock().await;
    match brain.process(&req.message).await {
        Ok(response) => Json(json!({
            "response": response,
            "actions": suggest_actions_from_response(&response),
        })).into_response(),
        Err(e) => {
            error!("[EDITOR-API] Brain error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response()
        }
    }
}

fn suggest_actions_from_response(response: &str) -> Vec<Value> {
    let lower = response.to_lowercase();
    let mut actions = Vec::new();
    if lower.contains("remov") || lower.contains("cut") || lower.contains("trim") {
        actions.push(json!({ "type": "auto-edit", "label": "Apply AI Edit", "params": { "intent": response } }));
    }
    if lower.contains("subtitle") || lower.contains("caption") || lower.contains("transcrib") {
        actions.push(json!({ "type": "transcribe", "label": "Transcribe Video", "params": {} }));
    }
    actions
}

// ─── AI Auto-Edit ─────────────────────────────────────────────────────────────
async fn ai_auto_edit(
    Path(session_id): Path<String>,
    State(s): State<EditorState>,
    Json(req): Json<AutoEditRequest>,
) -> impl IntoResponse {
    info!("[EDITOR-API] Auto-edit in session {}: {}", session_id, req.intent);

    let asset_id = req.asset_id.as_deref().unwrap_or("");
    let file_path = if asset_id.is_empty() {
        // Use the first asset in the session
        let store = s.store.lock().unwrap();
        store.assets.get(&session_id)
            .and_then(|a| a.first())
            .map(|a| {
                // Reconstruct path from session asset dir
                let dir = store.sessions.get(&session_id).unwrap().asset_dir.clone();
                // find file with asset_id prefix
                dir.join(format!("{}_{}", a.id, a.filename))
            })
    } else {
        find_asset_path(&s, &session_id, asset_id).await
    };

    let input = match file_path {
        Some(p) if p.exists() => p,
        _ => return (StatusCode::NOT_FOUND, Json(json!({"error": "Asset not found"}))).into_response(),
    };

    let output_name = req.output_path.unwrap_or_else(|| {
        format!("cortex_cache/editor_sessions/{}/ai_edit_output.mp4", session_id)
    });
    let output = PathBuf::from(&output_name);
    if let Some(parent) = output.parent() {
        let _ = tfs::create_dir_all(parent).await;
    }

    // Initialize job
    {
        let mut store = s.store.lock().unwrap();
        store.jobs.insert(session_id.clone(), RenderJob {
            progress: 0.0,
            status: "running".to_string(),
            output_path: None,
            error: None,
        });
    }

    let core = s.core.clone();
    let intent = req.intent.clone();
    let session_id_clone = session_id.clone();
    let store_clone = s.store.clone();
    let output_clone = output.clone();

    tokio::spawn(async move {
        let result = crate::agent::smart_editor::smart_edit(
            &input,
            &intent,
            &output_clone,
            false,
            Some(Box::new(move |msg: &str| {
                info!("[EDITOR-API] Edit progress: {}", msg);
            })),
            None,
            None,
            None,
        ).await;

        let mut store = store_clone.lock().unwrap();
        if let Some(job) = store.jobs.get_mut(&session_id_clone) {
            match result {
                Ok(_) => {
                    job.progress = 1.0;
                    job.status = "done".to_string();
                    job.output_path = Some(output_clone);
                }
                Err(e) => {
                    job.status = "error".to_string();
                    job.error = Some(e.to_string());
                }
            }
        }
    });

    Json(json!({
        "jobId": session_id,
        "status": "started",
        "outputPath": output_name,
    })).into_response()
}

// ─── Render ───────────────────────────────────────────────────────────────────
async fn start_render(
    Path(session_id): Path<String>,
    State(s): State<EditorState>,
    Json(req): Json<RenderRequest>,
) -> impl IntoResponse {
    let intent = req.intent.unwrap_or_default();
    let asset_id = req.asset_id.as_deref().unwrap_or("").to_string();

    // Find the input asset
    let file_path = if asset_id.is_empty() {
        let store = s.store.lock().unwrap();
        store.assets.get(&session_id)
            .and_then(|a| a.first())
            .and_then(|a| {
                let dir = store.sessions.get(&session_id)?.asset_dir.clone();
                // try to find file
                std::fs::read_dir(&dir).ok()?.filter_map(|e| e.ok()).find(|e| {
                    e.file_name().to_string_lossy().starts_with(&a.id)
                }).map(|e| e.path())
            })
    } else {
        find_asset_path(&s, &session_id, &asset_id).await
    };

    let input = match file_path {
        Some(p) if p.exists() => p,
        _ => return (StatusCode::BAD_REQUEST, Json(json!({"error": "No asset to render"}))).into_response(),
    };

    let output_path = PathBuf::from(format!(
        "cortex_cache/editor_sessions/{}/render_output.mp4", session_id
    ));
    if let Some(p) = output_path.parent() {
        let _ = tfs::create_dir_all(p).await;
    }

    {
        let mut store = s.store.lock().unwrap();
        store.jobs.insert(session_id.clone(), RenderJob {
            progress: 0.0,
            status: "rendering".to_string(),
            output_path: None,
            error: None,
        });
    }

    let core = s.core.clone();
    let store_clone = s.store.clone();
    let session_id_clone = session_id.clone();
    let output_clone = output_path.clone();

    tokio::spawn(async move {
        // If there's an intent, run smart_edit which handles both subtitle generation and editing
        if !intent.is_empty() {
            let _ = crate::agent::smart_editor::smart_edit(
                &input,
                &intent,
                &output_clone,
                false,
                None,
                None,
                None,
                None,
            ).await;
        } else {
            // Just copy-encode with subtitle burn-in if SRT exists
            let srt_path = input.with_extension("srt");
            let mut args = vec![
                "-y".to_string(),
                "-i".to_string(),
                input.to_string_lossy().to_string(),
            ];
            if srt_path.exists() {
                let srt_str = srt_path.to_string_lossy().to_string();
                // Escape colons on Windows paths for ffmpeg vf filter
                let safe_srt = srt_str.replace('\\', "/").replace(":/", "\\:/");
                args.extend([
                    "-vf".to_string(),
                    format!("subtitles='{}'", safe_srt),
                ]);
            }
            args.extend([
                "-c:v".to_string(), "libx264".to_string(),
                "-crf".to_string(), "18".to_string(),
                "-preset".to_string(), "fast".to_string(),
                "-c:a".to_string(), "aac".to_string(),
                output_clone.to_string_lossy().to_string(),
            ]);
            let _ = Command::new("ffmpeg").args(&args).status().await;
        }

        let mut store = store_clone.lock().unwrap();
        if let Some(job) = store.jobs.get_mut(&session_id_clone) {
            job.progress = 1.0;
            job.status = if output_clone.exists() { "done".to_string() } else { "error".to_string() };
            job.output_path = if output_clone.exists() { Some(output_clone) } else { None };
        }
    });

    Json(json!({
        "jobId": session_id,
        "status": "started",
    })).into_response()
}

async fn render_status(
    Path(session_id): Path<String>,
    State(s): State<EditorState>,
) -> impl IntoResponse {
    let store = s.store.lock().unwrap();
    match store.jobs.get(&session_id) {
        Some(job) => {
            Json(json!({
                "progress": job.progress,
                "status": job.status,
                "outputPath": job.output_path.as_ref().map(|p| p.to_string_lossy()),
                "error": job.error,
            })).into_response()
        }
        None => Json(json!({
            "progress": 0.0,
            "status": "idle",
        })).into_response(),
    }
}

// ─── Project Save/Load ────────────────────────────────────────────────────────
async fn save_project(
    Path(session_id): Path<String>,
    State(s): State<EditorState>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let project_path = PathBuf::from(format!(
        "cortex_cache/editor_sessions/{}/project.json", session_id
    ));
    if let Some(p) = project_path.parent() {
        let _ = tfs::create_dir_all(p).await;
    }
    match tfs::write(&project_path, &body).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn load_project(
    Path(session_id): Path<String>,
    State(s): State<EditorState>,
) -> impl IntoResponse {
    let project_path = PathBuf::from(format!(
        "cortex_cache/editor_sessions/{}/project.json", session_id
    ));
    match tfs::read_to_string(&project_path).await {
        Ok(content) => (
            [(header::CONTENT_TYPE, "application/json")],
            content,
        ).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────
async fn find_asset_path(
    s: &EditorState,
    session_id: &str,
    asset_id: &str,
) -> Option<PathBuf> {
    let asset_dir = {
        let store = s.store.lock().unwrap();
        store.sessions.get(session_id)?.asset_dir.clone()
    };
    let mut dir = tfs::read_dir(&asset_dir).await.ok()?;
    while let Ok(Some(entry)) = dir.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(asset_id) && !name.ends_with("_thumb.jpg") {
            return Some(entry.path());
        }
    }
    None
}

async fn probe_video_meta(path: &PathBuf) -> (f64, u32, u32, f64) {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=width,height,r_frame_rate:format=duration",
            "-of", "json",
        ])
        .arg(path)
        .output()
        .await;

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        if let Ok(v) = serde_json::from_str::<Value>(&text) {
            let dur = v["format"]["duration"]
                .as_str()
                .and_then(|d| d.parse::<f64>().ok())
                .unwrap_or(0.0);
            let w = v["streams"][0]["width"].as_u64().unwrap_or(1920) as u32;
            let h = v["streams"][0]["height"].as_u64().unwrap_or(1080) as u32;
            let fps_str = v["streams"][0]["r_frame_rate"].as_str().unwrap_or("30/1");
            let fps = parse_fps_ratio(fps_str);
            return (dur, w, h, fps);
        }
    }
    (0.0, 1920, 1080, 30.0)
}

fn parse_fps_ratio(s: &str) -> f64 {
    let parts: Vec<f64> = s.split('/').filter_map(|p| p.parse().ok()).collect();
    if parts.len() == 2 && parts[1] != 0.0 {
        parts[0] / parts[1]
    } else {
        parts.first().copied().unwrap_or(30.0)
    }
}

async fn extract_thumbnail(input: &PathBuf, output: &PathBuf, time: f64) {
    let _ = Command::new("ffmpeg")
        .args(["-y", "-ss", &time.to_string(), "-i"])
        .arg(input)
        .args(["-vframes", "1", "-q:v", "3", "-vf", "scale=320:-1"])
        .arg(output)
        .status()
        .await;
}

fn infer_asset_type(filename: &str) -> String {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "mp4" | "mkv" | "mov" | "avi" | "webm" => "video".to_string(),
        "mp3" | "wav" | "aac" | "ogg" | "m4a" | "flac" => "audio".to_string(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" => "image".to_string(),
        _ => "video".to_string(),
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

async fn serve_file_with_range(
    path: &PathBuf,
    headers: &HeaderMap,
    content_type: &str,
) -> Response {
    use axum::http::StatusCode;

    let metadata = match tfs::metadata(path).await {
        Ok(m) => m,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let total = metadata.len();
    let content_type = content_type.to_string();

    // Parse Range header
    if let Some(range_val) = headers.get("range").and_then(|v| v.to_str().ok()) {
        if let Some(range_bytes) = range_val.strip_prefix("bytes=") {
            let parts: Vec<&str> = range_bytes.split('-').collect();
            let start: u64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
            let end: u64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(total.saturating_sub(1)).min(total - 1);
            let length = end - start + 1;

            let data = read_file_range(path, start, length).await;
            return Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, total))
                .header(header::CONTENT_LENGTH, length)
                .header("Accept-Ranges", "bytes")
                .body(Body::from(data))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    }

    // Full file response
    let data = tfs::read(path).await.unwrap_or_default();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, total)
        .header("Accept-Ranges", "bytes")
        .body(Body::from(data))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn read_file_range(path: &PathBuf, start: u64, length: u64) -> Vec<u8> {
    use std::io::Read;
    use std::io::Seek;
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let _ = file.seek(std::io::SeekFrom::Start(start));
    let mut buf = vec![0u8; length as usize];
    let n = file.read(&mut buf).unwrap_or(0);
    buf.truncate(n);
    buf
}
