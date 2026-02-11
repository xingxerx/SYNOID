#![allow(dead_code, unused_variables)]
// SYNOID Vector Engine
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::vision_tools::calculate_optical_flow_diff;
use rayon::prelude::*;
use resvg::tiny_skia;
use resvg::usvg;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{error, info};

/// Upscale video by converting to Vector and re-rendering at higher resolution
pub async fn upscale_video(
    input: &Path,
    scale_factor: f64,
    output: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    info!(
        "[UPSCALE] Starting Infinite Zoom (Scale: {}x) on {:?}",
        scale_factor, input
    );

    // 1. Setup Directories
    let work_dir = input.parent().unwrap().join("synoid_upscale_work");
    if work_dir.exists() {
        fs::remove_dir_all(&work_dir)?;
    }
    fs::create_dir_all(&work_dir)?;

    let frames_src = work_dir.join("src_frames");
    let frames_svg = work_dir.join("vectors");
    let frames_out = work_dir.join("high_res_frames");

    fs::create_dir_all(&frames_src)?;
    fs::create_dir_all(&frames_svg)?;
    fs::create_dir_all(&frames_out)?;

    // 2. Extract Source Frames
    info!("[UPSCALE] Extracting source frames...");
    let status = Command::new("ffmpeg")
        .args([
            "-i",
            input.to_str().unwrap(),
            "-vf",
            frames_src.join("frame_%04d.png").to_str().unwrap(),
        ])
        .output()
        .await?;

    if !status.status.success() {
        return Err("FFmpeg extraction failed".into());
    }

    // 3. Resolution Safety Check
    // Calculate theoretical output size based on first frame
    if let Some(first_frame) = fs::read_dir(&frames_src)?.filter_map(|e| e.ok()).next() {
        if let Ok(dims) = image::image_dimensions(first_frame.path()) {
            let (orig_w, orig_h) = dims;
            let target_w = (orig_w as f64 * scale_factor) as u32;
            let target_h = (orig_h as f64 * scale_factor) as u32;

            info!(
                "[UPSCALE] Original: {}x{}, Target: {}x{}",
                orig_w, orig_h, target_w, target_h
            );

            if target_w > 16384 || target_h > 16384 {
                return Err(format!(
                    "Safety Stop: Target resolution {}x{} exceeds 16K limit (16384px). Reduce scale factor.",
                    target_w, target_h
                ).into());
            }
        }
    }

    // 4. Vectorize & Render High-Res (Parallel)
    let paths: Vec<PathBuf> = fs::read_dir(&frames_src)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    info!(
        "[UPSCALE] Processing {} frames (Vectorize -> Render {}x)...",
        paths.len(),
        scale_factor
    );

    // Offload CPU-intensive task to blocking thread pool
    let frames_svg_clone = frames_svg.clone();
    let frames_out_clone = frames_out.clone();
    tokio::task::spawn_blocking(move || {
        process_frames_core(paths, frames_svg_clone, frames_out_clone, scale_factor);
    })
    .await?;

    // 5. Encode High-Res Video
    info!("[UPSCALE] Encoding high-resolution video...");
    let status_enc = Command::new("ffmpeg")
        .args([
            "-framerate",
            "12",
            "-i",
            frames_out.join("frame_%04d.png").to_str().unwrap(),
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-y",
            output.to_str().unwrap(),
        ])
        .output()
        .await?;

    // Cleanup
    fs::remove_dir_all(work_dir)?;

    if status_enc.status.success() {
        Ok(format!("Upscaled video saved to {:?}", output))
    } else {
        Err("FFmpeg encoding failed".into())
    }
}

