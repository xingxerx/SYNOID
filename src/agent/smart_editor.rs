// SYNOID Smart Editor - AI-Powered Intent-Based Video Editing
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This module provides intelligent video editing based on natural language intent.
// It analyzes scenes, scores them against user intent, and generates trimmed output.

use crate::agent::production_tools;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use tokio::process::Command;
use tracing::{error, info, warn};

/// Configuration for the editing strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditingStrategy {
    pub scene_threshold: f64,
    pub min_scene_score: f64,
    pub boring_penalty_threshold: f64,
    pub speech_boost: f64,
    pub silence_penalty: f64,
    pub continuity_boost: f64,
    pub speech_ratio_threshold: f64,
    pub action_duration_threshold: f64,
}

impl Default for EditingStrategy {
    fn default() -> Self {
        Self {
            scene_threshold: 0.25,
            min_scene_score: 0.2,
            boring_penalty_threshold: 30.0,
            speech_boost: 0.4,
            silence_penalty: -0.4,
            continuity_boost: 0.6,
            speech_ratio_threshold: 0.1,
            action_duration_threshold: 3.0,
        }
    }
}

impl EditingStrategy {
    pub fn load() -> Self {
        // Try loading from JSON, fallback to default
        if let Ok(content) = fs::read_to_string("editing_strategy.json") {
            if let Ok(config) = serde_json::from_str(&content) {
                info!("[SMART] Loaded editing strategy from editing_strategy.json");
                return config;
            }
        }
        info!("[SMART] Using default editing strategy");
        Self::default()
    }
}

/// Represents an intent extracted from user input
#[derive(Debug, Clone)]
pub struct EditIntent {
    pub remove_boring: bool,
    pub keep_action: bool,
    pub remove_silence: bool,
    pub keep_speech: bool,
    pub ruthless: bool,
    pub custom_keywords: Vec<String>,
}

impl EditIntent {
    /// Parse natural language intent into structured intent
    pub fn from_text(text: &str) -> Self {
        let lower = text.to_lowercase();

        Self {
            remove_boring: lower.contains("boring")
                || lower.contains("lame")
                || lower.contains("dull")
                || lower.contains("slow"),
            keep_action: lower.contains("action")
                || lower.contains("exciting")
                || lower.contains("fast")
                || lower.contains("intense")
                || lower.contains("engaging")
                || lower.contains("interesting"),
            remove_silence: lower.contains("silence")
                || lower.contains("quiet")
                || lower.contains("dead air"),
            keep_speech: lower.contains("speech")
                || lower.contains("talking")
                || lower.contains("dialogue")
                || lower.contains("conversation")
                || lower.contains("voice")
                || lower.contains("transcript")
                || lower.contains("engaging"),
            ruthless: lower.contains("ruthless")
                || lower.contains("aggressive")
                || lower.contains("fast-paced")
                || lower.contains("no filler"),
            custom_keywords: vec![],
        }
    }

    /// Check if any editing intent was detected
    #[allow(dead_code)]
    pub fn has_intent(&self) -> bool {
        self.remove_boring || self.keep_action || self.remove_silence || self.keep_speech || self.ruthless
    }
}

/// Represents a detected scene in the video
#[derive(Debug, Clone)]
pub struct Scene {
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
    pub score: f64, // 0.0 = definitely remove, 1.0 = definitely keep
}

/// Detect scenes in a video using FFmpeg scene detection
pub async fn detect_scenes(
    input: &Path,
    threshold: f64,
) -> Result<Vec<Scene>, Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "[SMART] Detecting scenes in {:?} (threshold: {})",
        input, threshold
    );

    // Get total duration first
    let duration_output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            input.to_str().ok_or("Invalid input path")?,
        ])
        .output()
        .await?;

    let total_duration: f64 = String::from_utf8_lossy(&duration_output.stdout)
        .trim()
        .parse()
        .unwrap_or(0.0);

    if total_duration == 0.0 {
        return Err("Could not determine video duration".into());
    }

    info!("[SMART] Video duration: {:.2}s", total_duration);

    // Use FFmpeg to detect scene changes
    let output = Command::new("ffmpeg")
        .args([
            "-i",
            input.to_str().ok_or("Invalid input path")?,
            "-vf",
            &format!("select='gt(scene,{})',showinfo", threshold),
            "-f",
            "null",
            "-",
        ])
        .output()
        .await?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse scene timestamps from showinfo output
    let mut timestamps: Vec<f64> = vec![0.0]; // Start at 0

    for line in stderr.lines() {
        if line.contains("showinfo") && line.contains("pts_time:") {
            if let Some(pts_idx) = line.find("pts_time:") {
                let rest = &line[pts_idx + 9..];
                if let Some(space_idx) = rest.find(' ') {
                    if let Ok(ts) = rest[..space_idx].parse::<f64>() {
                        timestamps.push(ts);
                    }
                }
            }
        }
    }

    timestamps.push(total_duration); // End at total duration
    timestamps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    timestamps.dedup();

    // Convert timestamps to scenes
    let mut scenes = Vec::new();
    for i in 0..timestamps.len() - 1 {
        let start = timestamps[i];
        let end = timestamps[i + 1];
        let dur = end - start;

        // Skip very short segments (< 0.5s) - likely false positives
        if dur < 0.5 {
            continue;
        }

        scenes.push(Scene {
            start_time: start,
            end_time: end,
            duration: dur,
            score: 0.5, // Neutral score initially
        });
    }

    // If no scenes detected, treat entire video as one scene
    if scenes.is_empty() {
        scenes.push(Scene {
            start_time: 0.0,
            end_time: total_duration,
            duration: total_duration,
            score: 1.0,
        });
    }

    info!("[SMART] Detected {} scenes", scenes.len());
    Ok(scenes)
}

