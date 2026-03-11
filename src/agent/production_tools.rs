// SYNOID Production Tools - Editing & Compression
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
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
    pub duration: f64,
}

// Helper to ensure path is treated as file not flag
pub fn safe_arg_path(p: &Path) -> PathBuf {
    let p_str = p.to_string_lossy().replace("\\", "/");

    // Auto-detect and convert Windows paths in WSL (e.g., C:/... -> /mnt/c/...)
    if cfg!(unix)
        && p_str.len() >= 3
        && p_str.chars().nth(1) == Some(':')
        && p_str.chars().nth(2) == Some('/')
    {
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
        duration: get_video_duration(output).await.unwrap_or(0.0),
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

    let gpu_ctx = crate::gpu_backend::get_gpu_context().await;
    let neuro = crate::agent::neuroplasticity::Neuroplasticity::new();

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-i").arg(&safe_input);

    cmd.arg("-c:v").arg(gpu_ctx.ffmpeg_encoder());
    for flag in gpu_ctx.neuroplastic_ffmpeg_flags(neuro.current_speed()) {
        cmd.arg(flag);
    }

    cmd.args([
        "-b:v",
        &format!("{:.0}k", video_bitrate_kbps),
        "-maxrate",
        &format!("{:.0}k", video_bitrate_kbps * 1.5),
        "-bufsize",
        &format!("{:.0}k", video_bitrate_kbps * 2.0),
        "-c:a",
        "aac",
        "-b:a",
        &format!("{:.0}k", audio_bitrate_kbps),
    ]);

    let status = cmd.arg(&safe_output).status().await?;

    if !status.success() {
        return Err("FFmpeg compression failed".into());
    }

    let metadata = tokio::fs::metadata(output).await?;
    let size_mb = metadata.len() as f64 / 1_048_576.0;

    info!("[PROD] Compression Complete. Final Size: {:.2} MB", size_mb);

    Ok(ProductionResult {
        output_path: output.to_path_buf(),
        size_mb,
        duration: get_video_duration(output).await.unwrap_or(0.0),
    })
}

/// Enhance audio using vocal processing chain (EQ -> Compression -> Normalization)
pub async fn enhance_audio(
    input: &Path,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        duration: get_video_duration(output_path).await.unwrap_or(0.0),
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
    info!(
        "[PRODUCTION] Extracting audio for Whisper: {:?}",
        input_video
    );

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

/// Burn subtitles onto a video using FFmpeg.
///
/// Uses ASS (Advanced SubStation Alpha) format internally: the SRT is converted
/// to a styled ASS tempfile so all font/color/size settings live in the file
/// itself.  This avoids the Windows FFmpeg issue where commas in `force_style`
/// values break the filter-string parser even when passed via tokio (no shell).
pub async fn burn_subtitles(
    input_video: &Path,
    input_srt: &Path,
    output_video: &Path,
) -> Result<ProductionResult, Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "[PRODUCTION] Burning subtitles from {:?} onto {:?}",
        input_srt, input_video
    );

    // 1. Read the SRT file.
    let srt_content =
        std::fs::read_to_string(input_srt).map_err(|e| format!("Failed to read SRT: {}", e))?;

    // 2. Convert SRT → ASS in memory with embedded style.
    //    This keeps all comma-separated style values *inside* the ASS file,
    //    so the FFmpeg filter string stays comma-free.
    let ass_content = srt_to_ass(&srt_content);

    // 3. Write ASS to system temp dir (short, controlled path).
    let temp_dir = std::env::temp_dir();
    let temp_ass = temp_dir.join("synoid_sub.ass");
    std::fs::write(&temp_ass, &ass_content)
        .map_err(|e| format!("Failed to write ASS to temp dir: {}", e))?;

    // 4. Build filter path — only one colon (the drive separator) to escape.
    //    FFmpeg's subtitles filter does TWO passes of backslash interpretation,
    //    so the colon needs to be escaped as `\\:` (two backslashes + colon)
    //    to survive both passes and reach libavfilter as a literal `:`.
    let temp_str = temp_ass.to_string_lossy().replace('\\', "/");
    let temp_escaped = if temp_str.len() >= 2 && temp_str.as_bytes()[1] == b':' {
        // "C:/path" → "C\\:/path"
        format!("{}\\\\:{}", &temp_str[..1], &temp_str[2..])
    } else {
        temp_str.to_string()
    };

    // No `force_style` needed — styling is embedded in the ASS file.
    let filter = format!("subtitles=filename={}", temp_escaped);

    let safe_input = safe_arg_path(input_video);
    let safe_output = safe_arg_path(output_video);

    info!("[PRODUCTION] burn_subtitles filter: {}", filter);

    let gpu_ctx = crate::gpu_backend::get_gpu_context().await;
    let neuro = crate::agent::neuroplasticity::Neuroplasticity::new();

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-nostdin")
        .arg("-i")
        .arg(&safe_input)
        .arg("-vf")
        .arg(&filter)
        .arg("-c:a")
        .arg("copy")
        .arg("-c:v")
        .arg(gpu_ctx.ffmpeg_encoder());

    for flag in gpu_ctx.neuroplastic_ffmpeg_flags(neuro.current_speed()) {
        cmd.arg(flag);
    }

    if gpu_ctx.has_gpu() {
        cmd.arg("-cq").arg("23"); // NVENC
    } else {
        cmd.arg("-crf").arg("23"); // CPU
    }

    cmd.arg(&safe_output);
    let result = cmd.output().await?;

    // Clean up temp file regardless of outcome
    let _ = std::fs::remove_file(&temp_ass);

    if !result.status.success() {
        let err = String::from_utf8_lossy(&result.stderr);
        warn!("[PRODUCTION] FFmpeg burn_subtitles FAILED:\n{}", err);
        return Err(format!("FFmpeg subtitle burn error: {}", err).into());
    }

    info!(
        "[PRODUCTION] ✅ Subtitles burned successfully: {:?}",
        output_video
    );

    Ok(ProductionResult {
        output_path: output_video.to_path_buf(),
        size_mb: tokio::fs::metadata(output_video)
            .await
            .map(|m| m.len() as f64 / 1_048_576.0)
            .unwrap_or(0.0),
        duration: get_video_duration(output_video).await.unwrap_or(0.0),
    })
}

