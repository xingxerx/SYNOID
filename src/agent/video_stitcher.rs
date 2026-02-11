// SYNOID Video Stitcher — Lossless Chunk Concatenation
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// After chunked rendering, the Stitcher joins verified segments using
// FFmpeg's concat demuxer (`-f concat`).  Because we use `-c copy`,
// the resulting file has zero quality loss and near-zero CPU cost.

use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{error, info};

pub struct VideoStitcher;

impl VideoStitcher {
    /// Build the contents of an FFmpeg concat manifest from a list of
    /// verified chunk paths.
    ///
    /// Each line is `file '<absolute_path>'`.
    pub fn create_concat_manifest(segments: &[PathBuf]) -> String {
        segments
            .iter()
            .map(|p| format!("file '{}'", p.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Write the manifest to disk and invoke FFmpeg to join the chunks.
    ///
    /// The final output is a lossless copy-mux of all segments.
    pub async fn finalize(
        segments: &[PathBuf],
        output_path: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        if segments.is_empty() {
            return Err("No segments to stitch.".into());
        }

        // Write manifest next to the output file
        let manifest_path = output_path.with_extension("concat_manifest.txt");
        let manifest_content = Self::create_concat_manifest(segments);
        fs::write(&manifest_path, &manifest_content)?;

        info!(
            "[STITCHER] Manifest written ({} segments): {:?}",
            segments.len(),
            manifest_path
        );

        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-f", "concat",
                "-safe", "0",
                "-i",
            ])
            .arg(&manifest_path)
            .args(["-c", "copy"])
            .arg(output_path)
            .status()
            .await?;

        // Cleanup manifest
        let _ = fs::remove_file(&manifest_path);

        if status.success() {
            info!("[STITCHER] ✅ Final output: {:?}", output_path);
            Ok(output_path.to_path_buf())
        } else {
            error!("[STITCHER] ❌ FFmpeg concat failed.");
            Err("FFmpeg concat demuxer failed.".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concat_manifest_generation() {
        let segments = vec![
            PathBuf::from("/tmp/chunk_001.mp4"),
            PathBuf::from("/tmp/chunk_002.mp4"),
            PathBuf::from("/tmp/chunk_003.mp4"),
        ];
        let manifest = VideoStitcher::create_concat_manifest(&segments);
        let lines: Vec<&str> = manifest.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("chunk_001.mp4"));
        assert!(lines[2].contains("chunk_003.mp4"));
    }

    #[test]
    fn test_empty_segments() {
        let manifest = VideoStitcher::create_concat_manifest(&[]);
        assert!(manifest.is_empty());
    }
}