/// Core logic for vectorizing and rendering frames.
/// Uses Optical Flow (temporal coherence) to avoid jitter.
fn process_frames_core(
    mut paths: Vec<PathBuf>,
    frames_svg: PathBuf,
    frames_out: PathBuf,
    scale_factor: f64,
) {
    // Ensure sequential order for temporal analysis
    paths.sort();

    let num_cpus = num_cpus::get();
    info!(
        "[UPSCALE] Memory Guard: Processing in chunks of {}",
        num_cpus
    );

    // 1. Temporal Analysis (Sequential Pre-pass)
    // Identify Keyframes based on visual difference
    let mut keyframe_map: Vec<usize> = Vec::with_capacity(paths.len());
    if !paths.is_empty() {
        keyframe_map.push(0); // First frame is always keyframe
    }

    let static_threshold = 0.02; // 2% pixel difference threshold

    info!("[UPSCALE] Analyzing temporal coherence (Optical Flow Check)...");
    for i in 1..paths.len() {
        let diff = calculate_optical_flow_diff(&paths[i], &paths[i - 1]);
        if diff < static_threshold {
            // Scene is static, reuse previous keyframe
            keyframe_map.push(keyframe_map[i - 1]);
        } else {
            // Scene changed, new keyframe
            keyframe_map.push(i);
        }
    }

    let mut unique_keyframes: Vec<usize> = keyframe_map.clone();
    unique_keyframes.sort();
    unique_keyframes.dedup();

    info!(
        "[UPSCALE] Temporal Optimization: {}/{} frames are static. Vectorizing {} keyframes.",
        paths.len() - unique_keyframes.len(),
        paths.len(),
        unique_keyframes.len()
    );

    // 2. Vectorize Keyframes (Parallel)
    let keyframe_indices = unique_keyframes;

    for chunk in keyframe_indices.chunks(num_cpus) {
        chunk.par_iter().for_each(|&idx| {
            let img_path = &paths[idx];
            let stem = img_path.file_stem().unwrap().to_string_lossy();
            let svg_path = frames_svg.join(format!("{}.svg", stem));

            // A. Vectorize (Raster -> SVG)
            let config = vtracer::Config {
                color_mode: vtracer::ColorMode::Color,
                hierarchical: vtracer::Hierarchical::Stacked,
                filter_speckle: 4,
                color_precision: 6,
                layer_difference: 16,
                corner_threshold: 60,
                splice_threshold: 45,
                ..Default::default()
            };

            if let Ok(_) = vtracer::convert_image_to_svg(img_path, &svg_path, config) {
                // Success
            } else {
                error!("Failed to vectorize frame: {:?}", img_path);
            }
        });
    }

    // 3. Render All Frames (Parallel)
    // Using the mapped keyframe SVG (reuse vectors)
    let all_indices: Vec<usize> = (0..paths.len()).collect();

    for chunk in all_indices.chunks(num_cpus) {
        chunk.par_iter().for_each(|&i| {
            let key_idx = keyframe_map[i];
            let key_stem = paths[key_idx].file_stem().unwrap().to_string_lossy();
            let key_svg_path = frames_svg.join(format!("{}.svg", key_stem));

            let current_stem = paths[i].file_stem().unwrap().to_string_lossy();
            let out_png = frames_out.join(format!("{}.png", current_stem));

            // B. Render (SVG -> High-Res Raster)
            if let Ok(svg_data) = fs::read(&key_svg_path) {
                let opt = usvg::Options::default();
                if let Ok(tree) = usvg::Tree::from_data(&svg_data, &opt) {
                    let size = tree.size.to_screen_size();
                    let width = (size.width() as f64 * scale_factor) as u32;
                    let height = (size.height() as f64 * scale_factor) as u32;

                    if let Some(mut pixmap) = tiny_skia::Pixmap::new(width, height) {
                        let transform = tiny_skia::Transform::from_scale(
                            scale_factor as f32,
                            scale_factor as f32,
                        );
                        resvg::render(&tree, usvg::FitTo::Original, transform, pixmap.as_mut());
                        pixmap.save_png(out_png).unwrap();
                    }
                }
            }
        });
    }
}

pub async fn upscale_video_cuda(
    input: &Path,
    scale_factor: f64,
    output: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    // use cudarc::driver::CudaDevice;

    // info!("[UPSCALE-CUDA] Initializing CUDA 13.1 context...");
    // let dev = match CudaDevice::new(0) {
    //     Ok(d) => d,
    //     Err(e) => {
    //         error!("[UPSCALE-CUDA] Failed to initialize CUDA: {:?}", e);
    //         return Err(format!("CUDA Error: {:?}", e).into());
    //     }
    // };

    // info!("[UPSCALE-CUDA] Using device: {:?}", dev.ordinal());

    // For now, satisfy the interface while we build out the kernels
    // Later phases will move the processing_frames_core logic to GPU kernels
    info!("[UPSCALE-CUDA] CUDA temporarily disabled due to build stub issues. Proceeding with CPU pipeline...");

    // Fallback to CPU for the actual processing logic until kernels are compiled
    upscale_video(input, scale_factor, output).await
}

/// Helper for GPU-based rendering (Stub - CUDA disabled)
#[allow(dead_code)]
fn render_svg_gpu(_data: &[u8], _scale: f64, _output: &Path) {
    // CUDA disabled - this function is not used
}

/// Helper for GPU-based vectorization (Stub - CUDA disabled)
#[allow(dead_code)]
fn vectorize_frame_cuda(_img_path: &Path) -> Vec<u8> {
    // CUDA disabled - returning empty bytes
    vec![]
}

