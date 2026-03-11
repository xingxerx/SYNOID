// SYNOID Autonomous Learner
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID

use crate::agent::brain::Brain;
use crate::agent::smart_editor;
use crate::agent::{academy::code_scanner::CodeScanner, source_tools};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

#[derive(Serialize, Deserialize, Default, Clone)]
struct LearnerState {
    topic_index: usize,
    repo_index: usize,
    processed_urls: HashSet<String>,
    known_repos: Vec<String>,
    #[serde(default)]
    downloaded_videos: Vec<VideoRecord>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct VideoRecord {
    path: String,
    score: f64,
}

impl LearnerState {
    fn path(instance_id: &str) -> PathBuf {
        let dir = PathBuf::from(format!("cortex_cache{}", instance_id));
        let _ = fs::create_dir_all(&dir);
        dir.join("learner_state.json")
    }

    fn load(instance_id: &str) -> Self {
        if let Ok(data) = fs::read_to_string(Self::path(instance_id)) {
            if let Ok(state) = serde_json::from_str::<LearnerState>(&data) {
                info!(
                    "[LEARNER] 🧠 Restored state: {} topics processed, {} repos known",
                    state.topic_index,
                    state.known_repos.len()
                );
                return state;
            }
        }
        info!("[LEARNER] 🌱 Initialized fresh learner state");
        Self::default()
    }

    fn save(&self, instance_id: &str) {
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = fs::write(Self::path(instance_id), data);
        } else {
            error!("[LEARNER] ❌ Failed to serialize learner state");
        }
    }
}

pub struct AutonomousLearner {
    is_running: Arc<AtomicBool>,
    brain: Arc<Mutex<Brain>>,
    state: Arc<Mutex<LearnerState>>,
    learning_topics: Vec<String>,
    wiki_targets: Vec<String>,
    instance_id: String,
}

