// SYNOID I/O Shield — Shadow Write & Atomic Move
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Implements the "Shadow Write" pattern:
//   1. All renders write to a `.tmp` sidecar file.
//   2. On success, `AtomicMover::commit()` renames it to the final path.
//   3. If the process crashes mid-write, only the `.tmp` is damaged —
//      the previous good version (if any) remains intact.

use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// AtomicMover
// ---------------------------------------------------------------------------

pub struct AtomicMover;

impl AtomicMover {
    /// Safely move a completed temp file to its final destination.
    ///
    /// * Same drive → `fs::rename` (atomic, zero-copy).
    /// * Cross-drive → `fs::copy` + `fs::remove_file` (fallback).
    pub fn commit(temp_path: &Path, final_path: &Path) -> Result<(), String> {
        if !temp_path.exists() {
            return Err(format!("Source temp file missing: {:?}", temp_path));
        }

        // Try the fast, atomic rename first.
        match fs::rename(temp_path, final_path) {
            Ok(()) => {
                info!(
                    "[IO_SHIELD] ✅ Atomic rename: {:?} → {:?}",
                    temp_path, final_path
                );
                Ok(())
            }
            Err(_rename_err) => {
                // Likely a cross-drive scenario — fall back to copy-then-delete.
                warn!(
                    "[IO_SHIELD] Rename failed (cross-drive?). Falling back to copy-delete."
                );
                fs::copy(temp_path, final_path).map_err(|e| {
                    format!("Cross-drive copy failed: {}", e)
                })?;
                fs::remove_file(temp_path).map_err(|e| {
                    format!("Temp cleanup after copy failed: {}", e)
                })?;
                info!(
                    "[IO_SHIELD] ✅ Cross-drive move complete: {:?} → {:?}",
                    temp_path, final_path
                );
                Ok(())
            }
        }
    }

    /// Generate the `.tmp` sidecar path for a given final output path.
    ///
    /// Example: `output.mp4` → `output.mp4.synoid_tmp`
    pub fn tmp_path_for(final_path: &Path) -> PathBuf {
        let mut tmp = final_path.as_os_str().to_owned();
        tmp.push(".synoid_tmp");
        PathBuf::from(tmp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_atomic_mover_same_drive() {
        let tmp = PathBuf::from("__test_io_shield_src.tmp");
        let dest = PathBuf::from("__test_io_shield_dst.dat");

        // Cleanup leftovers
        let _ = fs::remove_file(&tmp);
        let _ = fs::remove_file(&dest);

        fs::write(&tmp, b"hello synoid").unwrap();
        assert!(tmp.exists());

        AtomicMover::commit(&tmp, &dest).unwrap();

        assert!(!tmp.exists(), "Source should be gone after rename");
        assert!(dest.exists(), "Destination should exist");
        assert_eq!(fs::read_to_string(&dest).unwrap(), "hello synoid");

        let _ = fs::remove_file(&dest);
    }

    #[test]
    fn test_tmp_path_generation() {
        let p = PathBuf::from("render/output.mp4");
        let tmp = AtomicMover::tmp_path_for(&p);
        assert_eq!(tmp, PathBuf::from("render/output.mp4.synoid_tmp"));
    }

    #[test]
    fn test_commit_missing_source() {
        let result = AtomicMover::commit(
            Path::new("__nonexistent_file_xyz.tmp"),
            Path::new("__dest.dat"),
        );
        assert!(result.is_err());
    }
}
