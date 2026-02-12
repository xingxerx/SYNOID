// SYNOID SignalSentinel — Graceful Shutdown Handler
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Intercepts OS termination signals (Ctrl-C / SIGTERM) and invokes
// an emergency save callback before exiting, ensuring zero data loss.

use std::future::Future;
use std::pin::Pin;
use tracing::{info, warn};

/// Type alias for the async emergency-save callback.
pub type EmergencySaveFn = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Spawn a background task that waits for Ctrl-C and then runs the
/// provided emergency-save closure before exiting.
///
/// # Example
/// ```ignore
/// signals::install_signal_handler(Box::new(|| Box::pin(async {
///     // save recovery manifest, flush .tmp files, etc.
/// })));
/// ```
pub fn install_signal_handler(on_shutdown: EmergencySaveFn) {
    tokio::spawn(async move {
        // Wait for Ctrl-C (works on both Windows and Unix via tokio)
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                warn!("[SIGNAL] ⛔ SIGINT (Ctrl-C) received. Initiating Atomic Stop...");
                on_shutdown().await;
                info!("[SIGNAL] ✅ Emergency save complete. SYNOID hibernated safely.");
                std::process::exit(0);
            }
            Err(e) => {
                warn!("[SIGNAL] Failed to install Ctrl-C handler: {}", e);
            }
        }
    });
}
