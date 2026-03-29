// SYNOID GEPA — Goal-Experience-Policy-Agent Self-Improvement Loop
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Architecture inspired by NousResearch/hermes-agent:
//
//   hermes-agent component       →   SYNOID GEPA equivalent
//   ─────────────────────────────────────────────────────────
//   trajectory.py                →   TrajectoryStore (trajectory.rs)
//   insights.py                  →   GoalEvaluator + GepaInsights
//   skills/*.md                  →   EditingPattern in LearningKernel
//   run_agent.py (improve loop)  →   GepaLoop::run_policy_update()
//
// The four pillars of GEPA:
//
//   G — Goal Evaluator   : Scores edit quality against measurable objectives
//                          (balance, timing, pattern adherence)
//   E — Experience Store : TrajectoryStore (JSONL episodes in cortex_cache/)
//   P — Policy Updater   : Synthesizes best-performing EditingPatterns from
//                          trajectory history and writes them to LearningKernel
//   A — Agent Loop       : Background task that closes G→E→P→A→G cycle
//
// Every completed edit is recorded as a trajectory episode. The GepaLoop
// periodically replays those episodes, scores them, distils the best patterns,
// and writes improved policies back to the LearningKernel — making every
// subsequent intelligent_edit measurably better.

#![allow(dead_code)]

use crate::agent::core_systems::brain::Brain;
use crate::agent::core_systems::learning::{EditingPattern, LearningKernel};
use crate::agent::core_systems::trajectory::{EditTrajectory, TrajectoryStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn};

// ─── Goal Evaluator ──────────────────────────────────────────────────────────

/// Evaluates the quality of a completed edit against GEPA objectives.
/// Mirrors hermes-agent's insights system: pattern-match trajectories to
/// derive measurable quality dimensions, then produce a composite score.
pub struct GoalEvaluator;

impl GoalEvaluator {
    pub fn new() -> Self {
        Self
    }

    /// Composite quality score 0.0–1.0 for a trajectory episode.
    ///
    /// Dimensions scored:
    ///   balance_score     — optimal kept_ratio (0.3–0.7) → full points
    ///   timing_score      — avg scene duration in cinematic sweet spot (1–8 s)
    ///   completeness      — edit produced output and ran without errors
    pub fn score_edit(&self, traj: &EditTrajectory) -> f64 {
        // 1. Balance: ideal kept_ratio 0.3–0.7
        let balance_score = if traj.kept_ratio >= 0.3 && traj.kept_ratio <= 0.7 {
            1.0
        } else if traj.kept_ratio < 0.15 || traj.kept_ratio > 0.9 {
            0.1
        } else {
            // Linear ramp in the shoulder regions (0.15–0.3) and (0.7–0.9)
            if traj.kept_ratio < 0.3 {
                (traj.kept_ratio - 0.15) / 0.15
            } else {
                (0.9 - traj.kept_ratio) / 0.2
            }
        };

        // 2. Timing: avg scene duration (duration / scenes), sweet-spot 1–8 s
        let avg_scene = if traj.scenes_detected > 0 {
            traj.duration_secs / traj.scenes_detected as f64
        } else {
            traj.duration_secs
        };
        let timing_score = if avg_scene >= 1.0 && avg_scene <= 8.0 {
            1.0
        } else if avg_scene < 0.25 || avg_scene > 30.0 {
            0.1
        } else if avg_scene < 1.0 {
            avg_scene / 1.0
        } else {
            // avg_scene > 8.0: decay
            (30.0 - avg_scene) / 22.0
        }
        .max(0.0)
        .min(1.0);

        // 3. Completeness bonus: output file exists and edit succeeded
        let completeness = if traj.success {
            1.0
        } else {
            0.0
        };

        // Weighted composite
        let composite = 0.45 * balance_score + 0.35 * timing_score + 0.20 * completeness;
        composite.max(0.0).min(1.0)
    }

    /// Detect whether the latest episode shows improvement over recent history.
    /// Returns a signed delta: positive = improving, negative = regressing.
    pub fn improvement_trend(
        &self,
        recent: &[EditTrajectory],
        window_before: usize,
        window_after: usize,
    ) -> f64 {
        if recent.len() < 2 {
            return 0.0;
        }
        let n = recent.len();
        let before_start = n.saturating_sub(window_before + window_after);
        let split = n.saturating_sub(window_after);

        let before_scores: Vec<f64> = recent[before_start..split]
            .iter()
            .map(|t| t.goal_score)
            .collect();
        let after_scores: Vec<f64> = recent[split..]
            .iter()
            .map(|t| t.goal_score)
            .collect();

        if before_scores.is_empty() || after_scores.is_empty() {
            return 0.0;
        }

        let avg_before = before_scores.iter().sum::<f64>() / before_scores.len() as f64;
        let avg_after = after_scores.iter().sum::<f64>() / after_scores.len() as f64;
        avg_after - avg_before
    }
}