/// Convert an SRT subtitle string into a styled ASS (Advanced SubStation Alpha) string.
/// All visual styles are embedded in the ASS header — no FFmpeg filter options needed.
fn srt_to_ass(srt: &str) -> String {
    // ASS header with custom style: white 28pt bold Arial, black outline, 30px from bottom
    let header = "\
[Script Info]\r\n\
ScriptType: v4.00+\r\n\
PlayResX: 1920\r\n\
PlayResY: 1080\r\n\
\r\n\
[V4+ Styles]\r\n\
Format: Name, Fontname, Fontsize, PrimaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\r\n\
Style: Default,Arial,28,&H00FFFFFF,&H00000000,&H00000000,-1,0,0,0,100,100,0,0,1,2,0,2,10,10,30,1\r\n\
\r\n\
[Events]\r\n\
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\r\n";

    let mut ass = header.to_string();

    // Parse SRT blocks: detect line-ending style and split once to avoid duplicates
    let separator = if srt.contains("\r\n\r\n") {
        "\r\n\r\n"
    } else {
        "\n\n"
    };
    for block in srt.trim().split(separator) {
        let lines: Vec<&str> = block.trim().lines().collect();
        if lines.len() < 3 {
            continue;
        }

        // Line 0: index (skip)
        // Line 1: "HH:MM:SS,mmm --> HH:MM:SS,mmm"
        let timing = lines[1];
        let parts: Vec<&str> = timing.splitn(2, " --> ").collect();
        if parts.len() != 2 {
            continue;
        }

        let start = srt_time_to_ass(parts[0].trim());
        let end = srt_time_to_ass(parts[1].trim());

        // Lines 2+: subtitle text (join with \N for ASS line breaks)
        let text = lines[2..].join("\\N");

        ass.push_str(&format!(
            "Dialogue: 0,{},{},Default,,0,0,0,,{}\r\n",
            start, end, text
        ));
    }

    ass
}

/// Convert SRT timestamp "HH:MM:SS,mmm" → ASS timestamp "H:MM:SS.cc"
fn srt_time_to_ass(t: &str) -> String {
    // SRT: 00:01:23,456  ASS: 0:01:23.45
    let t = t.replace(',', ".");
    let parts: Vec<&str> = t.splitn(2, '.').collect();
    let hms = parts[0];
    let ms_str = parts.get(1).copied().unwrap_or("0");
    // ASS uses centiseconds (2 digits)
    let cs: u32 = ms_str.parse::<u32>().unwrap_or(0) / 10;
    // Strip leading zero from hours: "00:01:23" → "0:01:23"
    let hms_parts: Vec<&str> = hms.splitn(3, ':').collect();
    if hms_parts.len() == 3 {
        let h: u32 = hms_parts[0].parse().unwrap_or(0);
        format!(
            "{}:{:02}:{:02}.{:02}",
            h,
            hms_parts[1].parse::<u32>().unwrap_or(0),
            hms_parts[2].parse::<u32>().unwrap_or(0),
            cs
        )
    } else {
        format!("{}.{:02}", hms, cs)
    }
}

