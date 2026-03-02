use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::sync::Arc;
use synoid_core::agent::core::AgentCore;
use synoid_core::server;
use synoid_core::state::KernelState;
use tower::ServiceExt;

#[tokio::test]
async fn test_api_status_authenticated_access() {
    std::env::set_var("SYNOID_API_KEY", "test_key");

    let core = Arc::new(AgentCore::new("http://localhost:11434/v1"));
    let state = Arc::new(KernelState::new(core));
    let app = server::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .header("X-API-Key", "test_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_status_unauthorized_access() {
    std::env::set_var("SYNOID_API_KEY", "test_key");

    let core = Arc::new(AgentCore::new("http://localhost:11434/v1"));
    let state = Arc::new(KernelState::new(core));
    let app = server::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_stream_query_param_auth() {
    std::env::set_var("SYNOID_API_KEY", "test_key");

    let core = Arc::new(AgentCore::new("http://localhost:11434/v1"));
    let state = Arc::new(KernelState::new(core));
    let app = server::create_router(state);

    // We don't care if the file exists for auth check,
    // but the handler might check it.
    // However, the middleware runs FIRST.

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/stream?path=test.mp4&api_key=test_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // It should NOT be 401. It might be 404 if test.mp4 doesn't exist.
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}
