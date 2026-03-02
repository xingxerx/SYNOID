// SYNOID Recovery Manifest — Crash-Proof State Persistence
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// The "Black Box" of the kernel. On Atomic Stop (or Ctrl-C), the
// current render state is serialized to a JSON manifest so SYNOID
// can resume from exactly where it left off.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};

/// Default directory for recovery data, relative to the project root.
const RECOVERY_DIR: &str = ".synoid/cortex_cache";
const MANIFEST_FILE: &str = "recovery_manifest.json";

/// Snapshot of the render state at the moment of interruption.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryManifest {
    /// Project or task name.
    pub project: String,
    /// Last successfully rendered frame index.
    pub last_frame: u64,
    /// The creative intent / command that was active.
    pub last_intent: String,
    /// Hardware state description (e.g. "OOM_THROTTLED", "NOMINAL").
    pub hardware_state: String,
    /// ISO-8601 timestamp of the snapshot.
    pub timestamp: String,
    /// Paths to verified chunk files that can be resumed from.
    pub completed_chunks: Vec<PathBuf>,
}

impl RecoveryManifest {
    /// Create a new manifest with the current UTC timestamp.
    pub fn new(project: &str, last_frame: u64, last_intent: &str, hardware_state: &str) -> Self {
        let timestamp = chrono_lite_now();
        Self {
            project: project.to_string(),
            last_frame,
            last_intent: last_intent.to_string(),
            hardware_state: hardware_state.to_string(),
            timestamp,
            completed_chunks: Vec::new(),
        }
    }

    /// Save the manifest to `.synoid/cortex_cache/recovery_manifest.json`
    /// relative to `project_root`.
    pub fn save(&self, project_root: &Path) -> Result<PathBuf, String> {
        let dir = project_root.join(RECOVERY_DIR);
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create recovery dir: {}", e))?;

        let path = dir.join(MANIFEST_FILE);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialization error: {}", e))?;

        fs::write(&path, json).map_err(|e| format!("Failed to write manifest: {}", e))?;

        info!("[RECOVERY] 💾 Manifest saved: {:?}", path);
        Ok(path)
    }

    /// Attempt to load a recovery manifest from the project root.
    /// Returns `None` if no manifest exists.
    pub fn load(project_root: &Path) -> Option<Self> {
        let path = project_root.join(RECOVERY_DIR).join(MANIFEST_FILE);
        if !path.exists() {
            return None;
        }

        match fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<RecoveryManifest>(&json) {
                Ok(manifest) => {
                    info!(
                        "[RECOVERY] 📂 Found manifest: project='{}', last_frame={}",
                        manifest.project, manifest.last_frame
                    );
                    Some(manifest)
                }
                Err(e) => {
                    error!("[RECOVERY] Failed to parse manifest: {}", e);
                    None
                }
            },
            Err(e) => {
                error!("[RECOVERY] Failed to read manifest: {}", e);
                None
            }
        }
    }

    /// Delete the recovery manifest (called after successful project completion).
    pub fn clear(project_root: &Path) -> Result<(), String> {
        let path = project_root.join(RECOVERY_DIR).join(MANIFEST_FILE);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Failed to remove manifest: {}", e))?;
            info!("[RECOVERY] 🗑️ Manifest cleared.");
        }
        Ok(())
    }
}

/// Lightweight ISO-8601 timestamp without pulling in the `chrono` crate.
fn chrono_lite_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    // Produce a Unix-seconds string (not ISO-8601, but functional & dependency-free)
    format!("unix:{}", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_manifest_round_trip() {
        let tmp_dir = PathBuf::from("__test_recovery_root");
        let _ = fs::remove_dir_all(&tmp_dir);

        let mut manifest = RecoveryManifest::new(
            "test_project",
            4502,
            "Apply teal_orange.cube LUT",
            "NOMINAL",
        );
        manifest.completed_chunks = vec![
            PathBuf::from("chunk_001.mp4"),
            PathBuf::from("chunk_002.mp4"),
        ];

        // Save
        let save_result = manifest.save(&tmp_dir);
        assert!(save_result.is_ok(), "Save failed: {:?}", save_result.err());

        // Load
        let loaded = RecoveryManifest::load(&tmp_dir);
        assert!(loaded.is_some(), "Manifest not found after save");
        let loaded = loaded.unwrap();
        assert_eq!(loaded.project, "test_project");
        assert_eq!(loaded.last_frame, 4502);
        assert_eq!(loaded.completed_chunks.len(), 2);

        // Clear
        RecoveryManifest::clear(&tmp_dir).unwrap();
        assert!(RecoveryManifest::load(&tmp_dir).is_none());

        // Cleanup
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_load_nonexistent() {
        let result = RecoveryManifest::load(Path::new("__nonexistent_project_zyx"));
        assert!(result.is_none());
    }
}