// ─── Policy Updater ──────────────────────────────────────────────────────────

/// Synthesises improved EditingPatterns from trajectory history and
/// writes them back to the LearningKernel — the GEPA "policy update" step.
///
/// Mirrors hermes-agent's skill auto-generation: the best experiences become
/// re-usable skills (EditingPatterns) that the agent applies going forward.
pub struct PolicyUpdater;

impl PolicyUpdater {
    pub fn new() -> Self {
        Self
    }

    /// Group trajectories by intent, distil the best pattern per intent,
    /// and upsert it into the LearningKernel.
    pub fn synthesize_from_trajectories(
        &self,
        trajectories: &[EditTrajectory],
        kernel: &mut LearningKernel,
    ) -> usize {
        if trajectories.is_empty() {
            return 0;
        }

        // Group by intent keyword (lowercase, first 64 chars to avoid key explosion)
        let mut groups: HashMap<String, Vec<&EditTrajectory>> = HashMap::new();
        for traj in trajectories {
            let key = traj.intent.to_lowercase();
            let key = key[..key.len().min(64)].to_string();
            groups.entry(key).or_default().push(traj);
        }

        let mut updated = 0usize;
        for (intent_key, episodes) in &groups {
            // Pick the episode with the highest goal_score as the "champion"
            let champion = episodes
                .iter()
                .max_by(|a, b| {
                    a.goal_score
                        .partial_cmp(&b.goal_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap(); // groups are non-empty by construction

            if champion.goal_score < 0.4 {
                // Not good enough to become a policy — skip
                continue;
            }

            // Average scene duration from the champion episode
            let avg_scene_duration = if champion.scenes_detected > 0 {
                champion.duration_secs / champion.scenes_detected as f64
            } else {
                3.5 // fallback
            };

            // Synthesise transition speed: faster cuts for short scenes
            let transition_speed = if avg_scene_duration < 2.0 {
                1.5
            } else if avg_scene_duration > 6.0 {
                0.8
            } else {
                1.0
            };

            // Build colour-grade hint from the pattern that was used
            let color_grade_style = champion
                .pattern_used
                .as_ref()
                .map(|p| p.color_grade_style.clone())
                .unwrap_or_else(|| "gepa_synthesised".to_string());

            let gepa_pattern = EditingPattern {
                intent_tag: intent_key.clone(),
                avg_scene_duration,
                transition_speed,
                music_sync_strictness: 0.6,
                color_grade_style,
                success_rating: (champion.goal_score * 5.0).round() as u32,
                source_video: Some(champion.input_path.clone()),
                kept_ratio: champion.kept_ratio,
                outcome_xp: champion.goal_score,
            };

            kernel.memorize(intent_key, gepa_pattern);
            updated += 1;

            info!(
                "[GEPA] PolicyUpdater: upserted pattern '{}' (score: {:.2}, avg_scene: {:.1}s)",
                intent_key, champion.goal_score, avg_scene_duration
            );
        }

        info!(
            "[GEPA] PolicyUpdater: {} pattern(s) synthesised from {} episodes",
            updated,
            trajectories.len()
        );
        updated
    }
}

// ─── Insights ────────────────────────────────────────────────────────────────

/// Aggregated analytics derived from trajectory history.
/// Mirrors hermes-agent's insights.py report structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GepaInsights {
    /// Total episodes recorded.
    pub total_episodes: usize,
    /// Fraction of successful edits.
    pub success_rate: f64,
    /// Mean goal_score across all episodes.
    pub avg_goal_score: f64,
    /// Intent tag with the highest mean goal_score.
    pub best_intent: String,
    /// Intent tag with the lowest mean goal_score.
    pub worst_intent: String,
    /// Rolling improvement trend over last 20 vs previous 20 episodes.
    pub improvement_trend: f64,
    /// Number of distinct intent categories recorded.
    pub intent_diversity: usize,
    /// Average scenes per edit.
    pub avg_scenes_per_edit: f64,
    /// Optimal kept_ratio observed in top-quartile edits.
    pub optimal_kept_ratio: f64,
}

impl GepaInsights {
    pub fn compute(trajectories: &[EditTrajectory]) -> Self {
        if trajectories.is_empty() {
            return Self {
                total_episodes: 0,
                success_rate: 0.0,
                avg_goal_score: 0.0,
                best_intent: "none".to_string(),
                worst_intent: "none".to_string(),
                improvement_trend: 0.0,
                intent_diversity: 0,
                avg_scenes_per_edit: 0.0,
                optimal_kept_ratio: 0.5,
            };
        }

        let total = trajectories.len();
        let successes = trajectories.iter().filter(|t| t.success).count();
        let success_rate = successes as f64 / total as f64;
        let avg_goal = trajectories.iter().map(|t| t.goal_score).sum::<f64>() / total as f64;

        // Per-intent averages
        let mut intent_scores: HashMap<String, Vec<f64>> = HashMap::new();
        for t in trajectories {
            let key = t.intent.to_lowercase();
            let key = key[..key.len().min(64)].to_string();
            intent_scores.entry(key).or_default().push(t.goal_score);
        }

        let intent_avgs: HashMap<String, f64> = intent_scores
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().sum::<f64>() / v.len() as f64))
            .collect();

        let best_intent = intent_avgs
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, _)| k.clone())
            .unwrap_or_default();

        let worst_intent = intent_avgs
            .iter()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, _)| k.clone())
            .unwrap_or_default();

        // Improvement trend: last 10 vs previous 10
        let evaluator = GoalEvaluator::new();
        let improvement_trend = evaluator.improvement_trend(trajectories, 10, 10);

        let intent_diversity = intent_avgs.len();
        let avg_scenes =
            trajectories.iter().map(|t| t.scenes_detected as f64).sum::<f64>() / total as f64;

        // Optimal kept_ratio from top quartile
        let mut top: Vec<&EditTrajectory> = trajectories.iter().collect();
        top.sort_by(|a, b| b.goal_score.partial_cmp(&a.goal_score).unwrap_or(std::cmp::Ordering::Equal));
        let quartile = (top.len() / 4).max(1);
        let optimal_kept_ratio =
            top[..quartile].iter().map(|t| t.kept_ratio).sum::<f64>() / quartile as f64;

        Self {
            total_episodes: total,
            success_rate,
            avg_goal_score: avg_goal,
            best_intent,
            worst_intent,
            improvement_trend,
            intent_diversity,
            avg_scenes_per_edit: avg_scenes,
            optimal_kept_ratio,
        }
    }

    pub fn print_report(&self) {
        info!("[GEPA] ═══════════════════════════════════════════");
        info!("[GEPA]  GEPA Insights Report");
        info!("[GEPA] ═══════════════════════════════════════════");
        info!("[GEPA]  Total Episodes   : {}", self.total_episodes);
        info!(
            "[GEPA]  Success Rate     : {:.1}%",
            self.success_rate * 100.0
        );
        info!(
            "[GEPA]  Avg Goal Score   : {:.3}",
            self.avg_goal_score
        );
        info!("[GEPA]  Best Intent      : {}", self.best_intent);
        info!("[GEPA]  Worst Intent     : {}", self.worst_intent);
        let trend_sign = if self.improvement_trend >= 0.0 {
            "↑"
        } else {
            "↓"
        };
        info!(
            "[GEPA]  Improvement Trend: {} {:.3}",
            trend_sign, self.improvement_trend
        );
        info!(
            "[GEPA]  Intent Diversity : {} categories",
            self.intent_diversity
        );
        info!(
            "[GEPA]  Avg Scenes/Edit  : {:.1}",
            self.avg_scenes_per_edit
        );
        info!(
            "[GEPA]  Optimal kept_ratio (top quartile): {:.2}",
            self.optimal_kept_ratio
        );
        info!("[GEPA] ═══════════════════════════════════════════");
    }
}

