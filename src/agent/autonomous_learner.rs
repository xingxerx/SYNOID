// SYNOID Autonomous Learner
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::brain::{Brain, Intent};
use crate::agent::source_tools;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info};

pub struct AutonomousLearner {
    is_running: Arc<AtomicBool>,
    brain: Arc<Mutex<Brain>>,
    learning_topics: Vec<String>,
}

impl AutonomousLearner {
    pub fn new(brain: Arc<Mutex<Brain>>) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            brain,
            learning_topics: vec![
                "cinematic travel video".to_string(),
                "gaming montage editing".to_string(),
                "vlog editing tips".to_string(),
                "documentary style editing".to_string(),
            ],
        }
    }

    pub fn start(&self) {
        if self.is_running.load(Ordering::SeqCst) {
            info!("[LEARNER] Already running.");
            return;
        }

        self.is_running.store(true, Ordering::SeqCst);
        let is_running = self.is_running.clone();
        let brain = self.brain.clone();
        let topics = self.learning_topics.clone();

        // Initialize Sentinel for health monitoring
        let mut sentinel = crate::agent::defense::Sentinel::new();

        info!("[LEARNER] 🚀 Autonomous Learning Loop Started (Sentinel Active)");

        tokio::spawn(async move {
            let mut topic_index = 0;

            while is_running.load(Ordering::SeqCst) {
                // 0. Sentinel Health Check
                let alerts = sentinel.scan_processes();
                if !alerts.is_empty() {
                    tracing::warn!("[LEARNER] ⚠️ System under pressure. Pausing learning cycle.");
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    continue;
                }

                let topic = &topics[topic_index % topics.len()];
                info!("[LEARNER] 🔍 Scouting topic: '{}'", topic);

                // 1. Search for candidates
                let search_result = source_tools::search_youtube(topic, 3)
                    .await
                    .map_err(|e| e.to_string());

                match search_result {
                    Ok(results) => {
                        for source in results {
                            if !is_running.load(Ordering::SeqCst) {
                                break;
                            }

                            // Filter criteria (e.g., duration < 10 mins to be quick)
                            if source.duration > 60.0 && source.duration < 600.0 {
                                // 1b. Safety Check URL
                                if let Some(url) = &source.original_url {
                                    if let Err(e) = crate::agent::download_guard::DownloadGuard::validate_url(url) {
                                        error!("[LEARNER] 🛡️ Skipped unsafe URL: {}", e);
                                        continue;
                                    }
                                }

                                info!("[LEARNER] 📥 Acquiring candidate: {}", source.title);

                                let cache_dir = std::path::Path::new("cortex_cache");
                                let download_result = source_tools::download_youtube(
                                    source.original_url.as_deref().unwrap_or(""),
                                    cache_dir,
                                    None,
                                )
                                .await
                                .map_err(|e| e.to_string());

                                match download_result {
                                    Ok(downloaded) => {
                                        // 1c. Safety Check File
                                        if let Err(e) = crate::agent::download_guard::DownloadGuard::validate_downloaded_file(&downloaded.local_path) {
                                            error!("[LEARNER] 🛡️ Downloaded file rejected: {}", e);
                                            let _ = std::fs::remove_file(downloaded.local_path);
                                            continue;
                                        }

                                        info!("[LEARNER] 🎓 Learning from: {}", downloaded.title);

                                        // 2. Process with Brain
                                        let mut brain_lock = brain.lock().await;

                                        // Calculate adaptive delay based on neuroplasticity
                                        let speed = brain_lock.neuroplasticity.current_speed();
                                        let level = brain_lock.neuroplasticity.adaptation_level();
                                        let sleep_duration = brain_lock.neuroplasticity.adaptive_delay_secs(30);

                                        let intent = Intent::LearnStyle {
                                            input: downloaded
                                                .local_path
                                                .to_string_lossy()
                                                .to_string(),
                                            name: format!("auto_{}", topic.replace(" ", "_")),
                                        };

                                        match brain_lock
                                            .process(&format!(
                                                "learn style '{:?}' from {:?}",
                                                intent, downloaded.local_path
                                            ))
                                            .await
                                        {
                                            Ok(res) => info!("[LEARNER] ✅ {} (Speed: {:.1}× - {})", res, speed, level),
                                            Err(e) => error!("[LEARNER] ❌ Failed to learn: {}", e),
                                        }

                                        // 3. Cleanup
                                        let _ = std::fs::remove_file(downloaded.local_path);

                                        // Adaptive Sleep
                                        drop(brain_lock); // Unlock before sleeping
                                        info!("[LEARNER] 💤 Resting for {}s (Adaptive)", sleep_duration);
                                        tokio::time::sleep(Duration::from_secs(sleep_duration)).await;
                                    }
                                    Err(e) => {
                                        error!("[LEARNER] Failed download: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => error!("[LEARNER] Search failed: {}", e),
                }

                topic_index += 1;
                // Sleep between topic cycles - also adaptive? For now fixed 30s base
                tokio::time::sleep(Duration::from_secs(30)).await;
            }

            info!("[LEARNER] 🛑 Loop Stopped");
        });
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}
