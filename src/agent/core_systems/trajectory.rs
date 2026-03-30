// SYNOID Trajectory Store — Episode Recording for GEPA
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Inspired by NousResearch/hermes-agent trajectory tracking:
// Every edit session is captured as a "trajectory episode" — a JSONL record
// containing the intent, pattern used, quality score, and outcome. This creates
// a persistent dataset the GEPA PolicyUpdater can mine to improve future edits.
//
// Storage format: one JSON object per line in cortex_cache/trajectories.jsonl
// This mirrors hermes-agent's approach of separating successful vs failed
// trajectories and preserving full reasoning context for downstream analysis.

#![allow(dead_code)]

use crate::agent::core_systems::learning::EditingPattern;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

// ─── Episode Record ──────────────────────────────────────────────────────────

/// A single completed edit session captured as a GEPA trajectory episode.
/// Mirrors hermes-agent's trajectory format: intent + context + outcome.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EditTrajectory {
    /// Unique episode ID (timestamp-based).
    pub id: String,
    /// Unix timestamp when the edit was completed.
    pub timestamp: u64,
    /// The user's creative intent / instruction.
    pub intent: String,
    /// Source video path (relative or absolute).
    pub input_path: String,
    /// Output video path if successful.
    pub output_path: Option<String>,
    /// Which EditingPattern was applied for this edit.
    pub pattern_used: Option<EditingPattern>,
    /// Number of scenes detected in the input.
    pub scenes_detected: usize,
    /// Fraction of scenes kept (0.0 = all cut, 1.0 = all kept).
    pub kept_ratio: f64,
    /// Duration of input video in seconds.
    pub duration_secs: f64,
    /// GEPA goal score: composite quality 0.0–1.0.
    pub goal_score: f64,
    /// Whether the edit completed without error.
    pub success: bool,
    /// Optional free-text note (error message, quality observation).
    pub notes: String,
}

impl EditTrajectory {
    /// Create a new trajectory episode with a timestamp-based ID.
    pub fn new(
        intent: impl Into<String>,
        input_path: impl Into<String>,
        output_path: Option<String>,
        pattern_used: Option<EditingPattern>,
        scenes_detected: usize,
        kept_ratio: f64,
        duration_secs: f64,
        goal_score: f64,
        success: bool,
        notes: impl Into<String>,
    ) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self {
            id: format!("ep_{}", ts),
            timestamp: (ts / 1000) as u64,
            intent: intent.into(),
            input_path: input_path.into(),
            output_path,
            pattern_used,
            scenes_detected,
            kept_ratio,
            duration_secs,
            goal_score,
            success,
            notes: notes.into(),
        }
    }
}

// ─── Trajectory Store ────────────────────────────────────────────────────────

/// Persists edit trajectories as JSONL for later policy analysis.
/// Mirrors hermes-agent's session recording: each line = one episode.
pub struct TrajectoryStore {
    path: PathBuf,
}

impl TrajectoryStore {
    pub fn new(instance_id: &str) -> Self {
        let dir = format!("cortex_cache{}", instance_id);
        let _ = fs::create_dir_all(&dir);
        Self {
            path: PathBuf::from(dir).join("trajectories.jsonl"),
        }
    }

    /// Append a trajectory episode to the JSONL store.
    pub fn record(&self, traj: &EditTrajectory) {
        match serde_json::to_string(traj) {
            Ok(line) => {
                match OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.path)
                {
                    Ok(mut file) => {
                        if let Err(e) = writeln!(file, "{}", line) {
                            error!("[TRAJECTORY] Failed to write episode: {}", e);
                        } else {
                            info!(
                                "[TRAJECTORY] Recorded episode '{}' (score: {:.2}, success: {})",
                                traj.id, traj.goal_score, traj.success
                            );
                        }
                    }
                    Err(e) => error!("[TRAJECTORY] Cannot open store: {}", e),
                }
            }
            Err(e) => error!("[TRAJECTORY] Serialization failed: {}", e),
        }
    }

    /// Load all recorded trajectories from disk.
    pub fn load_all(&self) -> Vec<EditTrajectory> {
        if !self.path.exists() {
            return Vec::new();
        }
        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(e) => {
                warn!("[TRAJECTORY] Cannot read store: {}", e);
                return Vec::new();
            }
        };
        BufReader::new(file)
            .lines()
            .filter_map(|line| {
                let line = line.ok()?;
                serde_json::from_str::<EditTrajectory>(&line).ok()
            })
            .collect()
    }

    /// Return the top-k trajectories by goal_score (best edits first).
    pub fn best_trajectories(&self, top_k: usize) -> Vec<EditTrajectory> {
        let mut all = self.load_all();
        all.sort_by(|a, b| b.goal_score.partial_cmp(&a.goal_score).unwrap_or(std::cmp::Ordering::Equal));
        all.truncate(top_k);
        all
    }

    /// Filter trajectories that match a given intent keyword.
    pub fn filter_by_intent(&self, intent: &str) -> Vec<EditTrajectory> {
        let intent_lower = intent.to_lowercase();
        self.load_all()
            .into_iter()
            .filter(|t| t.intent.to_lowercase().contains(&intent_lower))
            .collect()
    }

    /// Return only successful trajectories.
    pub fn successful(&self) -> Vec<EditTrajectory> {
        self.load_all().into_iter().filter(|t| t.success).collect()
    }

    /// Total number of recorded episodes.
    pub fn episode_count(&self) -> usize {
        self.load_all().len()
    }

    /// Compute the rolling average goal_score over the last N episodes.
    pub fn rolling_avg_score(&self, window: usize) -> f64 {
        let all = self.load_all();
        if all.is_empty() {
            return 0.0;
        }
        let recent: Vec<f64> = all
            .iter()
            .rev()
            .take(window)
            .map(|t| t.goal_score)
            .collect();
        recent.iter().sum::<f64>() / recent.len() as f64
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trajectory_round_trip() {
        let traj = EditTrajectory::new(
            "cinematic travel",
            "/tmp/input.mp4",
            Some("/tmp/output.mp4".to_string()),
            None,
            12,
            0.45,
            120.0,
            0.87,
            true,
            "test episode",
        );
        let json = serde_json::to_string(&traj).unwrap();
        let parsed: EditTrajectory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.intent, "cinematic travel");
        assert_eq!(parsed.scenes_detected, 12);
        assert!((parsed.goal_score - 0.87).abs() < 1e-9);
    }

    #[test]
    fn rolling_avg_empty_store() {
        let store = TrajectoryStore {
            path: PathBuf::from("/tmp/nonexistent_trajectories.jsonl"),
        };
        assert_eq!(store.rolling_avg_score(10), 0.0);
    }
}