// ─── GEPA Loop ───────────────────────────────────────────────────────────────

/// The core GEPA coordinator that closes the G→E→P→A→G self-improvement cycle.
///
/// Usage pattern:
///   1. Call `record_episode()` after every `intelligent_edit` completes.
///   2. Call `run_policy_update()` periodically (or from the background loop).
///   3. Call `generate_insights()` to inspect how the agent is improving.
///   4. Call `start_background_loop()` for fully autonomous improvement.
pub struct GepaLoop {
    pub store: Arc<TrajectoryStore>,
    evaluator: GoalEvaluator,
    updater: PolicyUpdater,
    brain: Arc<Mutex<Brain>>,
    is_running: Arc<AtomicBool>,
    instance_id: String,
}

impl GepaLoop {
    pub fn new(brain: Arc<Mutex<Brain>>, instance_id: &str) -> Self {
        Self {
            store: Arc::new(TrajectoryStore::new(instance_id)),
            evaluator: GoalEvaluator::new(),
            updater: PolicyUpdater::new(),
            brain,
            is_running: Arc::new(AtomicBool::new(false)),
            instance_id: instance_id.to_string(),
        }
    }

    /// Record a completed edit as a GEPA trajectory episode.
    /// Returns the computed goal_score so callers can log it.
    pub async fn record_episode(
        &self,
        intent: &str,
        input: &Path,
        output: Option<&Path>,
        pattern_used: Option<EditingPattern>,
        scenes_detected: usize,
        kept_ratio: f64,
        duration_secs: f64,
        success: bool,
        notes: &str,
    ) -> f64 {
        // Build a provisional trajectory to score
        let provisional = EditTrajectory::new(
            intent,
            input.to_string_lossy().as_ref(),
            output.map(|p| p.to_string_lossy().to_string()),
            pattern_used.clone(),
            scenes_detected,
            kept_ratio,
            duration_secs,
            0.0, // placeholder — replaced below
            success,
            notes,
        );

        let goal_score = self.evaluator.score_edit(&provisional);

        // Re-create with the real score
        let traj = EditTrajectory {
            goal_score,
            ..provisional
        };

        self.store.record(&traj);

        // Feed score back to Neuroplasticity
        {
            let mut brain = self.brain.lock().await;
            brain.neuroplasticity.record_success_with_quality(goal_score);
        }

        goal_score
    }

