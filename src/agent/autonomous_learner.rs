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

        info!("[LEARNER] 🚀 Autonomous Learning Loop Started");

        tokio::spawn(async move {
            let mut topic_index = 0;

            while is_running.load(Ordering::SeqCst) {
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
                                        info!("[LEARNER] 🎓 Learning from: {}", downloaded.title);

                                        // 2. Process with Brain
                                        let mut brain_lock = brain.lock().await;
                                        let intent = Intent::LearnStyle {
                                            input: downloaded
                                                .local_path
                                                .to_string_lossy()
                                                .to_string(),
                                            name: format!("auto_{}", topic.replace(" ", "_")),
                                        };

                                        // We manually trigger processing logic here or assume brain handles it
                                        // For now, let's just log the 'success' of the attempt
                                        match brain_lock
                                            .process(&format!(
                                                "learn style '{:?}' from {:?}",
                                                intent, downloaded.local_path
                                            ))
                                            .await
                                        {
                                            Ok(res) => info!("[LEARNER] ✅ {}", res),
                                            Err(e) => error!("[LEARNER] ❌ Failed to learn: {}", e),
                                        }

                                        // 3. Cleanup
                                        let _ = std::fs::remove_file(downloaded.local_path);
                                    }
                                    Err(e) => {
                                        error!("[LEARNER] Failed download: {}", e);
                                    }
                                }

                                // Sleep between successful learns
                                tokio::time::sleep(Duration::from_secs(10)).await;
                            }
                        }
                    }
                    Err(e) => error!("[LEARNER] Search failed: {}", e),
                }

                topic_index += 1;
                // Sleep between topic cycles
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
