// SYNOID Latent-Space Optimizer
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Uses latent-space processing for efficient video editing
// Reduces computational load by working in compressed latent representations

use crate::agent::engines::process_utils::CommandExt;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

/// Latent representation configuration
#[derive(Debug, Clone)]
pub struct LatentConfig {
    /// Compression ratio (higher = more compressed, lower quality)
    pub compression_ratio: f32,

    /// Enable temporal compression (group similar frames)
    pub temporal_compression: bool,

    /// Frame sampling rate (1 = all frames, 2 = every other frame, etc.)
    pub frame_sampling: usize,

    /// Use GPU for encoding/decoding
    pub gpu_accelerated: bool,
}

impl Default for LatentConfig {
    fn default() -> Self {
        Self {
            compression_ratio: 0.5,
            temporal_compression: true,
            frame_sampling: 1,
            gpu_accelerated: true,
        }
    }
}

pub struct LatentOptimizer {
    config: LatentConfig,
}

impl LatentOptimizer {
    pub fn new(config: LatentConfig) -> Self {
        Self { config }
    }

    /// Encode video to latent representation (compressed intermediate format)
    pub async fn encode_to_latent(
        &self,
        input_video: &Path,
        latent_output: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!("[LATENT] Encoding video to latent space: {:?}", input_video);

        let work_dir = input_video
            .parent()
            .unwrap_or(Path::new("."))
            .join(".synoid_latent");
        std::fs::create_dir_all(&work_dir)?;

        // Extract frames at specified sampling rate
        let frames_dir = work_dir.join("frames");
        std::fs::create_dir_all(&frames_dir)?;

        let fps_filter = if self.config.frame_sampling > 1 {
            format!("fps=fps=1/{}", self.config.frame_sampling)
        } else {
            "null".to_string()
        };

        // Extract frames as compressed JPEGs (latent representation)
        let quality = (100.0 * (1.0 - self.config.compression_ratio)) as i32;
        let quality = quality.max(10).min(95);

        info!("[LATENT] Extracting frames with quality: {}", quality);

        let mut cmd = Command::new("ffmpeg");
        cmd.stealth()
            .args(["-i"])
            .arg(input_video)
            .args(["-vf", &fps_filter])
            .args(["-q:v", &quality.to_string()])
            .arg(frames_dir.join("frame_%06d.jpg"))
            .args(["-y"]);

        if self.config.gpu_accelerated {
            // Use hardware acceleration if available
            cmd.args(["-hwaccel", "auto"]);
        }

        let status = cmd.status().await?;

        if !status.success() {
            return Err("Failed to encode to latent space".into());
        }

        // Create latent metadata
        let metadata = LatentMetadata {
            original_video: input_video.to_path_buf(),
            frames_dir: frames_dir.clone(),
            compression_ratio: self.config.compression_ratio,
            frame_sampling: self.config.frame_sampling,
            frame_count: self.count_frames(&frames_dir).await?,
        };

        // Save metadata
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(latent_output, metadata_json)?;

        info!(
            "[LATENT] Latent encoding complete: {} frames",
            metadata.frame_count
        );
        Ok(latent_output.to_path_buf())
    }

    /// Decode latent representation back to video
    pub async fn decode_from_latent(
        &self,
        latent_input: &Path,
        output_video: &Path,
        fps: Option<f64>,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "[LATENT] Decoding latent space to video: {:?}",
            output_video
        );

        // Load metadata
        let metadata_json = std::fs::read_to_string(latent_input)?;
        let metadata: LatentMetadata = serde_json::from_str(&metadata_json)?;

        let target_fps = fps.unwrap_or(30.0);

        // Reconstruct video from frames
        let mut cmd = Command::new("ffmpeg");
        cmd.stealth()
            .args(["-framerate", &target_fps.to_string()])
            .args(["-pattern_type", "glob"])
            .args(["-i"])
            .arg(
                metadata
                    .frames_dir
                    .join("frame_*.jpg")
                    .to_string_lossy()
                    .to_string(),
            )
            .args(["-c:v", "libx264"])
            .args(["-preset", "medium"])
            .args(["-crf", "18"])
            .args(["-pix_fmt", "yuv420p"])
            .arg(output_video)
            .args(["-y"]);

        if self.config.gpu_accelerated {
            cmd.args(["-hwaccel", "auto"]);
        }

        let status = cmd.status().await?;

        if !status.success() {
            return Err("Failed to decode from latent space".into());
        }

