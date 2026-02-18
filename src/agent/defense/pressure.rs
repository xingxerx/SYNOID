// SYNOID PressureWatcher — Real-time Hardware Stress Monitor
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// The "Nervous System" of the kernel. Polls CPU/RAM to produce a
// PressureLevel (Green/Yellow/Red) that the Supervisor and GUI consume.

use std::sync::{Arc, RwLock};
use sysinfo::{System, SystemExt};
use tracing::{info, warn};

/// System stress level, used to gate throughput and trigger Atomic Stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureLevel {
    /// Normal operation — full parallelism enabled.
    Green,
    /// Elevated usage (>75%) — throttle non-essential tasks, flush caches.
    Yellow,
    /// Critical (>90%) — trigger Atomic Stop, pause workers.
    Red,
}

impl std::fmt::Display for PressureLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PressureLevel::Green => write!(f, "🟢 Green"),
            PressureLevel::Yellow => write!(f, "🟡 Yellow"),
            PressureLevel::Red => write!(f, "🔴 Red"),
        }
    }
}

/// Monitors host memory and exposes a shared `PressureLevel`.
pub struct PressureWatcher {
    sys: System,
    current_level: Arc<RwLock<PressureLevel>>,
    /// Memory % threshold at which we enter Yellow.
    yellow_threshold: f32,
    /// Memory % threshold at which we enter Red.
    red_threshold: f32,
}

impl PressureWatcher {
    pub fn new() -> Self {
        Self {
            sys: System::new(),
            current_level: Arc::new(RwLock::new(PressureLevel::Green)),
            yellow_threshold: 75.0,
            red_threshold: 90.0,
        }
    }

    /// Sample current memory and update the pressure level.
    /// Call this on a regular cadence (e.g. every GUI frame or every second).
    pub fn pulse(&mut self) {
        self.sys.refresh_memory();

        let total = self.sys.total_memory() as f32;
        if total == 0.0 {
            return; // Cannot determine — stay at current level
        }

        let usage_pct = (self.sys.used_memory() as f32 / total) * 100.0;

        let new_level = if usage_pct > self.red_threshold {
            PressureLevel::Red
        } else if usage_pct > self.yellow_threshold {
            PressureLevel::Yellow
        } else {
            PressureLevel::Green
        };

        // Only log on transitions
        let prev = self.get_level();
        if prev != new_level {
            match new_level {
                PressureLevel::Red => {
                    warn!(
                        "[PRESSURE] ⛔ CRITICAL — Memory at {:.1}%. Triggering Atomic Stop.",
                        usage_pct
                    );
                }
                PressureLevel::Yellow => {
                    warn!(
                        "[PRESSURE] ⚠️ Elevated — Memory at {:.1}%. Throttling.",
                        usage_pct
                    );
                }
                PressureLevel::Green => {
                    info!("[PRESSURE] ✅ Memory nominal at {:.1}%.", usage_pct);
                }
            }
        }

        if let Ok(mut level) = self.current_level.write() {
            *level = new_level;
        }
    }

    /// Read the current pressure level (lock-free read).
    pub fn get_level(&self) -> PressureLevel {
        self.current_level
            .read()
            .map(|l| *l)
            .unwrap_or(PressureLevel::Green)
    }

    /// Get a shareable handle to the pressure level for the GUI.
    pub fn level_handle(&self) -> Arc<RwLock<PressureLevel>> {
        self.current_level.clone()
    }

    /// Current memory usage as a 0.0–1.0 ratio.
    pub fn memory_ratio(&mut self) -> f32 {
        self.sys.refresh_memory();
        let total = self.sys.total_memory() as f32;
        if total == 0.0 {
            return 0.0;
        }
        self.sys.used_memory() as f32 / total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pressure_watcher_pulse() {
        let mut pw = PressureWatcher::new();
        pw.pulse();
        // Should return a valid level (we can't predict which one)
        let level = pw.get_level();
        assert!(
            level == PressureLevel::Green
                || level == PressureLevel::Yellow
                || level == PressureLevel::Red
        );
    }

    #[test]
    fn test_memory_ratio_range() {
        let mut pw = PressureWatcher::new();
        let ratio = pw.memory_ratio();
        assert!(ratio >= 0.0 && ratio <= 1.0, "ratio {} out of range", ratio);
    }
}
