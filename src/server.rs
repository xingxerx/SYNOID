use axum::{
    extract::{Query, Request, State},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    http::{StatusCode, HeaderMap},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceExt; // For oneshot
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{error, info};
use std::path::{PathBuf, Component};

use crate::state::{DashboardStatus, DashboardTask, KernelState, TasksStatus};

pub type AppState = Arc<KernelState>;

#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub response: String,
}

#[derive(Deserialize)]
struct StreamParams {
    path: String,
}

const ALLOWED_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "mov", "avi", "webm", "flv", "wmv",
    "mp3", "wav", "flac", "aac", "ogg", "m4a",
    "jpg", "jpeg", "png", "gif", "bmp", "webp",
];

fn is_safe_media_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ALLOWED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}


fn validate_stream_path(raw_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_path);

    // 1. Prevent Directory Traversal
    for component in path.components() {
        if let Component::ParentDir = component {
            return Err("Access denied: Path traversal detected".to_string());
        }
    }

    // 2. Validate Extension
    let allowed_extensions = [
        "mp4", "mkv", "avi", "mov", "webm", // Video
        "mp3", "wav", "flac", "ogg", "m4a"  // Audio
    ];

    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext {
        Some(e) if allowed_extensions.contains(&e.as_str()) => Ok(path),
        Some(e) => Err(format!("Access denied: Invalid file extension '.{}'", e)),
        None => Err("Access denied: No file extension provided".to_string()),
    }
}

pub fn create_router(state: Arc<KernelState>) -> Router {
    Router::new()
        .nest_service("/", ServeDir::new("dashboard"))
        .route("/api/status", get(get_status))
        .route("/api/tasks", get(get_tasks))
        .route("/api/chat", post(handle_chat))
        .route("/api/stream", get(stream_video))
        .layer(middleware::from_fn(auth_middleware))
        .with_state(state)
        .layer(CorsLayer::permissive())
}

pub async fn start_server(port: u16, state: Arc<KernelState>) {
    if std::env::var("SYNOID_API_KEY").is_err() {
        warn!("💡 TIP: SYNOID_API_KEY is not set. Using default developer key for local access.");
        warn!("To secure your dashboard, set the SYNOID_API_KEY environment variable.");
    }

    let app = create_router(state);

    // Bind to localhost for security (prevent external access by default)
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let display_addr = addr.to_string();

    info!(
        "🚀 SYNOID Dashboard Server running on http://{}",
        display_addr
    );

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("❌ Failed to bind to address {}: {}", display_addr, e);
            return;
        }
    };
    if let Err(e) = axum::serve(listener, app).await {
        error!("❌ Server error: {}", e);
    }
}

async fn get_status(State(state): State<AppState>) -> Json<DashboardStatus> {
    let task = state.task.lock().unwrap();
    let active_count = if task.is_running { 1 } else { 0 };

    Json(DashboardStatus {
        tasks: TasksStatus {
            active: active_count,
            total: 20,
        },
        productivity: 85,
    })
}

async fn get_tasks(State(state): State<AppState>) -> Json<Vec<DashboardTask>> {
    let task = state.task.lock().unwrap();

    let mut tasks = vec![];
    if !task.input_path.is_empty() {
        tasks.push(DashboardTask {
            title: format!("Process: {}", task.input_path),
            category: "Video".to_string(),
            due: "Now".to_string(),
            completed: !task.is_running,
            priority: "High".to_string(),
        });
    }

    tasks.push(DashboardTask {
        title: "System Initialization".to_string(),
        category: "System".to_string(),
        due: "Done".to_string(),
        completed: true,
        priority: "Normal".to_string(),
    });

    Json(tasks)
}

async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: middleware::Next,
) -> Result<Response, StatusCode> {
    let api_key = std::env::var("SYNOID_API_KEY").unwrap_or_else(|_| "synoid_secret_v1".to_string());
    
    match headers.get("x-api-key") {
        Some(key) if key == api_key.as_str() => Ok(next.run(request).await),
        _ => {
            error!("❌ access denied: missing or invalid api key");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[axum::debug_handler]
async fn handle_chat(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Json<ChatResponse> {
    info!("Brain receiving: {}", payload.message);

    let mut brain = state.core.brain.lock().await;
    match brain.process(&payload.message).await {
        Ok(res) => Json(ChatResponse { response: res }),
        Err(e) => {
            error!("Brain Error: {}", e);
            Json(ChatResponse {
                response: format!("Error: {}", e),
            })
        }
    }
}

async fn stream_video(
    Query(params): Query<StreamParams>,
    req: Request,
) -> impl axum::response::IntoResponse {
    let path = std::path::PathBuf::from(&params.path);

    if !is_safe_media_path(&path) {
        return (
            axum::http::StatusCode::FORBIDDEN,
            "Access Denied: Invalid file type",
        )
            .into_response();
    }
    // Security check: Validate path before accessing filesystem
    let path = match validate_stream_path(&params.path) {
        Ok(p) => p,
        Err(e) => {
            error!("Stream access denied: {}", e);
            return (axum::http::StatusCode::FORBIDDEN, e).into_response();
        }
    };

    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }

    let service = tower_http::services::ServeFile::new(path);
    match service.oneshot(req).await {
        Ok(res) => res.into_response(),
        Err(err) => {
            error!("ServeFile error: {}", err);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_safe_media_path() {
        // Safe paths
        assert!(is_safe_media_path(Path::new("video.mp4")));
        assert!(is_safe_media_path(Path::new("movie.mkv")));
        assert!(is_safe_media_path(Path::new("image.jpg")));
        assert!(is_safe_media_path(Path::new("image.PNG"))); // Case insensitive
        assert!(is_safe_media_path(Path::new("/path/to/video.mp4")));

        // Unsafe paths
        assert!(!is_safe_media_path(Path::new("script.sh")));
        assert!(!is_safe_media_path(Path::new("/etc/passwd")));
        assert!(!is_safe_media_path(Path::new("config.json")));
        assert!(!is_safe_media_path(Path::new("no_extension")));
        assert!(!is_safe_media_path(Path::new("malicious.exe")));
        assert!(!is_safe_media_path(Path::new("image.svg"))); // SVG is unsafe
        assert!(!is_safe_media_path(Path::new("..")));
    }

    #[test]
    fn test_validate_stream_path() {
        // Valid cases
        assert!(validate_stream_path("video.mp4").is_ok());
        assert!(validate_stream_path("movie.mkv").is_ok());
        // Nested relative paths are OK
        assert!(validate_stream_path("nested/folder/song.mp3").is_ok());

        // Invalid cases
        assert!(validate_stream_path("../secret.txt").is_err()); // Traversal
        assert!(validate_stream_path("../../etc/passwd").is_err());
        assert!(validate_stream_path("script.sh").is_err()); // Invalid extension
        assert!(validate_stream_path("..").is_err());
        assert!(validate_stream_path("").is_err());
    }
}