/// Apply audio censorship to specified timestamps using FFmpeg volume attenuation or
/// 1 kHz broadcast-beep overlay.  When no `replacement_sfx` is provided, a short
/// 1 kHz sine tone (the industry-standard censor beep) is mixed over every muted
/// region in place of the original audio.  This replaces the old behaviour that
/// simply produced dead silence.
pub async fn apply_audio_censor(
    input_audio: &Path,
    output_audio: &Path,
    censor_timestamps: &[(f64, f64)],
    replacement_sfx: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "[PROD] Applying audio censorship to {} segments",
        censor_timestamps.len()
    );

    if censor_timestamps.is_empty() {
        // Nothing to censor — just copy input to output
        let safe_input = safe_arg_path(input_audio);
        let safe_output = safe_arg_path(output_audio);
        let st = Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(&safe_input)
            .arg("-c:a")
            .arg("pcm_s16le")
            .arg(&safe_output)
            .status()
            .await?;
        if !st.success() {
            return Err("Audio copy failed".into());
        }
        return Ok(());
    }

    let safe_input = safe_arg_path(input_audio);
    let safe_output = safe_arg_path(output_audio);

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-i").arg(&safe_input);

    let mut filter_complex = String::new();

    if let Some(sfx) = replacement_sfx {
        // ── Custom SFX overlay path ─────────────────────────────────
        let safe_sfx = safe_arg_path(Path::new(sfx));
        cmd.arg("-i").arg(&safe_sfx);

        // 1. Mute original at each window
        let volume_chain: String = censor_timestamps
            .iter()
            .map(|(s, e)| format!("volume=0:enable='between(t,{:.4},{:.4})'", s, e))
            .collect::<Vec<_>>()
            .join(",");
        filter_complex.push_str(&format!("[0:a]{}[muted];", volume_chain));

        // 2. Delay the SFX clip to each start position and tag it
        let mut sfx_tags: Vec<String> = Vec::new();
        for (i, (start, _end)) in censor_timestamps.iter().enumerate() {
            let delay_ms = (start * 1000.0) as i64;
            filter_complex.push_str(&format!(
                "[1:a]adelay={d}|{d}[sfx{i}];",
                d = delay_ms,
                i = i
            ));
            sfx_tags.push(format!("[sfx{}]", i));
        }

        // 3. amix muted + all SFX clips
        filter_complex.push_str("[muted]");
        for tag in &sfx_tags {
            filter_complex.push_str(tag);
        }
        filter_complex.push_str(&format!(
            "amix=inputs={}:duration=first:dropout_transition=0:normalize=0[out]",
            censor_timestamps.len() + 1
        ));

        cmd.arg("-filter_complex").arg(&filter_complex);
        cmd.arg("-map").arg("[out]");
    } else {
        // ── Real 1 kHz broadcast-beep path ───────────────────────────
        // Strategy:
        //   [0:a] muted in every censor window  → [muted]
        //   For every window: generate a sine=1000 Hz clip of the exact duration,
        //   apply adelay to shift it to the correct timeline position → [beepN]
        //   amix [muted][beep0][beep1]… → [out]

        // 1. Build volume=0 chain
        let volume_chain: String = censor_timestamps
            .iter()
            .map(|(s, e)| format!("volume=0:enable='between(t,{:.4},{:.4})'", s, e))
            .collect::<Vec<_>>()
            .join(",");
        filter_complex.push_str(&format!("[0:a]{}[muted];", volume_chain));

        // 2. Per-window: slice a 1 kHz sine, then delay to position
        let n = censor_timestamps.len();
        for (i, (start, end)) in censor_timestamps.iter().enumerate() {
            let dur = (end - start).max(0.05);
            let delay_ms = (start * 1000.0) as i64;
            // sine generates a continuous tone; we trim it to `dur` seconds,
            // lower the volume to 40 % so it blends cleanly, then delay it.
            filter_complex.push_str(&format!(
                "sine=frequency=1000:sample_rate=48000:duration={dur:.4},\
                 volume=0.70,\
                 adelay={delay}|{delay}[beep{i}];",
                dur = dur,
                delay = delay_ms,
                i = i
            ));
        }

        // 3. amix muted original + all beep clips
        filter_complex.push_str("[muted]");
        for i in 0..n {
            filter_complex.push_str(&format!("[beep{}]", i));
        }
        filter_complex.push_str(&format!(
            "amix=inputs={}:duration=first:dropout_transition=0:normalize=0[out]",
            n + 1
        ));

        cmd.arg("-filter_complex").arg(&filter_complex);
        cmd.arg("-map").arg("[out]");
    }

    cmd.arg("-c:a").arg("pcm_s16le");
    cmd.arg(&safe_output);

    let status = cmd.status().await?;

    if !status.success() {
        return Err("Audio censorship failed".into());
    }

    Ok(())
}
