// SYNOID Video Editing Agent - Orchestrator for ML-Driven Betterment
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::autonomous_learner::AutonomousLearner;
use crate::agent::brain::Brain;
use crate::agent::smart_editor;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use std::path::Path;

#[derive(Clone)]
pub struct VideoEditingAgent {
    brain: Arc<Mutex<Brain>>,
    learner: Arc<AutonomousLearner>,
}

impl VideoEditingAgent {
    pub fn new(brain: Arc<Mutex<Brain>>) -> Self {
        let learner = Arc::new(AutonomousLearner::new(brain.clone()));
        Self {
            brain,
            learner,
        }
    }

    /// Run a "Betterment Cycle": Find a benchmark, learn from it, then try to apply it
    pub async fn run_betterment_cycle(&self, topic: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("[VEA] 🚀 Starting Betterment Cycle for topic: '{}'", topic);

        // 1. Trigger the learner to find and acquire a benchmark
        // Since the learner runs as a background loop, we could theoretically just wait for it to populate memory,
        // or we can manually trigger a "one-off" learning event.
        // For now, let's assume the learner is already running.

        // 2. Retrieve the best pattern for this topic
        let pattern = {
            let brain_lock = self.brain.lock().await;
            brain_lock.learning_kernel.recall_pattern(topic)
        };

        info!("[VEA] 🧠 Recalled pattern: '{}' (S: {:.2}s, T: {:.2}x)", 
            pattern.intent_tag, pattern.avg_scene_duration, pattern.transition_speed);

        Ok(())
    }

    /// Smart Edit wrapper that automatically applies the best learned pattern
    pub async fn intelligent_edit(
        &self,
        input: &Path,
        instruction: &str,
        output: &Path,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        info!("[VEA] 🎨 Performing Intelligent Edit: '{}'", instruction);

        // 1. Recall best pattern based on instruction
        let pattern = {
            let brain_lock = self.brain.lock().await;
            brain_lock.learning_kernel.recall_pattern(instruction)
        };

        // 2. Perform Smart Edit with the pattern
        let result = smart_editor::smart_edit(
            input,
            instruction,
            output,
            false, // funny_mode
            None,  // progress_callback
            None,  // pre_scanned_scenes
            None,  // pre_scanned_transcript
            Some(pattern),
        ).await?;

        // 3. Feed back the result to the learner
        // (Wait for render to finish, then analyze the duration/pacing of the output)
        if let Ok(metadata) = std::fs::metadata(output) {
            if metadata.len() > 0 {
                // In a real scenario, we'd get the actual video duration here
                // For now, we use a mock or heuristic if needed
                self.learner.learn_from_edit(instruction, output, 30.0).await;
            }
        }

        Ok(result)
    }

    pub fn start_autonomous_learning(&self) {
        self.learner.start();
    }

    pub fn stop_autonomous_learning(&self) {
        self.learner.stop();
    }
}