    /// Replay stored trajectories, distil best patterns, and upsert into LearningKernel.
    /// This is the "P" step of GEPA — policy update from accumulated experience.
    pub async fn run_policy_update(&self) {
        let trajectories = self.store.successful();
        if trajectories.is_empty() {
            warn!("[GEPA] No successful trajectories yet — skipping policy update");
            return;
        }

        info!(
            "[GEPA] Running policy update from {} successful episode(s)",
            trajectories.len()
        );

        let brain = self.brain.lock().await;
        let kernel_arc = brain.learning_kernel.clone();
        drop(brain); // release brain lock before acquiring kernel

        let mut kernel = kernel_arc.lock().await;
        let updated = self.updater.synthesize_from_trajectories(&trajectories, &mut kernel);

        info!(
            "[GEPA] Policy update complete: {} pattern(s) improved",
            updated
        );
    }

    /// Generate a full insights report from trajectory history.
    pub fn generate_insights(&self) -> GepaInsights {
        let trajectories = self.store.load_all();
        let insights = GepaInsights::compute(&trajectories);
        insights.print_report();
        insights
    }

    /// Start the background GEPA improvement loop.
    /// Runs policy_update every `interval_secs` seconds.
    pub fn start_background_loop(self: Arc<Self>, interval_secs: u64) {
        if self.is_running.load(Ordering::SeqCst) {
            info!("[GEPA] Background loop already running");
            return;
        }

        self.is_running.store(true, Ordering::SeqCst);
        let running = self.is_running.clone();
        let gepa = self.clone();

        info!(
            "[GEPA] Background improvement loop started (interval: {}s)",
            interval_secs
        );

        tokio::spawn(async move {
            let mut cycle = 0u64;
            while running.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
                cycle += 1;
                info!("[GEPA] Background cycle #{}", cycle);
                gepa.run_policy_update().await;

                // Every 5 cycles print an insights summary
                if cycle % 5 == 0 {
                    gepa.generate_insights();
                }
            }
            info!("[GEPA] Background loop stopped");
        });
    }

    pub fn stop_background_loop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_traj(kept: f64, scenes: usize, dur: f64, success: bool) -> EditTrajectory {
        EditTrajectory::new(
            "test intent",
            "/tmp/in.mp4",
            None,
            None,
            scenes,
            kept,
            dur,
            0.0,
            success,
            "",
        )
    }

    #[test]
    fn goal_evaluator_optimal() {
        let ev = GoalEvaluator::new();
        let mut t = make_traj(0.5, 10, 30.0, true);
        t.goal_score = ev.score_edit(&t);
        // Optimal: kept_ratio=0.5 (balance=1), avg_scene=3s (timing=1), success=1 → 1.0
        assert!(t.goal_score > 0.95, "Expected near 1.0, got {}", t.goal_score);
    }

    #[test]
    fn goal_evaluator_failed_edit() {
        let ev = GoalEvaluator::new();
        let mut t = make_traj(0.5, 10, 30.0, false);
        t.goal_score = ev.score_edit(&t);
        // success=false → completeness=0 → score drops
        assert!(t.goal_score < 0.85);
    }

    #[test]
    fn insights_empty() {
        let i = GepaInsights::compute(&[]);
        assert_eq!(i.total_episodes, 0);
        assert_eq!(i.success_rate, 0.0);
    }

    #[test]
    fn insights_populated() {
        let ev = GoalEvaluator::new();
        let mut t1 = make_traj(0.5, 10, 30.0, true);
        let mut t2 = make_traj(0.9, 2, 5.0, false);
        t1.goal_score = ev.score_edit(&t1);
        t2.goal_score = ev.score_edit(&t2);
        let insights = GepaInsights::compute(&[t1, t2]);
        assert_eq!(insights.total_episodes, 2);
        assert!((insights.success_rate - 0.5).abs() < 1e-9);
    }
}
