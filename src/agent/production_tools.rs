// SYNOID Production Tools - Editing & Compression
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This module provides FFmpeg wrappers for trimming, clipping, and 
// intelligent compression to target file sizes.

use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};
use crate::agent::source_tools::get_video_duration;

/// Result of a production operation
#[derive(Debug)]
pub struct ProductionResult {
    pub output_path: PathBuf,
    pub size_mb: f64,
}

/// Trim a video to a specific range
pub async fn trim_video(
    input: &Path,
    start_time: f64,
    duration: f64,
    output: &Path,
) -> Result<ProductionResult, Box<dyn std::error::Error>> {
    info!("[PROD] Trimming video: {:?} ({:.2}s + {:.2}s)", input, start_time, duration);

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i", input.to_str().unwrap(),
            "-ss", &start_time.to_string(),
            "-t", &duration.to_string(),
            "-c", "copy", // Fast stream copy
            "-avoid_negative_ts", "make_zero",
            output.to_str().unwrap(),
        ])
        .status()
        .await?;

    if !status.success() {
        return Err("FFmpeg trim failed".into());
    }

    let metadata = std::fs::metadata(output)?;
    let size_mb = metadata.len() as f64 / 1_048_576.0;

    Ok(ProductionResult {
        output_path: output.to_path_buf(),
        size_mb,
    })
}

#[allow(dead_code)]
pub async fn apply_anamorphic_mask(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    info!("[PROD] Applying 2.39:1 Cinematic Mask");
    let status = Command::new("ffmpeg")
        .args([
            "-y", "-i", input.to_str().unwrap(),
            "-vf", "crop=in_w:in_w/2.39",
            "-c:a", "copy",
            output.to_str().unwrap(),
        ])
        .status()
        .await?;
    if !status.success() { return Err("Anamorphic mask failed".into()); }
    Ok(())
}

/// Compress video to target file size (in MB)
/// Uses 2-pass encoding for precision if size is critical
pub async fn compress_video(
    input: &Path,
    target_size_mb: f64,
    output: &Path,
) -> Result<ProductionResult, Box<dyn std::error::Error>> {
    info!("[PROD] Compressing video: {:?} -> {:.2} MB", input, target_size_mb);

    let duration = get_video_duration(input).await?;
    
    // Calculate target bitrate
    // Bitrate (bits/s) = (Target Size (MB) * 8192) / Duration (s)
    // We reserve ~128kbps for audio, so video bitrate is remainder
    let audio_bitrate_kbps = 128.0;
    let total_bitrate_kbps = (target_size_mb * 8192.0) / duration;
    let video_bitrate_kbps = total_bitrate_kbps - audio_bitrate_kbps;

    if video_bitrate_kbps < 100.0 {
        warn!("[PROD] Warning: Target size very small for duration. Quality will be low.");
    }

    info!("[PROD] Calculated Bitrates - Video: {:.0}k, Audio: {:.0}k", video_bitrate_kbps, audio_bitrate_kbps);

    // Single pass CRF (Consistant Rate Factor) capped by maxrate is usually better/faster for modern codecs
    // but 2-pass is standard for strict size. Let's do a smart single pass with bufsize for now for speed/simplicity
    // unless strict control is requested.
    
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i", input.to_str().unwrap(),
            "-c:v", "libx264",
            "-b:v", &format!("{:.0}k", video_bitrate_kbps),
            "-maxrate", &format!("{:.0}k", video_bitrate_kbps * 1.5),
            "-bufsize", &format!("{:.0}k", video_bitrate_kbps * 2.0),
            "-preset", "medium",
            "-c:a", "aac",
            "-b:a", &format!("{:.0}k", audio_bitrate_kbps),
            output.to_str().unwrap(),
        ])
        .status()
        .await?;

    if !status.success() {
        return Err("FFmpeg compression failed".into());
    }

    let metadata = std::fs::metadata(output)?;
    let size_mb = metadata.len() as f64 / 1_048_576.0;

    info!("[PROD] Compression Complete. Final Size: {:.2} MB", size_mb);

    Ok(ProductionResult {
        output_path: output.to_path_buf(),
        size_mb,
    })
}

/// Enhance audio using vocal processing chain (EQ -> Compression -> Normalization)
pub async fn enhance_audio(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    info!("[PROD] Enhancing audio: {:?}", input);

    // Filter Chain:
    // 1. highpass=f=80: Remove rumble
    // 2. lowpass=f=8000: Remove hiss
    // 3. acompressor: Even out dynamics
    // 4. loudnorm: target -16 LUFS (standard podcast/web loudness)
    let filter_complex = "highpass=f=80,lowpass=f=8000,acompressor=ratio=4:attack=200:threshold=-12dB,loudnorm=I=-16:TP=-1.5:LRA=11";

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-nostdin",
            "-i", input.to_str().unwrap(),
            "-vn", // Disable video (audio only)
            "-map", "0:a:0", // Take first audio track
            "-af", filter_complex,
            "-c:a", "pcm_s16le", // Use PCM for WAV (lossless intermediate)
            "-ar", "48000", // Force 48kHz (prevent 192kHz upsampling)
            output.to_str().unwrap(),
        ])
        .status()
        .await?;

    if !status.success() {
        return Err("Audio enhancement failed".into());
    }

    Ok(())
}
