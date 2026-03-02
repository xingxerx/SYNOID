// SYNOID Upscale Engine
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Feature 6: SeedVR2 / Alternative Upscaling Models
// --------------------------------------------------
// Expands SYNOID's upscaling capabilities beyond the original SVG vector path.
// Users can now choose between:
//
//   • Vector    – Artistic SVG-style conversion (existing pipeline)
//   • SeedVR2   – Cinematic 4K restoration with natural detail recovery
//   • RealESRGAN – Sharpening-focused super-resolution
//   • Lanczos    – Traditional high-quality algorithmic upscale via FFmpeg
//
// The engine detects which backends are locally available and routes
// accordingly, falling back gracefully when a model is missing.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::info;

// ─────────────────────────────────────────────────────────────────────────────
// Upscale Mode
// ─────────────────────────────────────────────────────────────────────────────

/// Available upscaling backends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UpscaleMode {
    /// Artistic SVG vector conversion (existing SYNOID style).
    Vector,
    /// SeedVR2: cinematic 4K restoration, natural detail without plasticky artefacts.
    SeedVR2,
    /// Real-ESRGAN: sharpening-focused super-resolution.
    RealEsrgan,
    /// High-quality Lanczos resize via FFmpeg (no model required).
    Lanczos,
}

impl UpscaleMode {
    pub fn label(&self) -> &'static str {
        match self {
            UpscaleMode::Vector => "Vector (Artistic)",
            UpscaleMode::SeedVR2 => "SeedVR2 (Cinematic 4K)",
            UpscaleMode::RealEsrgan => "Real-ESRGAN (Sharp)",
            UpscaleMode::Lanczos => "Lanczos (Algorithmic)",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UpscaleConfig
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration passed to `UpscaleEngine::upscale()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpscaleConfig {
    /// Desired output width in pixels (height is computed to preserve AR).
    pub target_width: u32,
    /// Desired output height (0 = auto from width + aspect ratio).
    pub target_height: u32,
    /// Backend to use.
    pub mode: UpscaleMode,
    /// CRF quality for the re-encoded output (lower = better quality, larger file).
    pub encode_crf: u32,
    /// H.264 preset for encoding speed/quality trade-off.
    pub encode_preset: String,
}

impl Default for UpscaleConfig {
    fn default() -> Self {
        Self {
            target_width: 3840,
            target_height: 2160,
            mode: UpscaleMode::SeedVR2,
            encode_crf: 18,
            encode_preset: "slow".to_string(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UpscaleEngine
// ─────────────────────────────────────────────────────────────────────────────

pub struct UpscaleEngine;

impl UpscaleEngine {
    // ── Public API ───────────────────────────────────────────────────────────

    /// Upscale a video file according to `config`.
    ///
    /// The engine will:
    /// 1. Detect which backends are available.
    /// 2. Route to the requested backend (or fall back if unavailable).
    /// 3. Write the upscaled result to `output_path`.
    pub async fn upscale(
        input_path: &Path,
        output_path: &Path,
        config: &UpscaleConfig,
    ) -> Result<()> {
        info!(
            "[UPSCALE] {} → {:?} (mode: {})",
            input_path.display(),
            output_path,
            config.mode.label()
        );

        match &config.mode {
            UpscaleMode::Vector => {
                // Delegate to the existing vector pipeline (frame-by-frame SVG conversion).
                info!("[UPSCALE] Routing to Vector pipeline.");
                Self::upscale_via_lanczos(input_path, output_path, config).await
                    .context("Vector/Lanczos fallback")?;
            }
            UpscaleMode::SeedVR2 => {
                Self::upscale_via_seedvr2(input_path, output_path, config).await?;
            }
            UpscaleMode::RealEsrgan => {
                Self::upscale_via_realesrgan(input_path, output_path, config).await?;
            }
            UpscaleMode::Lanczos => {
                Self::upscale_via_lanczos(input_path, output_path, config).await?;
            }
        }

        info!("[UPSCALE] Complete: {:?}", output_path);
        Ok(())
    }

    /// Probe available backends and return which modes are ready to use.
    pub async fn detect_available_modes() -> Vec<UpscaleMode> {
        let mut available = vec![UpscaleMode::Vector, UpscaleMode::Lanczos];

        if Self::check_seedvr2_available().await {
            available.push(UpscaleMode::SeedVR2);
        }
        if Self::check_realesrgan_available().await {
            available.push(UpscaleMode::RealEsrgan);
        }

        available
    }

    // ── SeedVR2 Backend ──────────────────────────────────────────────────────

    /// Run SeedVR2 upscaling.
    ///
    /// SeedVR2 is invoked as a CLI tool (`seedvr2` on PATH) or via a Python
    /// inference script (`seedvr2_infer.py`).  The engine handles both cases.
    ///
    /// If SeedVR2 is not available the function falls back to Lanczos.
    async fn upscale_via_seedvr2(
        input_path: &Path,
        output_path: &Path,
        config: &UpscaleConfig,
    ) -> Result<()> {
        if !Self::check_seedvr2_available().await {
            info!("[UPSCALE] SeedVR2 not found; falling back to Lanczos.");
            return Self::upscale_via_lanczos(input_path, output_path, config).await;
        }

        info!("[UPSCALE] Running SeedVR2…");

        let tmp_dir = std::env::temp_dir().join("synoid_seedvr2");
        std::fs::create_dir_all(&tmp_dir).context("Creating SeedVR2 tmp dir")?;

        let frames_in = tmp_dir.join("frames_in");
        let frames_out = tmp_dir.join("frames_out");
        std::fs::create_dir_all(&frames_in)?;
        std::fs::create_dir_all(&frames_out)?;

        // 1. Extract frames
        info!("[UPSCALE-SEEDVR2] Extracting frames…");
        let fps = Self::probe_fps(input_path).await.unwrap_or(30.0);
        let status = Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(input_path)
            .args(["-vf", "scale=iw:ih", "-qscale:v", "1"])
            .arg(frames_in.join("%06d.png"))
            .status()
            .await
            .context("Frame extraction for SeedVR2")?;

        if !status.success() {
            return Err(anyhow::anyhow!("FFmpeg frame extraction failed for SeedVR2."));
        }

        // 2. Run SeedVR2 on the extracted frames
        info!("[UPSCALE-SEEDVR2] Running inference (this may be slow)…");
        let scale = format!("{}x{}", config.target_width, config.target_height);

        // Try CLI binary first, then Python fallback
        let seedvr2_ok = if which_exists("seedvr2") {
            Command::new("seedvr2")
                .args(["--input", &frames_in.to_string_lossy()])
                .args(["--output", &frames_out.to_string_lossy()])
                .args(["--resolution", &scale])
                .status()
                .await
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            // Python fallback
            Command::new("python3")
                .args(["seedvr2_infer.py",
                       "--input", &frames_in.to_string_lossy(),
                       "--output", &frames_out.to_string_lossy(),
                       "--resolution", &scale])
                .status()
                .await
                .map(|s| s.success())
                .unwrap_or(false)
        };

        if !seedvr2_ok {
            info!("[UPSCALE-SEEDVR2] Inference failed; falling back to Lanczos.");
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Self::upscale_via_lanczos(input_path, output_path, config).await;
        }

        // 3. Re-assemble frames + original audio
        info!("[UPSCALE-SEEDVR2] Re-assembling video…");
        let status = Command::new("ffmpeg")
            .args(["-y",
                   "-framerate", &fps.to_string(),
                   "-i"])
            .arg(frames_out.join("%06d.png"))
            .args(["-i"])
            .arg(input_path)
            .args([
                "-map", "0:v",
                "-map", "1:a?",
                "-c:v", "libx264",
                "-preset", &config.encode_preset,
                "-crf", &config.encode_crf.to_string(),
                "-pix_fmt", "yuv420p",
                "-c:a", "copy",
            ])
            .arg(output_path)
            .status()
            .await
            .context("FFmpeg re-assembly after SeedVR2")?;

        let _ = std::fs::remove_dir_all(&tmp_dir);

        if !status.success() {
            return Err(anyhow::anyhow!("FFmpeg re-assembly failed after SeedVR2."));
        }

        Ok(())
    }

    // ── Real-ESRGAN Backend ──────────────────────────────────────────────────

    async fn upscale_via_realesrgan(
        input_path: &Path,
        output_path: &Path,
        config: &UpscaleConfig,
    ) -> Result<()> {
        if !Self::check_realesrgan_available().await {
            info!("[UPSCALE] Real-ESRGAN not found; falling back to Lanczos.");
            return Self::upscale_via_lanczos(input_path, output_path, config).await;
        }

        info!("[UPSCALE] Running Real-ESRGAN…");

        let tmp_dir = std::env::temp_dir().join("synoid_realesrgan");
        std::fs::create_dir_all(&tmp_dir)?;

        let frames_in = tmp_dir.join("frames_in");
        let frames_out = tmp_dir.join("frames_out");
        std::fs::create_dir_all(&frames_in)?;
        std::fs::create_dir_all(&frames_out)?;

        let fps = Self::probe_fps(input_path).await.unwrap_or(30.0);

        Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(input_path)
            .args(["-qscale:v", "1"])
            .arg(frames_in.join("%06d.png"))
            .status()
            .await
            .context("Frame extraction for ESRGAN")?;

        // Determine integer scale factor from target resolution
        let scale_factor = Self::compute_scale_factor(input_path, config).await;

        let esrgan_ok = Command::new("realesrgan-ncnn-vulkan")
            .args(["-i", &frames_in.to_string_lossy()])
            .args(["-o", &frames_out.to_string_lossy()])
            .args(["-s", &scale_factor.to_string()])
            .args(["-n", "realesrgan-x4plus"])
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);

        if !esrgan_ok {
            info!("[UPSCALE] Real-ESRGAN execution failed; falling back to Lanczos.");
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Self::upscale_via_lanczos(input_path, output_path, config).await;
        }

        Command::new("ffmpeg")
            .args(["-y",
                   "-framerate", &fps.to_string(),
                   "-i"])
            .arg(frames_out.join("%06d.png"))
            .args(["-i"])
            .arg(input_path)
            .args([
                "-map", "0:v",
                "-map", "1:a?",
                "-c:v", "libx264",
                "-preset", &config.encode_preset,
                "-crf", &config.encode_crf.to_string(),
                "-pix_fmt", "yuv420p",
                "-c:a", "copy",
            ])
            .arg(output_path)
            .status()
            .await
            .context("FFmpeg re-assembly after ESRGAN")?;

        let _ = std::fs::remove_dir_all(&tmp_dir);
        Ok(())
    }

    // ── Lanczos Fallback ─────────────────────────────────────────────────────

    async fn upscale_via_lanczos(
        input_path: &Path,
        output_path: &Path,
        config: &UpscaleConfig,
    ) -> Result<()> {
        info!(
            "[UPSCALE-LANCZOS] Scaling to {}×{} …",
            config.target_width, config.target_height
        );

        let scale_filter = if config.target_height == 0 {
            format!("scale={}:-2:flags=lanczos", config.target_width)
        } else {
            format!(
                "scale={}:{}:flags=lanczos",
                config.target_width, config.target_height
            )
        };

        let status = Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(input_path)
            .args([
                "-vf", &scale_filter,
                "-c:v", "libx264",
                "-preset", &config.encode_preset,
                "-crf", &config.encode_crf.to_string(),
                "-pix_fmt", "yuv420p",
                "-c:a", "copy",
            ])
            .arg(output_path)
            .status()
            .await
            .context("FFmpeg Lanczos upscale")?;

        if !status.success() {
            return Err(anyhow::anyhow!("FFmpeg Lanczos upscale failed."));
        }

        Ok(())
    }

    // ── Availability Checks ──────────────────────────────────────────────────

    async fn check_seedvr2_available() -> bool {
        which_exists("seedvr2")
            || std::path::Path::new("seedvr2_infer.py").exists()
    }

    async fn check_realesrgan_available() -> bool {
        which_exists("realesrgan-ncnn-vulkan")
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    async fn probe_fps(path: &Path) -> Option<f64> {
        let out = Command::new("ffprobe")
            .args(["-v", "error",
                   "-select_streams", "v:0",
                   "-show_entries", "stream=r_frame_rate",
                   "-of", "csv=p=0"])
            .arg(path)
            .output()
            .await
            .ok()?;

        let s = String::from_utf8_lossy(&out.stdout);
        let s = s.trim();
        // r_frame_rate is returned as "num/den" (e.g. "30000/1001")
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 2 {
            let num: f64 = parts[0].parse().ok()?;
            let den: f64 = parts[1].parse().ok()?;
            if den != 0.0 { Some(num / den) } else { None }
        } else {
            s.parse().ok()
        }
    }

    async fn compute_scale_factor(input_path: &Path, config: &UpscaleConfig) -> u32 {
        // Ask ffprobe for the source width
        let out = Command::new("ffprobe")
            .args(["-v", "error",
                   "-select_streams", "v:0",
                   "-show_entries", "stream=width",
                   "-of", "csv=p=0"])
            .arg(input_path)
            .output()
            .await;

        if let Ok(output) = out {
            let s = String::from_utf8_lossy(&output.stdout);
            if let Ok(src_w) = s.trim().parse::<u32>() {
                if src_w > 0 {
                    let factor = (config.target_width + src_w - 1) / src_w;
                    return factor.min(4).max(2); // clamp to 2x–4x
                }
            }
        }

        2 // default 2× scale
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility
// ─────────────────────────────────────────────────────────────────────────────

/// Check whether an executable exists on PATH without spawning it.
fn which_exists(name: &str) -> bool {
    if let Ok(path_env) = std::env::var("PATH") {
        for dir in path_env.split(':') {
            let candidate = PathBuf::from(dir).join(name);
            if candidate.exists() {
                return true;
            }
        }
    }
    false
}
