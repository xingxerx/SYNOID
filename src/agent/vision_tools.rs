// SYNOID Vision Tools
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualScene {
    pub timestamp: f64,
    pub motion_score: f64,
    pub scene_score: f64,
}

/// Scan video for visual scenes using FFmpeg/FFprobe
/// In a real implementation this might call Cuda kernels, but here we perform a simulated scan
/// or use ffprobe's scene detection filter.
pub async fn scan_visual(path: &Path) -> Result<Vec<VisualScene>, Box<dyn std::error::Error>> {
    info!("[EYES] Scanning visual content: {:?}", path);

    // Using ffprobe to detect scene changes (>0.3 difference)
    // We pass the file directly with -i to avoid complex escaping issues with the 'movie' filter on Windows
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "frame=pkt_pts_time,frame_tags=lavfi.scene_score",
            "-of",
            "csv=p=0", // Comma separated values for easier parsing
            "-vf",
            "select='gt(scene,0.3)'",
            "-i",
        ])
        .arg(path)
        .output()
        .await?; // Async execution

    if !output.status.success() {
        return Err(format!(
            "FFprobe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Offload parsing to blocking task
    let stdout_bytes = output.stdout;
    let scenes = tokio::task::spawn_blocking(move || {
        let stdout = String::from_utf8_lossy(&stdout_bytes);
        let mut scenes = Vec::new();

        // Always add start as a scene
        scenes.push(VisualScene {
            timestamp: 0.0,
            motion_score: 0.0,
            scene_score: 1.0,
        });

        for line in stdout.lines() {
            // Output format: timestamp,score
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 1 {
                if let Ok(ts) = parts[0].trim().parse::<f64>() {
                    // Parse score if available, otherwise default to 1.0
                    let score = if parts.len() >= 2 {
                        parts[1].trim().parse::<f64>().unwrap_or(1.0)
                    } else {
                        1.0
                    };

                    // Avoid duplicate 0.0 or very close timestamps
                    if !scenes.is_empty() && (ts - scenes.last().unwrap().timestamp).abs() < 0.5 {
                        continue;
                    }

                    scenes.push(VisualScene {
                        timestamp: ts,
                        motion_score: score, // Use scene score as proxy for motion intensity
                        scene_score: score,
                    });
                }
            }
        }
        scenes
    })
    .await?;

    // Fallback if no scenes detected (e.g. short video or no changes) - ensure at least start is there
    if scenes.is_empty() {
        scenes.push(VisualScene {
            timestamp: 0.0,
            motion_score: 0.0,
            scene_score: 1.0,
        });
    }

    info!("[EYES] Detected {} scenes.", scenes.len());
    Ok(scenes)
}

/// Connects to the CUDA stream for real-time subject tracking
/// Returns coordinates for Rule-of-Thirds framing (x_offset, y_offset, zoom_factor)
pub fn track_subject_cuda(_device_id: usize, frame_path: &Path) -> (f64, f64, f64) {
    // In a real implementation, this would:
    // 1. Load the frame into GPU memory
    // 2. Run a TensorRT or YOLO model to find the subject
    // 3. Calculate the centroid
    // 4. Return the pan/zoom needed to center the subject on the Rule of Thirds grid

    info!("[VISION-CUDA] Tracking subject in frame: {:?}", frame_path);

    // Simulated "Cinematic" panning
    // Returns a slight pan and 1.0 zoom (no zoom) for now
    (0.0, 0.0, 1.0)
}

/// Calculates a simple pixel-wise difference between two frames.
/// Returns a normalized difference score (0.0 - 1.0).
/// Used for Temporal Coherence checks in Vector Engine.
pub fn calculate_optical_flow_diff(frame1: &Path, frame2: &Path) -> f64 {
    use image::GenericImageView;

    // We swallow errors and return 1.0 (max diff) to force re-render/vectorization if something fails
    let img1 = match image::open(frame1) {
        Ok(i) => i,
        Err(_) => return 1.0,
    };
    let img2 = match image::open(frame2) {
        Ok(i) => i,
        Err(_) => return 1.0,
    };

    if img1.dimensions() != img2.dimensions() {
        return 1.0;
    }

    let (w, h) = img1.dimensions();
    let num_pixels = (w * h) as f64;

    // Convert to RGB8 buffers for fast pixel access
    let buf1 = img1.to_rgb8();
    let buf2 = img2.to_rgb8();

    let mut total_diff = 0.0;

    // Check every pixel
    for (p1, p2) in buf1.pixels().zip(buf2.pixels()) {
        let r_diff = (p1[0] as i32 - p2[0] as i32).abs();
        let g_diff = (p1[1] as i32 - p2[1] as i32).abs();
        let b_diff = (p1[2] as i32 - p2[2] as i32).abs();

        total_diff += (r_diff + g_diff + b_diff) as f64 / 3.0;
    }

    // Normalize: max diff per pixel is 255.
    if num_pixels > 0.0 {
        let avg_diff = total_diff / num_pixels;
        avg_diff / 255.0
    } else {
        0.0
    }
}
