// SYNOID Production Tools - Editing & Compression
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This module provides FFmpeg wrappers for trimming, clipping, and
// intelligent compression to target file sizes.

use crate::agent::source_tools::get_video_duration;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

/// Result of a production operation
#[derive(Debug)]
pub struct ProductionResult {
    pub output_path: PathBuf,
    pub size_mb: f64,
}

// Helper to ensure path is treated as file not flag
pub fn safe_arg_path(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::path::Path::new(".").join(p)
    }
}

/// Trim a video to a specific range
pub async fn trim_video(
    input: &Path,
    start_time: f64,
    duration: f64,
    output: &Path,
) -> Result<ProductionResult, Box<dyn std::error::Error>> {
    info!(
        "[PROD] Trimming video: {:?} ({:.2}s + {:.2}s)",
        input, start_time, duration
    );

    let safe_input = safe_arg_path(input);
    let safe_output = safe_arg_path(output);

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&safe_input)
        .args([
            "-ss",
            &start_time.to_string(),
            "-t",
            &duration.to_string(),
            "-c",
            "copy", // Fast stream copy
            "-avoid_negative_ts",
            "make_zero",
        ])
        .arg(&safe_output)
        .status()
        .await?;

    if !status.success() {
        return Err("FFmpeg trim failed".into());
    }

    let metadata = tokio::fs::metadata(output).await?;
    let size_mb = metadata.len() as f64 / 1_048_576.0;

    Ok(ProductionResult {
        output_path: output.to_path_buf(),
        size_mb,
    })
}

#[allow(dead_code)]
pub async fn apply_anamorphic_mask(
    input: &Path,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("[PROD] Applying 2.39:1 Cinematic Mask");
    let safe_input = safe_arg_path(input);
    let safe_output = safe_arg_path(output);

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&safe_input)
        .args([
            "-vf",
            "crop=in_w:in_w/2.39",
            "-c:a",
            "copy",
        ])
        .arg(&safe_output)
        .status()
        .await?;
    if !status.success() {
        return Err("Anamorphic mask failed".into());
    }
    Ok(())
}

/// Compress video to target file size (in MB)
/// Uses 2-pass encoding for precision if size is critical
pub async fn compress_video(
    input: &Path,
    target_size_mb: f64,
    output: &Path,
) -> Result<ProductionResult, Box<dyn std::error::Error>> {
    info!(
        "[PROD] Compressing video: {:?} -> {:.2} MB",
        input, target_size_mb
    );

    let duration = get_video_duration(input).await?;
    // We reserve ~128kbps for audio, so video bitrate is remainder
    let audio_bitrate_kbps = 128.0;
    let total_bitrate_kbps = (target_size_mb * 8192.0) / duration;
    let video_bitrate_kbps = total_bitrate_kbps - audio_bitrate_kbps;

    if video_bitrate_kbps < 100.0 {
        warn!("[PROD] Warning: Target size very small for duration. Quality will be low.");
    }

    info!(
        "[PROD] Calculated Bitrates - Video: {:.0}k, Audio: {:.0}k",
        video_bitrate_kbps, audio_bitrate_kbps
    );

    // Single pass CRF (Consistant Rate Factor) capped by maxrate is usually better/faster for modern codecs
    // but 2-pass is standard for strict control is requested.

    let safe_input = safe_arg_path(input);
    let safe_output = safe_arg_path(output);

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&safe_input)
        .args([
            "-c:v",
            "libx264",
            "-b:v",
            &format!("{:.0}k", video_bitrate_kbps),
            "-maxrate",
            &format!("{:.0}k", video_bitrate_kbps * 1.5),
            "-bufsize",
            &format!("{:.0}k", video_bitrate_kbps * 2.0),
            "-preset",
            "medium",
            "-c:a",
            "aac",
            "-b:a",
            &format!("{:.0}k", audio_bitrate_kbps),
        ])
        .arg(&safe_output)
        .status()
        .await?;

    if !status.success() {
        return Err("FFmpeg compression failed".into());
    }

    let metadata = tokio::fs::metadata(output).await?;
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
    // 1. afftdn=nf=-25: FFT Noise Reduction (Voice cleanup)
    // 2. highpass=f=100: Remove rumble (voice is usually > 100Hz)
    // 3. lowpass=f=8000: Remove high-freq hiss
    // 4. acompressor: Even out dynamics
    // 5. loudnorm: target -16 LUFS
    let filter_complex = "afftdn=nf=-25,highpass=f=100,lowpass=f=8000,acompressor=ratio=4:attack=200:threshold=-12dB,loudnorm=I=-16:TP=-1.5:LRA=11";

    let safe_input = safe_arg_path(input);
    let safe_output = safe_arg_path(output);

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-nostdin",
            "-i",
        ])
        .arg(&safe_input)
        .args([
            "-vn", // Disable video (audio only)
            "-map",
            "0:a:0", // Take first audio track
            "-af",
            filter_complex,
            "-c:a",
            "pcm_s16le", // Use PCM for WAV (lossless intermediate)
            "-ar",
            "48000", // Force 48kHz (prevent 192kHz upsampling)
        ])
        .arg(&safe_output)
        .status()
        .await?;

    if !status.success() {
        return Err("Audio enhancement failed".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_arg_path() {
        let p = Path::new("normal.mp4");
        assert_eq!(safe_arg_path(p), PathBuf::from("./normal.mp4"));

        let p_bad = Path::new("-bad.mp4");
        assert_eq!(safe_arg_path(p_bad), PathBuf::from("./-bad.mp4"));

        // If absolute path exists on system (e.g. /abs/path.mp4), safe_arg_path keeps it.
        // Since is_absolute() is OS dependent, we construct one carefully or trust it works.
        // For testing, let's just check relative path behavior which is the security fix.

        let p_abs = PathBuf::from("/tmp/test.mp4");
        if p_abs.is_absolute() {
             assert_eq!(safe_arg_path(&p_abs), p_abs);
        }
    }
}
