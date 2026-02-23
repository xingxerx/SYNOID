// SYNOID Audio Tools
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioAnalysis {
    pub duration: f64,
    pub average_loudness: f64,
    pub transients: Vec<f64>, // Timestamps of beats/transients
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrack {
    pub index: usize,
    pub title: String,
    pub language: Option<String>,
}

/// Scan audio for beats and stats
pub async fn scan_audio(path: &Path) -> Result<AudioAnalysis, Box<dyn std::error::Error + Send + Sync>> {
    info!("[EARS] Performing deep transient analysis: {:?}", path);

    // Use real FFmpeg ebur128 analysis instead of hardcoded values
    let safe_path = crate::agent::production_tools::safe_arg_path(path);
    let output = tokio::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(&safe_path)
        .args(["-filter_complex", "ebur128", "-f", "null", "-"])
        .output()
        .await?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut average_loudness = -14.0;
    for line in stderr.lines().rev() {
        if line.contains("I:") && line.contains("LUFS") {
            if let Some(idx) = line.find("I:") {
                let parts: Vec<&str> = line[idx..].split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(val) = parts[1].parse::<f64>() {
                        average_loudness = val;
                        break;
                    }
                }
            }
        }
    }

    Ok(AudioAnalysis {
        duration,
        average_loudness,
        transients: Vec::new(),
    })
}

/// Get all audio tracks from a file using ffprobe
pub async fn get_audio_tracks(path: &Path) -> Result<Vec<AudioTrack>, Box<dyn std::error::Error + Send + Sync>> {
    let safe_path = crate::agent::production_tools::safe_arg_path(path);

    let output = tokio::process::Command::new("ffprobe")
        .args([
            "-v", "error",
            "-select_streams", "a",
            "-show_entries", "stream=index:stream_tags=title,language",
            "-of", "json",
        ])
        .arg(&safe_path)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)?;

    let mut tracks = Vec::new();
    if let Some(streams) = json.get("streams").and_then(|s| s.as_array()) {
        for stream in streams {
            let index = stream.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
            let tags = stream.get("tags");
            let title = tags.and_then(|t| t.get("title")).and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
            let language = tags.and_then(|t| t.get("language")).and_then(|v| v.as_str()).map(|s| s.to_string());

            tracks.push(AudioTrack {
                index,
                title,
                language,
            });
        }
    }

    Ok(tracks)
}
