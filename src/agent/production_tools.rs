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
    let p_str = p.to_string_lossy().replace("\\", "/");
    
    // Auto-detect and convert Windows paths in WSL (e.g., C:/... -> /mnt/c/...)
    if cfg!(unix) && p_str.len() >= 3 && p_str.chars().nth(1) == Some(':') && p_str.chars().nth(2) == Some('/') {
        let drive_letter = p_str.chars().next().unwrap().to_ascii_lowercase();
        let wsl_path = format!("/mnt/{}/{}", drive_letter, &p_str[3..]);
        return PathBuf::from(wsl_path);
    }
    
    let path_to_check = PathBuf::from(p_str);
    if path_to_check.is_absolute() {
        path_to_check
    } else {
        std::path::Path::new(".").join(path_to_check)
    }
}

/// Trim a video to a specific range
pub async fn trim_video(
    input: &Path,
    start_time: f64,
    duration: f64,
    output: &Path,
) -> Result<ProductionResult, Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "[PROD] Trimming video: {:?} ({:.2}s + {:.2}s)",
        input, start_time, duration
    );

    let safe_input = safe_arg_path(input);
    let safe_output = safe_arg_path(output);

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg(&start_time.to_string())
        .arg("-t")
        .arg(&duration.to_string())
        .arg("-i")
        .arg(&safe_input)
        .args([
            "-c:v",
            "libx264",
            "-preset",
            "faster",
            "-crf",
            "23",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("[PROD] Applying 2.39:1 Cinematic Mask");
    let safe_input = safe_arg_path(input);
    let safe_output = safe_arg_path(output);

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&safe_input)
        .args(["-vf", "crop=in_w:in_w/2.39", "-c:a", "copy"])
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
) -> Result<ProductionResult, Box<dyn std::error::Error + Send + Sync>> {
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
pub async fn enhance_audio(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        .args(["-y", "-nostdin", "-i"])
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

/// Combine a video file with an external audio file
/// Replaces the video's original audio with the new audio track.
pub async fn combine_av(
    video_path: &Path,
    audio_path: &Path,
    output_path: &Path,
) -> Result<ProductionResult, Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "[PROD] Combining Video: {:?} + Audio: {:?}",
        video_path, audio_path
    );

    let safe_video = safe_arg_path(video_path);
    let safe_audio = safe_arg_path(audio_path);
    let safe_output = safe_arg_path(output_path);

    // FFmpeg command to replace audio:
    // -map 0:v (Take video from input 0)
    // -map 1:a (Take audio from input 1)
    // -c:v copy (Copy video stream directly - fast!)
    // -c:a aac (Re-encode audio to AAC for compatibility)
    // -shortest (Finish when the shortest stream ends)

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&safe_video)
        .arg("-i")
        .arg(&safe_audio)
        .args([
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&safe_output)
        .status()
        .await?;

    if !status.success() {
        return Err("FFmpeg combine failed".into());
    }

    let metadata = tokio::fs::metadata(output_path).await?;
    let size_mb = metadata.len() as f64 / 1_048_576.0;

    info!("[PROD] Combine Complete. Final Size: {:.2} MB", size_mb);

    Ok(ProductionResult {
        output_path: output_path.to_path_buf(),
        size_mb,
    })
}

/// Build a complex filtergraph for transitions
pub fn build_transition_filter(
    inputs: usize,
    transition_duration: f64,
    video_durations: &[f64],
) -> String {
    let mut filter = String::new();
    let mut offset = 0.0;

    // We need at least 2 inputs to transition
    if inputs < 2 {
        return "".to_string();
    }

    for i in 0..inputs - 1 {
        let seg_duration = video_durations[i];
        offset += seg_duration - transition_duration;

        let prev_label = if i == 0 {
            "0:v".to_string()
        } else {
            format!("v{}", i)
        };
        let next_label = format!("{}:v", i + 1);
        let out_label = format!("v{}", i + 1);

        // Select random transition effect
        let transitions = [
            "fade",
            "wipeleft",
            "wiperight",
            "slideleft",
            "slideright",
            "circlecrop",
            "rectcrop",
        ];
        let effect = transitions[i % transitions.len()];

        filter.push_str(&format!(
            "[{}{}][{}]xfade=transition={}:duration={}:offset={}[{}];",
            if i == 0 { "" } else { "" }, // Empty prefix hack
            prev_label,
            next_label,
            effect,
            transition_duration,
            offset,
            out_label
        ));
    }

    // Audio crossfade (acrossfade)

    for i in 0..inputs - 1 {
        let prev_label = if i == 0 {
            "0:a".to_string()
        } else {
            format!("a{}", i)
        };
        let next_label = format!("{}:a", i + 1);
        let out_label = format!("a{}", i + 1);

        filter.push_str(&format!(
            "[{}][{}]acrossfade=d={}[{}];",
            prev_label, next_label, transition_duration, out_label
        ));
    }

    filter
}

/// Extract audio as 16kHz Mono PCM WAV (Ideal for Whisper)
pub async fn extract_audio_wav(
    input_video: &Path,
    output_wav: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    info!("[PRODUCTION] Extracting audio for Whisper: {:?}", input_video);

    let output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(safe_arg_path(input_video))
        .arg("-vn") // No video
        .arg("-acodec")
        .arg("pcm_s16le") // 16-bit PCM
        .arg("-ar")
        .arg("16000") // 16kHz
        .arg("-ac")
        .arg("1") // Mono
        .arg(safe_arg_path(output_wav))
        .output()
        .await?;

    if !output.status.success() {
        warn!("[PRODUCTION] FFmpeg audio extraction failed!");
        let err = String::from_utf8_lossy(&output.stderr);
        warn!("{}", err);
        return Err(format!("FFmpeg error: {}", err).into());
    }

    Ok(output_wav.to_path_buf())
}

/// Burn subtitles onto a video using FFmpeg
pub async fn burn_subtitles(
    input_video: &Path,
    input_srt: &Path,
    output_video: &Path,
) -> Result<ProductionResult, Box<dyn std::error::Error + Send + Sync>> {
    info!("[PRODUCTION] Burning subtitles from {:?} onto {:?}", input_srt, input_video);

    // FFmpeg subtitle filter is strict about paths. Drive letter colons must be escaped.
    let mut srt_safe = safe_arg_path(input_srt).to_string_lossy().into_owned();
    if cfg!(windows) {
        srt_safe = srt_safe.replace(":", "\\:");
    }

    // Force a clean modern font
    let filter = format!("subtitles='{}':force_style='FontName=Arial,FontSize=24,PrimaryColour=&H00FFFFFF,OutlineColour=&H00000000,BorderStyle=1,Outline=2'", srt_safe);

    let output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(safe_arg_path(input_video))
        .arg("-vf")
        .arg(&filter)
        .arg("-c:a")
        .arg("copy")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("fast")
        .arg(safe_arg_path(output_video))
        .output()
        .await?;

    if !output.status.success() {
        warn!("[PRODUCTION] FFmpeg burn_subtitles failed!");
        let err = String::from_utf8_lossy(&output.stderr);
        warn!("{}", err);
        return Err(format!("FFmpeg error: {}", err).into());
    }

    info!("[PRODUCTION] Subtitles burned successfully: {:?}", output_video);

    Ok(ProductionResult {
        output_path: output_video.to_path_buf(),
        duration: get_video_duration(output_video).await.unwrap_or(0.0),
    })
}