        info!("[LATENT] Decoding complete: {:?}", output_video);
        Ok(output_video.to_path_buf())
    }

    /// Apply processing to latent representation (frame-by-frame operations)
    pub async fn process_latent<F>(
        &self,
        latent_path: &Path,
        operation: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(&Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
    {
        info!("[LATENT] Processing latent frames");

        // Load metadata
        let metadata_json = std::fs::read_to_string(latent_path)?;
        let metadata: LatentMetadata = serde_json::from_str(&metadata_json)?;

        // Process each frame
        let frames = self.get_frame_list(&metadata.frames_dir).await?;

        for frame_path in frames {
            operation(&frame_path)?;
        }

        Ok(())
    }

    /// Optimize temporal consistency across latent frames
    pub async fn optimize_temporal_consistency(
        &self,
        latent_path: &Path,
        consistency_strength: f32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "[LATENT] Optimizing temporal consistency: {:.2}",
            consistency_strength
        );

        if !self.config.temporal_compression {
            warn!("[LATENT] Temporal compression disabled, skipping optimization");
            return Ok(());
        }

        // Load metadata
        let metadata_json = std::fs::read_to_string(latent_path)?;
        let metadata: LatentMetadata = serde_json::from_str(&metadata_json)?;

        let frames = self.get_frame_list(&metadata.frames_dir).await?;

        if frames.len() < 2 {
            return Ok(());
        }

        // Apply temporal smoothing using FFmpeg's minterpolate or blend
        let temp_dir = metadata.frames_dir.parent().unwrap().join("temp_smooth");
        std::fs::create_dir_all(&temp_dir)?;

        for i in 0..frames.len() {
            let current_frame = &frames[i];

            if i == 0 || i == frames.len() - 1 {
                // Copy first and last frames as-is
                std::fs::copy(current_frame, temp_dir.join(format!("frame_{:06}.jpg", i)))?;
                continue;
            }

            let prev_frame = &frames[i - 1];
            let next_frame = &frames[i + 1];

            // Blend current frame with neighbors for temporal smoothing
            let blend_weight = consistency_strength * 0.3; // 30% max influence from neighbors
            let output_frame = temp_dir.join(format!("frame_{:06}.jpg", i));

            let blend_filter = format!(
                "[0:v][1:v]blend=all_mode=average:all_opacity={}[mid];[mid][2:v]blend=all_mode=average:all_opacity={}",
                blend_weight, blend_weight
            );

            let status = Command::new("ffmpeg")
                .stealth()
                .args(["-i"])
                .arg(prev_frame)
                .args(["-i"])
                .arg(current_frame)
                .args(["-i"])
                .arg(next_frame)
                .args(["-filter_complex", &blend_filter])
                .args(["-frames:v", "1"])
                .arg(&output_frame)
                .args(["-y"])
                .status()
                .await?;

            if !status.success() {
                warn!("[LATENT] Failed to smooth frame {}, using original", i);
                std::fs::copy(current_frame, output_frame)?;
            }
        }

        // Replace original frames with smoothed versions
        for entry in std::fs::read_dir(&temp_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            let dest = metadata.frames_dir.join(&filename);
            std::fs::copy(entry.path(), dest)?;
        }

        // Cleanup
        std::fs::remove_dir_all(temp_dir)?;

        info!("[LATENT] Temporal consistency optimization complete");
        Ok(())
    }

    /// Count frames in a directory
    async fn count_frames(
        &self,
        frames_dir: &Path,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let count = std::fs::read_dir(frames_dir)?
            .filter(|entry| {
                entry
                    .as_ref()
                    .map(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "jpg")
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            })
            .count();
        Ok(count)
    }

    /// Get sorted list of frame paths
    async fn get_frame_list(
        &self,
        frames_dir: &Path,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
        let mut frames: Vec<PathBuf> = std::fs::read_dir(frames_dir)?
            .filter_map(|entry| entry.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|ext| ext == "jpg").unwrap_or(false))
            .collect();

        frames.sort();
        Ok(frames)
    }

    /// Cleanup latent workspace
    pub async fn cleanup_latent(
        &self,
        latent_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("[LATENT] Cleaning up latent workspace");

        // Load metadata
        let metadata_json = std::fs::read_to_string(latent_path)?;
        let metadata: LatentMetadata = serde_json::from_str(&metadata_json)?;

        // Remove frames directory
        if metadata.frames_dir.exists() {
            std::fs::remove_dir_all(&metadata.frames_dir)?;
        }

        // Remove metadata file
        std::fs::remove_file(latent_path)?;

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LatentMetadata {
    original_video: PathBuf,
    frames_dir: PathBuf,
    compression_ratio: f32,
    frame_sampling: usize,
    frame_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latent_config_default() {
        let config = LatentConfig::default();
        assert_eq!(config.compression_ratio, 0.5);
        assert!(config.temporal_compression);
        assert_eq!(config.frame_sampling, 1);
    }

    #[test]
    fn test_quality_calculation() {
        let config = LatentConfig {
            compression_ratio: 0.5,
            ..Default::default()
        };
        let quality = (100.0 * (1.0 - config.compression_ratio)) as i32;
        assert_eq!(quality, 50);
    }
}
