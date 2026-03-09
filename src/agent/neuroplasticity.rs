// SYNOID Neuroplasticity — Adaptive Speed Doubling
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// The Brain grows faster with experience. Processing speed doubles
// at fixed experience thresholds, modelling biological neuroplasticity
// where repeated pathways become faster over time.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

/// Experience thresholds at which speed doubles.
/// At 50 tasks → 2×, 100 → 4×, 150 → 8×, 200 → 16× (cap).
const DOUBLING_INTERVAL: u64 = 50;
const MAX_MULTIPLIER: f64 = 16.0;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Neuroplasticity {
    /// Total successful operations processed.
    pub experience_points: u64,
    /// Fractional XP buffer for quality-weighted accumulation.
    #[serde(default)]
    pub experience_points_f64: f64,
    /// Current speed multiplier (starts at 1.0, doubles per threshold).
    pub speed_multiplier: f64,
    /// Unix timestamp when this instance was first created.
    pub created_at: u64,
    /// Total adaptation events (number of doublings that have occurred).
    pub adaptations: u32,
}

impl Neuroplasticity {
    /// Load from disk or create a fresh instance.
    pub fn new() -> Self {
        let path = Self::persistence_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(state) = serde_json::from_str::<Neuroplasticity>(&data) {
                    info!(
                        "[NEUROPLASTICITY] 🧠 Restored: {} XP, {:.1}× speed ({})",
                        state.experience_points,
                        state.speed_multiplier,
                        state.adaptation_level()
                    );
                    return state;
                }
            }
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let fresh = Self {
            experience_points: 0,
            experience_points_f64: 0.0,
            speed_multiplier: 1.0,
            created_at: now,
            adaptations: 0,
        };

