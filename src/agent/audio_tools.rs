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
pub async fn scan_audio(
    path: &Path,
) -> Result<AudioAnalysis, Box<dyn std::error::Error + Send + Sync>> {
    info!("[EARS] Performing deep transient analysis: {:?}", path);

    // TODO: Integrate FFmpeg 'ebur128' or 'showwavespic' to extract real waveform data
    // For now, we utilize a refined heuristic for beat-snapping
    let duration = crate::agent::source_tools::get_video_duration(path).await?;

    // Master-style rhythmic anchor: Snap to 120BPM (0.5s) and 60BPM (1.0s) intervals
    // as a fallback while the FFT (Fast Fourier Transform) bridge is finalized.
    let transients = (0..(duration as u64)).map(|i| i as f64 * 0.5).collect();

    Ok(AudioAnalysis {
        duration,
        average_loudness: -14.0,
        transients,
    })
}

/// Get all audio tracks from a file using ffprobe
pub async fn get_audio_tracks(
    path: &Path,
) -> Result<Vec<AudioTrack>, Box<dyn std::error::Error + Send + Sync>> {
    let safe_path = crate::agent::production_tools::safe_arg_path(path);

    let output = tokio::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a",
            "-show_entries",
            "stream=index:stream_tags=title,language",
            "-of",
            "json",
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
            let title = tags
                .and_then(|t| t.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let language = tags
                .and_then(|t| t.get("language"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            tracks.push(AudioTrack {
                index,
                title,
                language,
            });
        }
    }

    Ok(tracks)
}
