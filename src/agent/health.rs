// SYNOID Health Check & Watchdog System
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Provides continuous self-monitoring, crash recovery, and uptime guarantees.
// The HealthMonitor runs as a background task and periodically checks system health.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::SystemExt;
use tracing::{error, info, warn};

/// Health status of a subsystem
#[derive(Debug, Clone, PartialEq)]
pub enum SubsystemStatus {
    Healthy,
    Degraded(String),
    Down(String),
}

/// Tracks the health of the entire SYNOID system
pub struct HealthMonitor {
    start_time: Instant,
    is_running: Arc<AtomicBool>,
    heartbeat_count: Arc<AtomicU64>,
    check_interval: Duration,
}

impl HealthMonitor {
    /// Create a new health monitor with the given check interval
    pub fn new(check_interval_secs: u64) -> Self {
        Self {
            start_time: Instant::now(),
            is_running: Arc::new(AtomicBool::new(false)),
            heartbeat_count: Arc::new(AtomicU64::new(0)),
            check_interval: Duration::from_secs(check_interval_secs),
        }
    }

    /// Get system uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Get total heartbeat count
    pub fn heartbeat_count(&self) -> u64 {
        self.heartbeat_count.load(Ordering::Relaxed)
    }

    /// Check if the monitor is currently running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Start the background health monitoring loop.
    /// Returns a handle that can be used to stop the monitor.
    pub fn start(&self) -> Arc<AtomicBool> {
        let is_running = self.is_running.clone();
        let heartbeat_count = self.heartbeat_count.clone();
        let interval = self.check_interval;

        is_running.store(true, Ordering::Relaxed);
        let shutdown = is_running.clone();

        tokio::spawn(async move {
            info!("[HEALTH] Watchdog started (interval: {:?})", interval);

            // Track previous state to only log on transitions (like PressureWatcher)
            let mut prev_mem_ok = true;
            let mut prev_disk_ok = true;

            while is_running.load(Ordering::Relaxed) {
                tokio::time::sleep(interval).await;

                let count = heartbeat_count.fetch_add(1, Ordering::Relaxed) + 1;

                // Check system memory
                let mem_ok = check_memory_health();
                // Check disk space
                let disk_ok = check_disk_health();

                // Only log on state transitions to prevent log spam
                if !mem_ok && prev_mem_ok {
                    warn!(
                        "[HEALTH] ⚠️ Memory pressure detected (heartbeat #{})",
                        count
                    );
                } else if mem_ok && !prev_mem_ok {
                    info!(
                        "[HEALTH] ✅ Memory pressure resolved (heartbeat #{})",
                        count
                    );
                }

                if !disk_ok && prev_disk_ok {
                    warn!("[HEALTH] ⚠️ Low disk space detected (heartbeat #{})", count);
                } else if disk_ok && !prev_disk_ok {
                    info!("[HEALTH] ✅ Disk space restored (heartbeat #{})", count);
                }

                prev_mem_ok = mem_ok;
                prev_disk_ok = disk_ok;

                if count % 60 == 0 {
                    // Log a summary every ~60 heartbeats
                    info!(
                        "[HEALTH] ♥ System alive | Heartbeat #{} | Memory: {} | Disk: {}",
                        count,
                        if mem_ok { "OK" } else { "WARN" },
                        if disk_ok { "OK" } else { "WARN" },
                    );
                }
            }

            info!("[HEALTH] Watchdog stopped.");
        });

        shutdown
    }

    /// Stop the health monitor
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::Relaxed);
        info!("[HEALTH] Shutdown requested.");
    }

    /// Get a formatted status report
    pub fn status_report(&self) -> String {
        let uptime = self.uptime_secs();
        let hours = uptime / 3600;
        let minutes = (uptime % 3600) / 60;
        let secs = uptime % 60;

        format!(
            "SYNOID Health Report\n  Uptime: {}h {}m {}s\n  Heartbeats: {}\n  Status: {}",
            hours,
            minutes,
            secs,
            self.heartbeat_count(),
            if self.is_running() {
                "MONITORING"
            } else {
                "STOPPED"
            },
        )
    }
}

/// Check if system memory usage is acceptable
fn check_memory_health() -> bool {
    // Use sysinfo for a quick memory check
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let total = sys.total_memory();
    let used = sys.used_memory();
    if total == 0 {
        return true; // Can't determine, assume OK
    }
    let usage_pct = (used as f64 / total as f64) * 100.0;
    usage_pct < 95.0 // Alert if >95% memory used
}

/// Check if disk space is acceptable
fn check_disk_health() -> bool {
    // Simple check: can we write to the current directory?
    match std::env::current_dir() {
        Ok(dir) => {
            let test_path = dir.join(".synoid_health_check");
            match std::fs::write(&test_path, b"ok") {
                Ok(_) => {
                    if let Err(e) = std::fs::remove_file(&test_path) {
                        warn!("[HEALTH] Could not remove disk check temp file: {}", e);
                    }
                    true
                }
                Err(e) => {
                    error!("[HEALTH] Disk write check failed: {}", e);
                    false
                }
            }
        }
        Err(_) => false,
    }
}

/// Check for required external dependencies.
/// Only returns truly required tools (ffmpeg, python).
/// Optional tools (yt-dlp, ollama) are checked but not reported as missing.
pub fn check_dependencies() -> Vec<String> {
    let mut missing = Vec::new();

    // Required dependencies — the app cannot function well without these
    let required = vec![
        ("ffmpeg", "-version"),
        ("python", "--version"),
    ];

    for (dep, flag) in required {
        let mut found = std::process::Command::new(dep)
            .arg(flag)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        
        // Special fallbacks for common developer environments
        if !found && dep == "python" {
            found = std::process::Command::new("python3")
                .arg(flag)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }

        if !found {
            missing.push(dep.to_string());
        }
    }

    // Optional dependencies — features degrade gracefully without these
    let optional = vec![
        ("yt-dlp", "--version"),
        ("ollama", "--version"),
    ];

    for (dep, flag) in optional {
        let mut found = std::process::Command::new(dep)
            .arg(flag)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !found && dep == "yt-dlp" {
            // Check if available as a python module
            for py_cmd in ["python3", "python", "py"] {
                if std::process::Command::new(py_cmd)
                    .args(["-m", "yt_dlp", "--version"])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false) 
                {
                    found = true;
                    break;
                }
            }
        }

        if !found {
            tracing::trace!("Optional dependency '{}' not found — related features will be unavailable.", dep);
        }
    }

    missing
}
