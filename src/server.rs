use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
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

async fn auth_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    let api_key = std::env::var("SYNOID_API_KEY").map_err(|_| {
        error!("SYNOID_API_KEY not set");
        StatusCode::UNAUTHORIZED
    })?;

    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    let x_api_key = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok());

    let query_api_key = req.uri().query().and_then(|q| {
        url::form_urlencoded::parse(q.as_bytes())
            .find(|(key, _)| key == "api_key")
            .map(|(_, value)| value.into_owned())
    });

    if let Some(key) = auth_header.or(x_api_key).or(query_api_key.as_deref()) {
        if key == api_key {
            return Ok(next.run(req).await);
        }
    }

    error!("Unauthorized access attempt to API");
    Err(StatusCode::UNAUTHORIZED)
}

pub fn create_router(state: Arc<KernelState>) -> Router {
    let api_routes = Router::new()
        .route("/status", get(get_status))
        .route("/tasks", get(get_tasks))
        .route("/chat", post(handle_chat))
        .route("/stream", get(stream_video))
        .layer(middleware::from_fn(auth_middleware));

    Router::new()
        .nest_service("/", ServeDir::new("dashboard"))
        .nest("/api", api_routes)
        .with_state(state)
        .layer(CorsLayer::permissive())
}

pub async fn start_server(port: u16, state: Arc<KernelState>) {
    if std::env::var("SYNOID_API_KEY").is_err() {
        error!("🚨 SECURITY ALERT: SYNOID_API_KEY is not set!");
        error!("API endpoints are LOCKED. Set the SYNOID_API_KEY environment variable to allow access.");
    }

    let app = create_router(state);

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

async fn stream_video(
    Query(params): Query<StreamParams>,
    req: Request,
) -> impl axum::response::IntoResponse {
    let path = std::path::PathBuf::from(params.path);
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
