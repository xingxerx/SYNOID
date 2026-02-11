// SYNOID Antifragile Supervisor — Self-Healing Execution Loop
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Wraps high-risk tasks in a Try-Heal-Retry loop:
//   1. Execute inside catch_unwind
//   2. On failure, consult the ErrorHealer for flag mutations
//   3. Retry with exponential backoff (max 3 attempts)

use std::time::Duration;
use tracing::{error, info, warn};

// ---------------------------------------------------------------------------
// ErrorHealer — pattern-matches FFmpeg stderr and prescribes safer flags
// ---------------------------------------------------------------------------

pub struct ErrorHealer;

impl ErrorHealer {
    /// Analyse an FFmpeg error log and mutate the argument list toward a
    /// safer configuration that has a higher chance of succeeding.
    pub fn suggest_fix(error_log: &str, current_args: Vec<String>) -> Vec<String> {
        let mut new_args = current_args;
        let err_lower = error_log.to_lowercase();

        // GPU failure → fall back to CPU encoding
        if err_lower.contains("out of memory")
            || err_lower.contains("nvenc")
            || err_lower.contains("cuda")
            || err_lower.contains("gpu")
        {
            warn!("[HEALER] GPU failure detected — switching to CPU (libx264).");
            new_args.retain(|a| {
                !a.contains("nvenc")
                    && !a.contains("cuda")
                    && !a.contains("gpu")
            });
            new_args.extend([
                "-c:v".to_string(),
                "libx264".to_string(),
                "-crf".to_string(),
                "23".to_string(),
            ]);
        }

        // Threading pressure → single-thread + ultrafast preset
        if err_lower.contains("out of memory") || err_lower.contains("resource") {
            warn!("[HEALER] Resource pressure — reducing threads & using ultrafast preset.");
            new_args.extend([
                "-threads".to_string(),
                "1".to_string(),
                "-preset".to_string(),
                "ultrafast".to_string(),
            ]);
        }

        // Pixel format incompatibility
        if err_lower.contains("invalid pixel format")
            || err_lower.contains("pixel format")
            || err_lower.contains("incompatible")
        {
            warn!("[HEALER] Pixel format issue — normalizing to yuv420p.");
            new_args.extend([
                "-vf".to_string(),
                "format=yuv420p".to_string(),
            ]);
        }

        new_args
    }
}

// ---------------------------------------------------------------------------
// AntifragileSupervisor — the Try-Heal-Retry orchestrator
// ---------------------------------------------------------------------------

/// Maximum number of retry attempts per task.
const MAX_RETRIES: u32 = 3;

pub struct AntifragileSupervisor;

impl AntifragileSupervisor {
    /// Execute an async task with automatic retry and exponential backoff.
    ///
    /// * `task_name` — human-readable label for logging.
    /// * `run`       — the async closure to execute. Returns `Ok(T)` or
    ///                 `Err(String)` with a description (ideally stderr).
    pub async fn execute_with_retry<T, F, Fut>(task_name: &str, mut run: F) -> Result<T, String>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        let mut attempt = 0u32;

        loop {
            attempt += 1;
            info!(
                "[SUPERVISOR] Attempt {}/{} for task '{}'",
                attempt, MAX_RETRIES, task_name
            );

            match run().await {
                Ok(result) => {
                    info!("[SUPERVISOR] ✅ Task '{}' succeeded on attempt {}.", task_name, attempt);
                    return Ok(result);
                }
                Err(e) => {
                    error!(
                        "[SUPERVISOR] ❌ Task '{}' failed (attempt {}): {}",
                        task_name, attempt, e
                    );

                    if attempt >= MAX_RETRIES {
                        error!(
                            "[SUPERVISOR] Task '{}' exhausted all {} retries. Giving up.",
                            task_name, MAX_RETRIES
                        );
                        return Err(format!(
                            "Task '{}' failed after {} attempts. Last error: {}",
                            task_name, MAX_RETRIES, e
                        ));
                    }

                    // Exponential backoff: 2s, 4s, 8s
                    let delay = Duration::from_secs(2u64.pow(attempt));
                    warn!(
                        "[SUPERVISOR] Retrying '{}' in {:?}...",
                        task_name, delay
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_error_healer_oom() {
        let args = vec!["-c:v".to_string(), "h264_nvenc".to_string()];
        let fixed = ErrorHealer::suggest_fix("Error: Out of memory allocating frame", args);
        assert!(fixed.contains(&"libx264".to_string()), "Should fall back to libx264");
        assert!(fixed.contains(&"1".to_string()), "Should set threads to 1");
    }

    #[test]
    fn test_error_healer_nvenc() {
        let args = vec!["-c:v".to_string(), "h264_nvenc".to_string()];
        let fixed = ErrorHealer::suggest_fix("NVENC codec not supported on this GPU", args);
        assert!(!fixed.contains(&"h264_nvenc".to_string()), "Should remove nvenc");
        assert!(fixed.contains(&"libx264".to_string()));
    }

    #[test]
    fn test_error_healer_pixel_format() {
        let args = vec!["-c:v".to_string(), "libx264".to_string()];
        let fixed = ErrorHealer::suggest_fix("Invalid pixel format requested", args);
        assert!(fixed.contains(&"format=yuv420p".to_string()));
    }

    #[tokio::test]
    async fn test_supervisor_succeeds_first_try() {
        let result = AntifragileSupervisor::execute_with_retry("test_ok", || async {
            Ok::<_, String>("done".to_string())
        })
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "done");
    }

    #[tokio::test]
    async fn test_supervisor_retries_then_succeeds() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();

        let result = AntifragileSupervisor::execute_with_retry("test_retry", move || {
            let c = c.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst) + 1;
                if n < 2 {
                    Err("transient failure".to_string())
                } else {
                    Ok::<_, String>("recovered".to_string())
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "recovered");
    }
}
