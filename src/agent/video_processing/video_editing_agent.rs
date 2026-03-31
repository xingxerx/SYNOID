// SYNOID Video Editing Agent — GEPA-enhanced orchestrator
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// GEPA integration: every call to `intelligent_edit` is recorded as a
// trajectory episode. The GepaLoop runs in the background to distil the
// best-performing patterns back into the LearningKernel so future edits
// automatically benefit from accumulated experience.

use crate::agent::core_systems::autonomous_learner::AutonomousLearner;
use crate::agent::core_systems::brain::Brain;
use crate::agent::core_systems::gepa::GepaLoop;
use crate::agent::specialized::smart_editor;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone)]
pub struct VideoEditingAgent {
    brain: Arc<Mutex<Brain>>,
    learner: Arc<AutonomousLearner>,
    animator: Arc<crate::agent::video_processing::animator::Animator>,
    /// GEPA loop: records trajectory episodes and updates the editing policy.
    pub gepa: Arc<GepaLoop>,
}

impl VideoEditingAgent {
    pub fn new(
        brain: Arc<Mutex<Brain>>,
        instance_id: &str,
        animator: Arc<crate::agent::video_processing::animator::Animator>,
    ) -> Self {
        let learner = Arc::new(AutonomousLearner::new(brain.clone(), instance_id));
        let gepa = Arc::new(GepaLoop::new(brain.clone(), instance_id));
        Self {
            brain,
            learner,
            animator,
            gepa,
        }
    }

    /// Run a "Betterment Cycle": recall the best known pattern for a topic.
    pub async fn run_betterment_cycle(
        &self,
        topic: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("[VEA] Starting Betterment Cycle for topic: '{}'", topic);

        let kernel = self.brain.lock().await.learning_kernel.clone();
        let pattern = kernel.lock().await.recall_pattern(topic);

        info!(
            "[VEA] Recalled pattern: '{}' (scene: {:.2}s, transition: {:.2}x, XP: {:.2})",
            pattern.intent_tag,
            pattern.avg_scene_duration,
            pattern.transition_speed,
            pattern.outcome_xp,
        );

        Ok(())
    }

    /// Perform an intent-driven edit, automatically applying the best learned
    /// pattern and recording the result as a GEPA trajectory episode.
    pub async fn intelligent_edit(
        &self,
        input: &Path,
        instruction: &str,
        output: &Path,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        info!("[VEA] Performing Intelligent Edit: '{}'", instruction);

        // 1. Recall best pattern for this instruction
        let kernel = self.brain.lock().await.learning_kernel.clone();
        let pattern = kernel.lock().await.recall_pattern(instruction);

        // 2. Smart edit with the recalled pattern
        let result = smart_editor::smart_edit(
            input,
            instruction,
            output,
            false, // funny_mode
            None,  // progress_callback
            None,  // pre_scanned_scenes
            None,  // pre_scanned_transcript
            Some(pattern.clone()),
            Some(self.animator.clone()),
            true,
        )
        .await;

        let (success, notes) = match &result {
            Ok(_) => (true, String::new()),
            Err(e) => (false, e.to_string()),
        };

        // 3. Feed result to the legacy learner (kept_ratio heuristic)
        if success {
            if let Ok(meta) = std::fs::metadata(output) {
                if meta.len() > 0 {
                    self.learner
                        .learn_from_edit(instruction, output, 30.0, 0.5)
                        .await;
                }
            }
        }

        // 4. GEPA: record trajectory episode and run policy update
        //    Detect scenes from the output (or input on failure) for accurate metrics
        let (scene_count, kept_ratio, duration_secs) =
            self.extract_edit_metrics(input, output, success).await;

        let goal_score = self
            .gepa
            .record_episode(
                instruction,
                input,
                if success { Some(output) } else { None },
                Some(pattern),
                scene_count,
                kept_ratio,
                duration_secs,
                success,
                &notes,
            )
            .await;

        info!(
            "[VEA] GEPA episode recorded — goal_score: {:.3}, scenes: {}, kept: {:.0}%",
            goal_score,
            scene_count,
            kept_ratio * 100.0
        );

        // 5. Trigger a policy update every 5 successful edits
        let ep_count = self.gepa.store.episode_count();
        if ep_count > 0 && ep_count % 5 == 0 {
            info!("[VEA] GEPA: triggering policy update at episode {}", ep_count);
            self.gepa.run_policy_update().await;
        }

        result
    }

    /// Derive scene count, kept_ratio, and duration from the output video.
    /// Falls back to safe defaults if ffprobe/scene-detect is unavailable.
    async fn extract_edit_metrics(
        &self,
        input: &Path,
        output: &Path,
        success: bool,
    ) -> (usize, f64, f64) {
        let probe_path = if success && output.exists() {
            output
        } else {
            input
        };

        // Try scene detection on the output to get accurate pacing metrics
        let scene_count = smart_editor::detect_scenes(probe_path, 0.4)
            .await
            .map(|s| s.len())
            .unwrap_or(0);

        // Estimate duration via metadata (file size proxy) — replace with ffprobe if available
        let duration_secs = std::fs::metadata(probe_path)
            .map(|m| {
                // Very rough: ~1 MB/s for compressed video, capped for sanity
                let mb = m.len() as f64 / 1_048_576.0;
                (mb * 1.0).max(5.0).min(3600.0)
            })
            .unwrap_or(30.0);

        // kept_ratio: if we have both input and output scene counts, compare them
        let input_scenes = if success && output != input {
            smart_editor::detect_scenes(input, 0.4)
                .await
                .map(|s| s.len())
                .unwrap_or(scene_count)
        } else {
            scene_count
        };

        let kept_ratio = if input_scenes > 0 {
            (scene_count as f64 / input_scenes as f64).min(1.0)
        } else {
            0.5
        };

        (scene_count, kept_ratio, duration_secs)
    }

    /// Start the autonomous learning loop (downloads + style learning).
    pub fn start_autonomous_learning(&self) {
        self.learner.start();
    }

    /// Stop the autonomous learning loop.
    pub fn stop_autonomous_learning(&self) {
        self.learner.stop();
    }

    /// Start the GEPA background improvement loop.
    /// The loop runs policy_update every `interval_secs` seconds.
    pub fn start_gepa_loop(self: &Arc<Self>, interval_secs: u64) {
        self.gepa.clone().start_background_loop(interval_secs);
    }

    /// Stop the GEPA background loop.
    pub fn stop_gepa_loop(&self) {
        self.gepa.stop_background_loop();
    }
}