/// Score scenes based on user intent
pub fn score_scenes(
    scenes: &mut [Scene],
    intent: &EditIntent,
    config: &EditingStrategy,
) {
    info!("[SMART] Scoring {} scenes based on intent...", scenes.len());

    for scene in scenes.iter_mut() {
        let mut score: f64 = 0.3; // Base neutral-to-keep score

        if intent.remove_boring {
            let boring_penalty = 0.3;
            if scene.duration > config.boring_penalty_threshold {
                score -= boring_penalty;
            } else if scene.duration > 15.0 {
                score -= boring_penalty / 2.0;
            } else if scene.duration < 3.0 {
                score += 0.2;
            }
        }

        if intent.keep_action && scene.duration < config.action_duration_threshold {
            score += 0.3;
        }

        if intent.ruthless {
            score -= 0.1;
            if scene.duration < 1.5 {
                score += 0.2;
            }
        }

        scene.score = score.clamp(0.0, 1.0);
    }
}

/// Main smart editing function
pub async fn smart_edit(
    input: &Path,
    intent_text: &str,
    output: &Path,
    progress_callback: Option<Box<dyn Fn(&str) + Send + Sync>>,
    pre_scanned_scenes: Option<Vec<Scene>>,
    _unused: Option<()>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let log = move |msg: &str| {
        info!("{}", msg);
        if let Some(ref cb) = progress_callback {
            cb(msg);
        }
    };

    log("[SMART] 🧠 Starting AI-powered edit...");

    let mut output_buf = output.to_path_buf();
    if let Some(ext) = output_buf.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if ext_str == "txt" || !["mp4", "mkv", "mov", "avi"].contains(&ext_str.as_str()) {
            output_buf.set_extension("mp4");
            log(&format!("[SMART] ⚠️ Correcting output extension to .mp4: {:?}", output_buf));
        }
    } else {
        output_buf.set_extension("mp4");
        log(&format!("[SMART] ⚠️ Adding .mp4 extension: {:?}", output_buf));
    }
    let output = output_buf.as_path();

    let work_dir = input.parent().ok_or("Input path has no parent")?;
    let enhanced_audio_path = work_dir.join("synoid_audio_enhanced.wav");

    log("[SMART] 🎙️ Enhancing audio (High-Pass + Compression + Normalization)...");
    match production_tools::enhance_audio(input, &enhanced_audio_path).await {
        Ok(_) => log("[SMART] Audio enhanced successfully."),
        Err(e) => {
            warn!("[SMART] Audio enhancement failed ({}), using original.", e);
        }
    }

    let use_enhanced_audio = if let Ok(metadata) = fs::metadata(&enhanced_audio_path) {
        metadata.len() > 0
    } else {
        false
    };

    let config = EditingStrategy::load();
    let intent = EditIntent::from_text(intent_text);

    log(&format!(
        "[SMART] Intent: remove_boring={}, keep_action={}, keep_speech={}, ruthless={}",
        intent.remove_boring, intent.keep_action, intent.keep_speech, intent.ruthless
    ));

    // 2. Detect scenes
    log("[SMART] 🔍 Analyzing video scenes...");
    let mut scenes = if let Some(s) = pre_scanned_scenes {
        log(&format!("[SMART] Using pre-scanned scenes ({} scenes)", s.len()));
        s
    } else {
        detect_scenes(input, config.scene_threshold).await?
    };

    // 3. Score scenes based on intent
    log("[SMART] 📊 Scoring scenes based on intent...");
    score_scenes(&mut scenes, &intent, &config);

    // 4. Filter scenes to keep (score > threshold)
    let keep_threshold = config.min_scene_score;
    let total_before_filtering = scenes.len();
    let mut scenes_to_keep: Vec<Scene> = scenes.clone().into_iter().filter(|s| s.score > keep_threshold).collect();

    let mut total_kept = scenes_to_keep.len();
    let removed = total_before_filtering - total_kept;

    if scenes_to_keep.is_empty() {
        log("[SMART] ⚠️ All scenes were filtered out! Triggering Best-of Fallback...");
        // Sort all scenes by score descending and take the top 3 (or all if < 3)
        let mut all_scenes = scenes.clone();
        all_scenes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        scenes_to_keep = all_scenes.into_iter().take(3).collect();
        // Sort back by time
        scenes_to_keep.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap_or(std::cmp::Ordering::Equal));
        
        total_kept = scenes_to_keep.len();
        log(&format!("[SMART] 🎯 Fallback: Selected top {} highest-scoring segments.", total_kept));
    }

    log(&format!(
        "[SMART] Keeping {}/{} segments after refinement. Final duration: {:.2}s",
        total_kept,
        total_before_filtering,
        scenes_to_keep.iter().map(|s| s.duration).sum::<f64>()
    ));

    if scenes_to_keep.is_empty() {
        return Err("Fatal: Could not produce any segments even with fallback.".into());
    }

    // 5. Generate concat file or transition Inputs
    let segments_dir = work_dir.join("synoid_smart_edit_temp");
    if segments_dir.exists() {
        fs::remove_dir_all(&segments_dir)?;
    }
    fs::create_dir_all(&segments_dir)?;

    log("[SMART] ✂️ Extracting good segments (muxing enhanced audio)...");

    // Extract each segment
    let mut segment_files = Vec::new();
    let mut segment_durations = Vec::new();

    let total_segments = scenes_to_keep.len();
    for (i, scene) in scenes_to_keep.iter().enumerate() {
        let seg_path = segments_dir.join(format!("seg_{:04}.mp4", i));

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y").arg("-nostdin");

        // Accurate input-seeking (-ss and -t before -i) prevents frame doubling and lag
        cmd.arg("-ss").arg(&scene.start_time.to_string());
        cmd.arg("-t").arg(&scene.duration.to_string());
        cmd.arg("-i").arg(production_tools::safe_arg_path(input));

        if use_enhanced_audio {
            cmd.arg("-ss").arg(&scene.start_time.to_string());
            cmd.arg("-t").arg(&scene.duration.to_string());
            cmd.arg("-i").arg(production_tools::safe_arg_path(&enhanced_audio_path));
        }

        // Mapping
        // Always re-encode for frame accuracy (Fixes "doubling" issue)
        cmd.arg("-map").arg("0:v"); // Video from input 0

        if use_enhanced_audio {
            cmd.arg("-map").arg("1:a:0"); // Audio from input 1 (enhanced)
        } else {
            cmd.arg("-map").arg("0:a:0"); // Original audio
        }

        // CRF 23 is a good balance for quality/size. Preset faster for speed.
        cmd.arg("-c:v")
            .arg("libx264")
            .arg("-preset")
            .arg("faster")
            .arg("-crf")
            .arg("23");

        // Always re-encode audio to AAC to ensure format consistency
        cmd.arg("-c:a").arg("aac").arg("-b:a").arg("192k");

        cmd.arg("-avoid_negative_ts").arg("make_zero");
        cmd.arg(production_tools::safe_arg_path(&seg_path));

        let status = cmd.output().await?;

        if !status.status.success() {
            continue;
        }

        segment_files.push(seg_path);
        segment_durations.push(scene.duration);

        if i < 3 || i % 10 == 0 || i == total_segments - 1 {
            log(&format!(
                "[SMART] ⏳ Segment {}/{} processed",
                i + 1,
                total_segments
            ));
        }
    }

    if segment_files.is_empty() {
        fs::remove_dir_all(&segments_dir)?;
        return Err("Failed to extract any segments".into());
    }

    // 6. Simple Concat
    let concat_file = segments_dir.join("concat_list.txt");

    {
        let mut file = fs::File::create(&concat_file)?;
        for seg in &segment_files {
            writeln!(file, "file '{}'", seg.to_str().ok_or("Invalid segment path")?)?;
        }
    }

    log("[SMART] 🔗 Stitching segments together...");

    // 7. Concatenate segments
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-nostdin")
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg(production_tools::safe_arg_path(&concat_file))
        .arg("-c")
        .arg("copy")
        .arg(production_tools::safe_arg_path(output))
        .output()
        .await?;

    // Clean up
    fs::remove_dir_all(&segments_dir)?;
    if use_enhanced_audio {
        let _ = fs::remove_file(enhanced_audio_path);
    }

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        error!("[SMART] FFmpeg concat failed: {}", stderr);
        return Err("Failed to concatenate segments".into());
    }

    // Get output file size
    let metadata = fs::metadata(output)?;
    let size_mb = metadata.len() as f64 / 1_048_576.0;

    let summary = format!(
        "✅ Smart edit complete! Removed {} boring segments. Output: {:.2} MB",
        removed, size_mb
    );

    log(&format!("[SMART] {}", summary));

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_parsing() {
        let intent = EditIntent::from_text("Remove boring segments");
        assert!(intent.remove_boring);
        assert!(!intent.remove_silence);

        let intent2 = EditIntent::from_text("get rid of silence and dead air");
        assert!(intent2.remove_silence);
    }

    #[test]
    fn test_scoring_logic() {
        let mut scenes = vec![
            Scene {
                start_time: 0.0,
                end_time: 5.0,
                duration: 5.0,
                score: 0.5,
            },
        ];

        let intent = EditIntent::from_text("remove boring");
        let config = EditingStrategy::default();
        
        score_scenes(&mut scenes, &intent, &config);
        
        // No transcript provided, neutral score should remain around 0.3-0.5
        assert!(scenes[0].score >= 0.3);
    }
}