/// Configuration struct passed from CLI/GUI
pub struct VectorConfig {
    pub colormode: String,
    pub hierarchical: String,
    pub filter_speckle: usize,
    pub color_precision: i32,
    pub layer_difference: i32,
    pub mode: String, // Kept for interface compatibility but ignored
    pub corner_threshold: i32,
    pub splice_threshold: i32,
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            colormode: "color".to_string(),
            hierarchical: "stacked".to_string(),
            filter_speckle: 4,
            color_precision: 6,
            layer_difference: 16,
            mode: "spline".to_string(),
            corner_threshold: 60,
            splice_threshold: 45,
        }
    }
}

/// Main function to vectorizing a video (SVG Output only)
pub async fn vectorize_video(
    input: &Path,
    output_dir: &Path,
    config: VectorConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    info!("[VECTOR] Starting vectorization engine on {:?}", input);

    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    // 1. Extract Frames using FFmpeg
    let frames_dir = output_dir.join("frames_src");
    fs::create_dir_all(&frames_dir)?;

    info!("[VECTOR] Extracting frames...");
    let status = Command::new("ffmpeg")
        .args([
            "-i",
            input.to_str().unwrap(),
            frames_dir.join("frame_%04d.png").to_str().unwrap(),
        ])
        .output()
        .await?;
    if !status.status.success() {
        return Err("FFmpeg frame extraction failed".into());
    }

    // 2. Vectorize Frames using vtracer (Parallelized)
    let paths: Vec<PathBuf> = fs::read_dir(&frames_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();

    info!("[VECTOR] Vectorizing {} frames (Parallel)...", paths.len());

    // Convert Config to vtracer Config
    let vt_config = vtracer::Config {
        color_mode: parse_colormode(&config.colormode),
        hierarchical: parse_hierarchical(&config.hierarchical),
        filter_speckle: config.filter_speckle,
        color_precision: config.color_precision,
        layer_difference: config.layer_difference,
        corner_threshold: config.corner_threshold,
        splice_threshold: config.splice_threshold,
        ..Default::default()
    };

    // Parallel processing with Rayon
    paths.par_iter().for_each(|frame_path| {
        let stem = frame_path.file_stem().unwrap().to_string_lossy();
        let out_svg = output_dir.join(format!("{}.svg", stem));

        match vtracer::convert_image_to_svg(frame_path, &out_svg, vt_config.clone()) {
            Ok(_) => {} // Silent success for speed
            Err(e) => error!("Failed frame {}: {}", stem, e),
        }
    });

    // 3. Cleanup Source Frames
    fs::remove_dir_all(&frames_dir)?;

    Ok(format!(
        "Vectorization complete. SVGs saved in {:?}",
        output_dir
    ))
}

// Helpers to map string configs to vtracer enums
fn parse_colormode(s: &str) -> vtracer::ColorMode {
    match s {
        "binary" => vtracer::ColorMode::Binary,
        _ => vtracer::ColorMode::Color,
    }
}

fn parse_hierarchical(s: &str) -> vtracer::Hierarchical {
    match s {
        "cutout" => vtracer::Hierarchical::Cutout,
        _ => vtracer::Hierarchical::Stacked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_process_frames_core() {
        // Setup temp dir
        let temp_dir = std::env::temp_dir().join("synoid_test_upscale");
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).unwrap();
        }
        let src_dir = temp_dir.join("src");
        let svg_dir = temp_dir.join("svg");
        let out_dir = temp_dir.join("out");

        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&svg_dir).unwrap();
        fs::create_dir_all(&out_dir).unwrap();

        // Create dummy image (100x100 red png)
        let img_path = src_dir.join("frame_0001.png");
        let mut img = tiny_skia::Pixmap::new(100, 100).unwrap();
        img.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        img.save_png(&img_path).unwrap();

        // Create duplicate image (100x100 red png)
        let img_path2 = src_dir.join("frame_0002.png");
        img.save_png(&img_path2).unwrap();

        let paths = vec![img_path, img_path2];

        // Run core logic
        process_frames_core(paths, svg_dir.clone(), out_dir.clone(), 2.0);

        // Verify output
        let out_path = out_dir.join("frame_0001.png");
        assert!(out_path.exists(), "Output PNG 1 should exist");
        let out_path2 = out_dir.join("frame_0002.png");
        assert!(out_path2.exists(), "Output PNG 2 should exist");

        // Check dimensions (200x200)
        let dims = image::image_dimensions(&out_path).unwrap();
        assert_eq!(dims, (200, 200));

        // Cleanup
        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
