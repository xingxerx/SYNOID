// SYNOID Networking — shared HTTP client factory and retry primitives
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID

use reqwest::{Client, ClientBuilder};
use std::time::Duration;
use tracing::warn;

pub const USER_AGENT: &str =
    concat!("SYNOID/", env!("CARGO_PKG_VERSION"), " (sovereign-ai-video-editor)");

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const POOL_MAX_IDLE_PER_HOST: usize = 10;
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const TCP_KEEPALIVE: Duration = Duration::from_secs(30);

/// HTTP client tuned for external HTTPS services.
/// Enables gzip/brotli/deflate decompression, HTTP/2 adaptive window,
/// connection pooling, TCP keepalive, and a 10 s connect timeout.
pub fn build_client(request_timeout: Duration) -> Client {
    base_builder(request_timeout)
        .gzip(true)
        .deflate(true)
        .brotli(true)
        .build()
        .expect("failed to build HTTP client")
}

/// HTTP client tuned for localhost services (Ollama, ComfyUI).
/// No TLS overhead, no compression (localhost is already fast),
/// same pool / keepalive / connect-timeout discipline.
pub fn build_local_client(request_timeout: Duration) -> Client {
    base_builder(request_timeout)
        .build()
        .expect("failed to build local HTTP client")
}

fn base_builder(request_timeout: Duration) -> ClientBuilder {
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(request_timeout)
        .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .tcp_keepalive(TCP_KEEPALIVE)
        .tcp_nodelay(true)
        .http2_adaptive_window(true)
        .user_agent(USER_AGENT)
}

/// Retry an async closure up to `max_retries` additional attempts on error.
/// Backoff: 1 s → 2 s → 4 s (capped). Only retries when `is_retryable(err)` is true.
pub async fn retry<F, Fut, T, E, R>(max_retries: u32, is_retryable: R, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
    R: Fn(&E) -> bool,
{
    let mut attempt = 0u32;
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt < max_retries && is_retryable(&e) => {
                attempt += 1;
                let delay = Duration::from_millis(500u64 * (1u64 << attempt));
                warn!(
                    "[NET] transient error (attempt {}/{}): {}. retrying in {}ms.",
                    attempt,
                    max_retries,
                    e,
                    delay.as_millis()
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Returns `true` for reqwest errors that are worth retrying (timeout, connect, network I/O).
pub fn is_transient_reqwest(e: &reqwest::Error) -> bool {
    e.is_timeout() || e.is_connect() || e.is_request()
}

/// Constant-time byte-slice comparison — prevents timing-based secret leakage.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}