impl AutonomousLearner {
    pub fn new(brain: Arc<Mutex<Brain>>, instance_id: &str) -> Self {
        let mut state = LearnerState::default();
        let inst_id = instance_id.to_string();

        // Pre-populate some known repos if empty (fresh state)
        if state.known_repos.is_empty() {
            state.known_repos = vec![
                "https://github.com/mltframework/mlt".to_string(),
                "https://github.com/KDE/kdenlive".to_string(),
                "https://github.com/OpenShot/libopenshot".to_string(),
                "https://github.com/Shotcut/shotcut".to_string(),
                "https://github.com/obsproject/obs-studio".to_string(),
            ];
        }

        // Merge saved state
        let saved = LearnerState::load(&inst_id);
        if !saved.known_repos.is_empty() {
            state = saved;
        }

        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            brain,
            state: Arc::new(Mutex::new(state)),
            instance_id: inst_id,
            learning_topics: vec![
                "cinematic travel video".to_string(),
                "gaming montage editing".to_string(),
                "vlog editing tips".to_string(),
                "documentary style editing".to_string(),
            ],
            wiki_targets: vec![
                "https://en.wikipedia.org/wiki/Film_editing".to_string(),
                "https://en.wikipedia.org/wiki/Montage_(filmmaking)".to_string(),
                "https://en.wikipedia.org/wiki/Color_grading".to_string(),
                "https://en.wikipedia.org/wiki/Kuleshov_effect".to_string(),
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
        let state_arc = self.state.clone();
        let topics = self.learning_topics.clone();
        let wikis = self.wiki_targets.clone();

        // Initialize Sentinel and Scanner (non-async)
        let mut sentinel = crate::agent::defense::Sentinel::new();
        let scanner = CodeScanner::new("http://localhost:11434/v1");

        let instance_id = self.instance_id.clone();

        info!("[LEARNER] 🚀 Autonomous Learning Loop Started (Sentinel Active)");

        tokio::spawn(async move {
            let mut cycle_count = 0;

            while is_running.load(Ordering::SeqCst) {
                cycle_count += 1;
                info!("[LEARNER] 🏁 Starting Learning Cycle #{}", cycle_count);

                // 0. Sentinel Health Check
                let alerts = sentinel.scan_processes();
                if !alerts.is_empty() {
                    tracing::warn!("[LEARNER] ⚠️ System under pressure. Pausing learning cycle.");
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    continue;
                }

                // Lock state for this cycle
                let mut state = state_arc.lock().await;

                let topic = &topics[state.topic_index % topics.len()];
                info!("[LEARNER] 🔍 Scouting topic: '{}'", topic);

                // 1. Search for candidates
                let search_result = source_tools::search_youtube(topic, 5) // Increased limit
                    .await
                    .map_err(|e| e.to_string());

                match search_result {
                    Ok(results) => {
                        for source in results {
                            if !is_running.load(Ordering::SeqCst) {
                                break;
                            }

                            // Check if already processed
                            if let Some(url) = &source.original_url {
                                if state.processed_urls.contains(url) {
                                    continue;
                                }
                            }

                            // Filter criteria (e.g., duration < 10 mins to be quick)
                            if source.duration > 60.0 && source.duration < 900.0 {
                                // Increased max duration
                                // 1b. Safety Check URL
                                if let Some(url) = &source.original_url {
                                    if let Err(e) =
                                        crate::agent::download_guard::DownloadGuard::validate_url(
                                            url,
                                        )
                                    {
                                        error!("[LEARNER] 🛡️ Skipped unsafe URL: {}", e);
                                        continue;
                                    }
                                }

                                info!("[LEARNER] 📥 Acquiring candidate: {}", source.title);

                                let download_dir_buf =
                                    crate::agent::video_style_learner::get_download_dir();
                                let download_dir = download_dir_buf.as_path();
                                let _ = std::fs::create_dir_all(download_dir);

                                let download_result = source_tools::download_youtube(
                                    source.original_url.as_deref().unwrap_or(""),
                                    download_dir,
                                    None,
                                )
                                .await
                                .map_err(|e| e.to_string());

                                match download_result {
                                    Ok(downloaded) => {
                                        // 1c. Safety Check File
                                        if let Err(e) = crate::agent::download_guard::DownloadGuard::validate_downloaded_file(&downloaded.local_path) {
                                            error!("[LEARNER] 🛡️ Downloaded file rejected: {}", e);
                                            let _ = std::fs::remove_file(&downloaded.local_path);
                                            continue;
                                        }

                                        info!(
                                            "[LEARNER] 🎓 New video acquired: '{}'",
                                            downloaded.title
                                        );

                                        // ── Full style-learning pass ──────────────────────────────────────
                                        // Run video_style_learner on the entire Download folder.
                                        // Existing videos use their cached profiles (instant, no XP).
                                        // The newly downloaded file gets real scene detection + XP.
                                        // Eviction happens AFTER the new video is fully memorized.
                                        let mut brain_lock = brain.lock().await;

                                        let result = crate::agent::video_style_learner::learn_from_downloads(
                                            &mut brain_lock,
                                        )
                                        .await;

                                        if result.has_new {
                                            crate::agent::video_style_learner::synthesise_and_save_strategy(
                                                &result.profiles,
                                            );
                                            info!(
                                                "[LEARNER] 🎨 EditingStrategy updated from {} profile(s)",
                                                result.profiles.len()
                                            );
                                        }

                                        let speed = brain_lock.neuroplasticity.current_speed();
                                        let level = brain_lock.neuroplasticity.adaptation_level();
                                        let sleep_duration =
                                            brain_lock.neuroplasticity.adaptive_delay_secs(30);

                                        drop(brain_lock);
                                        // ── End style-learning pass ───────────────────────────────────────

                                        // Mark URL as processed so we never re-download it
                                        if let Some(url) = &source.original_url {
                                            state.processed_urls.insert(url.clone());
                                        }

                                        // ── Enforce 10-video cap ──────────────────────────────────────────
                                        // Find and evict the lowest-quality video from disk + cache,
                                        // but only if we're over the limit — and never delete the
                                        // video we just finished learning.
                                        let new_path_str =
                                            downloaded.local_path.to_string_lossy().to_string();

                                        if result.profiles.len()
                                            > crate::agent::video_style_learner::MAX_VIDEOS
                                        {
                                            if let Some(lowest) = result
                                                .profiles
                                                .iter()
                                                .filter(|p| p.path != new_path_str)
                                                .min_by(|a, b| {
                                                    a.outcome_xp
                                                        .partial_cmp(&b.outcome_xp)
                                                        .unwrap_or(std::cmp::Ordering::Equal)
                                                })
                                            {
                                                let lowest_path =
                                                    std::path::Path::new(&lowest.path);
                                                let lowest_filename = lowest_path
                                                    .file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or("");

                                                let _ = std::fs::remove_file(lowest_path);
                                                crate::agent::video_style_learner::remove_from_cache(
                                                    lowest_filename,
                                                );
                                                state
                                                    .downloaded_videos
                                                    .retain(|v| v.path != lowest.path);

                                                info!(
                                                    "[LEARNER] 🗑️ Evicted lowest-quality video (xp={:.2}): {}",
                                                    lowest.outcome_xp, lowest.path
                                                );
                                            }
                                        }

                                        // Record the new video in state tracking
                                        let new_score = result
                                            .profiles
                                            .iter()
                                            .find(|p| p.path == new_path_str)
                                            .map(|p| p.outcome_xp * 5.0)
                                            .unwrap_or(4.0);
                                        state.downloaded_videos.push(VideoRecord {
                                            path: new_path_str,
                                            score: new_score,
                                        });

                                        state.save(&instance_id);

                                        info!(
                                            "[LEARNER] ✅ '{}' learned & memorized (Speed: {:.1}× - {})",
                                            downloaded.title, speed, level
                                        );

                                        // Adaptive sleep — release locks first
                                        drop(state);

                                        info!(
                                            "[LEARNER] 💤 Resting for {}s (Adaptive)",
                                            sleep_duration
                                        );
                                        tokio::time::sleep(Duration::from_secs(sleep_duration))
                                            .await;

                                        // Re-lock state for loop continuation
                                        state = state_arc.lock().await;
                                    }
                                    Err(e) => {
                                        error!("[LEARNER] Failed download: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => error!("[LEARNER] Search failed for topic '{}': {}", topic, e),
                }

                // 2. Interleaved Code Analysis (Stealthy)
                // Random chance or round-robin to scan a repo file
                if cycle_count % 3 == 0 && !state.known_repos.is_empty() {
                    let repo_url = &state.known_repos[state.repo_index % state.known_repos.len()];
                    info!(
                        "[LEARNER] 🕵️ Switching mode: Stealth Analysis on {}",
                        repo_url
                    );

                    match scanner.scan_remote_code(repo_url).await {
                        Ok(concept) => {
                            info!(
                                "[LEARNER] 💡 Discovered Logic: '{}' ({})",
                                concept.logic_summary, concept.file_type
                            );

                            // Memorize the abstract concept
                            let mut brain_lock = brain.lock().await;
                            // We map this to a "Conceptual" pattern
                            let pattern = crate::agent::learning::EditingPattern {
                                intent_tag: format!("algo_{}", concept.file_type),
                                avg_scene_duration: 0.0, // N/A
                                transition_speed: 1.0,
                                music_sync_strictness: 0.0,
                                color_grade_style: "algorithmic".to_string(),
                                success_rating: 5,
                                source_video: Some(repo_url.clone()),
                                kept_ratio: 0.5,
                                outcome_xp: 0.9,
                            };
                            brain_lock
                                .learning_kernel
                                .memorize(&format!("algo_{}", concept.file_type), pattern);
                            brain_lock.neuroplasticity.record_success();
                            info!("[LEARNER] 🧠 Integrated concept into neuroplasticity network.");

                            // Advance repo index
                            state.repo_index += 1;
                            state.save(&instance_id);
                        }
                        Err(e) => {
                            warn!("[LEARNER] Analysis skipped (Stealth Mode/Limit): {}", e);
                        }
                    }
                }

                // 3. Interleaved Theory Learning (Wikipedia)
                if cycle_count % 3 == 1 {
                    let wiki_url = &wikis[cycle_count % wikis.len()];
                    info!("[LEARNER] 📖 Studying Theory: {}", wiki_url);

                    // Use Wikipedia REST API for plain text extract (Send-safe, no scraper)
                    let title = wiki_url.rsplit('/').next().unwrap_or("Film_editing");
                    let api_url = format!(
                        "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
                        title
                    );

                    match reqwest::get(&api_url).await {
                        Ok(resp) => {
                            if let Ok(text) = resp.text().await {
                                // Extract the "extract" field from the JSON
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                    let extract = json["extract"].as_str().unwrap_or("No content");
                                    info!("[LEARNER] 📖 Read: {} ({} chars)", title, extract.len());

                                    let mut brain_lock = brain.lock().await;
                                    let mem_pattern = crate::agent::learning::EditingPattern {
                                        intent_tag: format!("theory_{}", title),
                                        avg_scene_duration: 0.0,
                                        transition_speed: 1.0,
                                        music_sync_strictness: 0.0,
                                        color_grade_style: "theoretical".to_string(),
                                        success_rating: 5,
                                        source_video: Some(wiki_url.clone()),
                                        kept_ratio: 0.5,
                                        outcome_xp: 0.85,
                                    };
                                    brain_lock
                                        .learning_kernel
                                        .memorize(&format!("theory_{}", title), mem_pattern);
                                    brain_lock.neuroplasticity.record_success();
                                    info!("[LEARNER] 🎓 Absorbed theory on '{}'", title);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("[LEARNER] Theory study failed: {}", e);
                        }
                    }
                }

                // 4. Free Web Scouting (DuckDuckGo Lite)
                if cycle_count % 5 == 2 {
                    let search_topic = format!("{} editing techniques tips blog", topic);
                    info!(
                        "[LEARNER] 🕵️ Scouting the web for keywords: '{}'",
                        search_topic
                    );

                    match source_tools::web_search(&search_topic).await {
                        Ok(results) => {
                            for (res_title, snippet) in results {
                                info!("[LEARNER] 📖 Scouted: {} - {}", res_title, snippet);
                                // Synthesize knowledge from snippet
                                let mut brain_lock = brain.lock().await;
                                let tag =
                                    format!("web_{}", res_title.replace(" ", "_").to_lowercase());
                                let pattern = crate::agent::learning::EditingPattern {
                                    intent_tag: tag.clone(),
                                    avg_scene_duration: 0.0,
                                    transition_speed: 1.0,
                                    music_sync_strictness: 0.0,
                                    color_grade_style: "learned_from_web".to_string(),
                                    success_rating: 4,
                                    source_video: Some(res_title.clone()),
                                    kept_ratio: 0.5,
                                    outcome_xp: 0.7,
                                };
                                brain_lock.learning_kernel.memorize(&tag, pattern);
                                brain_lock.neuroplasticity.record_success();
                            }
                        }
                        Err(e) => warn!("[LEARNER] Web scout failed: {}", e),
                    }
                }

                state.topic_index += 1;
                state.save(&instance_id);

                info!(
                    "[LEARNER] ✅ Cycle #{} Summary: Topic '{}' processed. Next cycle in 30s.",
                    cycle_count, topic
                );

                // Release state lock before long sleep
                drop(state);

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

    /// NEW: Learn from a recently completed manual or queued edit job
    pub async fn learn_from_edit(
        &self,
        intent: &str,
        input_path: &std::path::Path,
        duration: f64,
        kept_ratio: f64,
    ) {
        info!(
            "[LEARNER] 📈 Analyzing completed edit: '{}' (Duration: {:.2}s, Kept Ratio: {:.2})",
            intent, duration, kept_ratio
        );

        // 1. Scene density analysis of the result
        let mut avg_scene_duration = duration / 5.0; // Default fallback
        if let Ok(scenes) = smart_editor::detect_scenes(input_path, 0.4).await {
            if !scenes.is_empty() {
                avg_scene_duration = duration / scenes.len() as f64;
                info!(
                    "[LEARNER] 📊 Feedback: Detected {} scenes, avg duration: {:.2}s",
                    scenes.len(),
                    avg_scene_duration
                );
            }
        }

        // Calculate Quality based on kept_ratio.
        // A good edit is balanced (ratio 0.3-0.7 gives 1.0 XP). Too low or high means extreme edits, which give less XP.
        let quality = if kept_ratio >= 0.3 && kept_ratio <= 0.7 {
            1.0
        } else if kept_ratio < 0.15 || kept_ratio > 0.9 {
            0.2
        } else {
            0.6 // Moderate
        };

        let mut brain_lock = self.brain.lock().await;

        // Record success in neuroplasticity with quality weight
        brain_lock
            .neuroplasticity
            .record_success_with_quality(quality);

        // Extract style if possible or just update the frequency of the intent
        let pattern = crate::agent::learning::EditingPattern {
            intent_tag: intent.to_string(),
            avg_scene_duration,
            transition_speed: if avg_scene_duration < 2.0 { 1.5 } else { 1.0 },
            music_sync_strictness: 0.6,
            color_grade_style: "feedback_learned".to_string(),
            success_rating: 5,
            source_video: Some(input_path.to_string_lossy().to_string()),
            kept_ratio,
            outcome_xp: quality,
        };

        brain_lock.learning_kernel.memorize(intent, pattern);
        info!(
            "[LEARNER] 🧠 Knowledge base updated with feedback from '{}' (Quality XP: {:.2})",
            intent, quality
        );

        // Potential: If duration was very short, maybe speed up the next one?
        if duration < 10.0 {
            info!("[LEARNER] ⚡ Detecting fast workflow. Boosting adaptive speed.");
            brain_lock
                .neuroplasticity
                .record_success_with_quality(quality); // Double boost
        }
    }
}
