use axum::{
    extract::{Query, Request, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceExt; // For oneshot
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{error, info};
use std::path::{Path, PathBuf, Component};

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

pub async fn start_server(port: u16, state: Arc<KernelState>) {
    let app = Router::new()
        .nest_service("/", ServeDir::new("dashboard"))
        .route("/api/status", get(get_status))
        .route("/api/tasks", get(get_tasks))
        .route("/api/chat", post(handle_chat))
        .route("/api/stream", get(stream_video))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let display_addr = if addr.ip().is_unspecified() {
        format!("127.0.0.1:{}", port)
    } else {
        addr.to_string()
    };
    info!(
        "🚀 SYNOID Dashboard Server running on http://{}",
        display_addr
    );

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
fn is_safe_path(path: &std::path::Path) -> bool {
    // 1. Check for directory traversal (..)
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return false;
        }
    }

    // 2. Check for hidden files (starting with .)
    if let Some(file_name) = path.file_name() {
        let name = file_name.to_string_lossy();
        if name.starts_with('.') {
            return false;
        }
    } else {
        return false; // No filename? Unlikely to be a valid file to stream
    }

    // 3. Check extension against strict allowlist
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        let allowed_extensions = [
            "mp4", "mkv", "avi", "mov", "webm", // Video
            "mp3", "wav", "flac", "aac", "ogg", // Audio
            "jpg", "jpeg", "png", "webp", "gif", // Image
        ];

        // Explicitly reject SVG as per security standards
        if ext_str == "svg" {
            return false;
        }

        allowed_extensions.contains(&ext_str.as_str())
    } else {
        false // No extension is suspicious for media streaming
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

async fn stream_video(
    Query(params): Query<StreamParams>,
    req: Request,
) -> impl axum::response::IntoResponse {
    let path = std::path::PathBuf::from(params.path);

    if !is_safe_media_path(&path) {
        return (
            axum::http::StatusCode::FORBIDDEN,
            "Access Denied: Invalid file type",
        )
            .into_response();
    }
    // Security check: Validate path before accessing filesystem
    if !is_safe_path(&path) {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    let path = match validate_stream_path(&params.path) {
        Ok(p) => p,
        Err(e) => {
            error!("Stream access denied: {}", e);
            return (axum::http::StatusCode::FORBIDDEN, e).into_response();
        }
    };

    if !path.exists() {
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

    #[test]
    fn test_validate_stream_path() {
        // Valid cases
        assert!(validate_stream_path("video.mp4").is_ok());
        assert!(validate_stream_path("movie.mkv").is_ok());
        assert!(validate_stream_path("/abs/path/to/video.mp4").is_ok());
        assert!(validate_stream_path("nested/folder/song.mp3").is_ok());

        // Invalid cases
        assert!(validate_stream_path("../secret.txt").is_err());
        assert!(validate_stream_path("../../etc/passwd").is_err());
        assert!(validate_stream_path("/etc/passwd").is_err()); // No extension
        assert!(validate_stream_path("script.sh").is_err()); // Invalid extension
        assert!(validate_stream_path("image.png").is_err()); // Invalid extension
        assert!(validate_stream_path("..").is_err());
        assert!(validate_stream_path("").is_err());
    }
}
