// SYNOID Validation Gate — Null-Decode Integrity Checker
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Performs a "Null Decode" pass on a media file: FFmpeg reads and decodes
// every packet but writes nothing. Any bitstream corruption surfaces as
// text on stderr.

use std::path::Path;
use std::process::Command;
use tracing::{error, info};

pub struct ValidationGate;

impl ValidationGate {
    /// Deep-stream integrity check on a media file.
    ///
    /// Returns `true` if FFmpeg can fully decode the file with zero errors.
    pub fn verify_chunk(path: &Path) -> bool {
        let path_str = match path.to_str() {
            Some(s) => s,
            None => {
                error!("[VALIDATION] Invalid path (non-UTF-8): {:?}", path);
                return false;
            }
        };

        let output = Command::new("ffmpeg")
            .args([
                "-v", "error",   // Only report real errors
                "-i", path_str,
                "-f", "null",    // Don't produce output
                "-",             // Null sink
            ])
            .output();

        match output {
            Ok(res) => {
                let stderr = String::from_utf8_lossy(&res.stderr);

                if res.status.success() && stderr.trim().is_empty() {
                    info!(
                        "[VALIDATION] ✅ Chunk verified: {:?}",
                        path.file_name().unwrap_or_default()
                    );
                    true
                } else {
                    error!(
                        "[VALIDATION] ❌ Corruption in {:?}: {}",
                        path,
                        stderr.trim()
                    );
                    false
                }
            }
            Err(e) => {
                error!(
                    "[VALIDATION] Failed to spawn ffmpeg for verification: {}",
                    e
                );
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_verify_nonexistent_file() {
        let result =
            ValidationGate::verify_chunk(&PathBuf::from("__nonexistent_file_xyz.mp4"));
        assert!(!result, "Non-existent file should fail validation");
    }
}