        info!("[NEUROPLASTICITY] 🌱 Fresh brain initialized — speed 1.0×");
        fresh
    }

    /// Record a successful task completion and potentially increase speed.
    pub fn record_success(&mut self) {
        self.record_success_with_quality(1.0);
    }

    /// Record a success weighted by edit quality.
    ///
    /// `quality` should be in [0.0, 1.0] where:
    /// - 1.0 = ideal balanced edit (kept_ratio 0.3–0.7)
    /// - 0.5 = mediocre edit
    /// - 0.1 = poor/extreme edit (cut everything or kept everything)
    ///
    /// Fractional XP accumulates in a buffer and is flushed to
    /// `experience_points` when it crosses a whole number.
    pub fn record_success_with_quality(&mut self, quality: f64) {
        let xp_gain = quality.clamp(0.0, 1.0).max(0.1); // Minimum 0.1 XP always
        self.experience_points_f64 += xp_gain;

        // Flush whole XP units from the buffer
        let gained_whole = self.experience_points_f64.floor() as u64;
        if gained_whole > 0 {
            self.experience_points += gained_whole;
            self.experience_points_f64 -= gained_whole as f64;
        }

        let new_multiplier = self.calculate_multiplier();
        if (new_multiplier - self.speed_multiplier).abs() > f64::EPSILON {
            self.adaptations += 1;
            info!(
                "[NEUROPLASTICITY] ⚡ ADAPTATION #{}: Speed {:.1}× → {:.1}× (at {} XP, quality={:.2})",
                self.adaptations, self.speed_multiplier, new_multiplier, self.experience_points, xp_gain
            );
            self.speed_multiplier = new_multiplier;
        }

        self.save();
    }

    /// Current speed multiplier.
    pub fn current_speed(&self) -> f64 {
        self.speed_multiplier
    }

    /// Human-readable adaptation tier.
    pub fn adaptation_level(&self) -> &'static str {
        match self.speed_multiplier as u32 {
            0..=1 => "Baseline",
            2..=3 => "Accelerated",
            4..=7 => "Hyperspeed",
            8..=15 => "Neural Overdrive",
            _ => "Singularity",
        }
    }

    /// Calculate the multiplier from raw experience points.
    fn calculate_multiplier(&self) -> f64 {
        if self.experience_points == 0 {
            return 1.0;
        }

        let doublings = self.experience_points / DOUBLING_INTERVAL;
        let raw = 2.0_f64.powi(doublings as i32);
        raw.min(MAX_MULTIPLIER)
    }

    /// Compute an adaptive sleep duration — faster brains sleep less.
    /// Takes a base duration in seconds and divides by the speed multiplier.
    pub fn adaptive_delay_secs(&self, base_secs: u64) -> u64 {
        let adjusted = (base_secs as f64) / self.speed_multiplier;
        // Floor at 2 seconds minimum to avoid hammering
        (adjusted as u64).max(2)
    }

    fn persistence_path() -> PathBuf {
        let suffix = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_default();
        let dir = PathBuf::from(format!("cortex_cache{}", suffix));
        let _ = fs::create_dir_all(&dir);
        dir.join("neuroplasticity.json")
    }

    // -------------------------------------------------------------------
    // GPU-aware adaptive parameters
    // -------------------------------------------------------------------

    /// Batch size scaling factor for CUDA workloads.
    /// Higher experience → larger GPU batches.
    pub fn gpu_batch_multiplier(&self) -> usize {
        (self.speed_multiplier as usize).max(1)
    }

    /// Recommended FFmpeg thread count based on neuroplasticity level.
    pub fn gpu_thread_count(&self) -> usize {
        let base = num_cpus::get();
        let scaled = (base as f64 * self.speed_multiplier).min(base as f64 * 2.0);
        (scaled as usize).max(1)
    }

    /// Formatted acceleration report combining speed + adaptation level.
    pub fn acceleration_report(&self) -> String {
        format!(
            "⚡ {:.1}× speed | {} | {} XP | {} adaptations",
            self.speed_multiplier,
            self.adaptation_level(),
            self.experience_points,
            self.adaptations,
        )
    }

    fn save(&self) {
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = fs::write(Self::persistence_path(), data);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Neuroplasticity {
        Neuroplasticity {
            experience_points: 0,
            experience_points_f64: 0.0,
            speed_multiplier: 1.0,
            created_at: 0,
            adaptations: 0,
        }
    }

    #[test]
    fn test_speed_starts_at_one() {
        let np = fresh();
        assert!((np.current_speed() - 1.0).abs() < f64::EPSILON);
        assert_eq!(np.adaptation_level(), "Baseline");
    }

    #[test]
    fn test_speed_doubles_at_threshold() {
        let mut np = fresh();
        // Simulate 50 successes
        for _ in 0..50 {
            np.record_success();
        }
        assert!((np.current_speed() - 2.0).abs() < f64::EPSILON);
        assert_eq!(np.adaptation_level(), "Accelerated");
    }

    #[test]
    fn test_speed_quadruples() {
        let mut np = fresh();
        for _ in 0..100 {
            np.record_success();
        }
        assert!((np.current_speed() - 4.0).abs() < f64::EPSILON);
        assert_eq!(np.adaptation_level(), "Hyperspeed");
    }

    #[test]
    fn test_speed_caps_at_max() {
        let mut np = fresh();
        for _ in 0..500 {
            np.record_success();
        }
        assert!(np.current_speed() <= MAX_MULTIPLIER);
        assert_eq!(np.adaptation_level(), "Singularity");
    }

    #[test]
    fn test_adaptive_delay() {
        let mut np = fresh();
        assert_eq!(np.adaptive_delay_secs(30), 30);

        // At 2× speed, 30s base → 15s
        np.speed_multiplier = 2.0;
        assert_eq!(np.adaptive_delay_secs(30), 15);

        // At 16× speed, 30s base → 2s (floor)
        np.speed_multiplier = 16.0;
        assert_eq!(np.adaptive_delay_secs(30), 2);
    }
}
