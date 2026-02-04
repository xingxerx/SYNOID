// SYNOID Smart Editor - AI-Powered Intent-Based Video Editing
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This module provides intelligent video editing based on natural language intent.
// It analyzes scenes, scores them against user intent, and generates trimmed output.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use std::io::Write;
use tracing::{info, warn, error};

/// Represents an intent extracted from user input
#[derive(Debug, Clone)]
pub struct EditIntent {
    pub remove_boring: bool,
    pub keep_action: bool,
    pub remove_silence: bool,
    pub keep_speech: bool,
    pub custom_keywords: Vec<String>,
}

impl EditIntent {
    /// Parse natural language intent into structured intent
    pub fn from_text(text: &str) -> Self {
        let lower = text.to_lowercase();
        
        Self {
            remove_boring: lower.contains("boring") || lower.contains("lame") || 
                          lower.contains("dull") || lower.contains("slow"),
            keep_action: lower.contains("action") || lower.contains("exciting") ||
                        lower.contains("fast") || lower.contains("intense"),
            remove_silence: lower.contains("silence") || lower.contains("quiet") ||
                           lower.contains("dead air"),
            keep_speech: lower.contains("speech") || lower.contains("talking") ||
                        lower.contains("dialogue") || lower.contains("conversation"),
            custom_keywords: vec![],
        }
    }
    
    /// Check if any editing intent was detected
    pub fn has_intent(&self) -> bool {
        self.remove_boring || self.keep_action || self.remove_silence || self.keep_speech
    }
}

/// Represents a detected scene in the video
#[derive(Debug, Clone)]
pub struct Scene {
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
    pub score: f64,  // 0.0 = definitely remove, 1.0 = definitely keep
}

/// Detect scenes in a video using FFmpeg scene detection
pub fn detect_scenes(input: &Path) -> Result<Vec<Scene>, Box<dyn std::error::Error>> {
    info!("[SMART] Detecting scenes in {:?}", input);
    
    // Get total duration first
    let duration_output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            input.to_str().unwrap()
        ])
        .output()?;
    
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
            "-i", input.to_str().unwrap(),
            "-vf", "select='gt(scene,0.25)',showinfo",
            "-f", "null",
            "-"
        ])
        .output()?;
    
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
    timestamps.sort_by(|a, b| a.partial_cmp(b).unwrap());
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
pub fn score_scenes(scenes: &mut [Scene], intent: &EditIntent) {
    info!("[SMART] Scoring {} scenes based on intent", scenes.len());
    
    for scene in scenes.iter_mut() {
        let mut score: f64 = 0.5; // Start neutral
        
        if intent.remove_boring {
            // Heuristic: Long static scenes are often boring
            // Short scenes (< 3s) are usually cuts/action
            if scene.duration > 10.0 {
                score -= 0.3; // Penalize very long scenes
            } else if scene.duration > 5.0 {
                score -= 0.1;
            } else if scene.duration < 2.0 {
                score += 0.2; // Favor short/punchy scenes
            }
        }
        
        if intent.keep_action {
            // Short scenes often indicate action editing
            if scene.duration < 3.0 {
                score += 0.3;
            }
        }
        
        // Clamp score to 0-1 range
        scene.score = score.clamp(0.0_f64, 1.0_f64);
    }
}

/// Main smart editing function
pub async fn smart_edit(
    input: &Path,
    intent_text: &str,
    output: &Path,
    progress_callback: Option<Box<dyn Fn(&str) + Send>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let log = |msg: &str| {
        info!("{}", msg);
        if let Some(ref cb) = progress_callback {
            cb(msg);
        }
    };
    
    log("[SMART] 🧠 Starting AI-powered edit...");
    
    // Fix: Ensure output path has a valid video extension
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

    // 1. Parse intent
    let intent = EditIntent::from_text(intent_text);
    log(&format!("[SMART] Intent: remove_boring={}, keep_action={}", 
                 intent.remove_boring, intent.keep_action));
    
    if !intent.has_intent() {
        log("[SMART] ⚠️ No specific editing intent detected. Copying file as-is.");
        fs::copy(input, output)?;
        return Ok("No editing intent detected - file copied".to_string());
    }
    
    // 2. Detect scenes
    log("[SMART] 🔍 Analyzing video scenes...");
    let mut scenes = detect_scenes(input)?;
    log(&format!("[SMART] Found {} scenes", scenes.len()));
    
    // 3. Score scenes based on intent
    log("[SMART] 📊 Scoring scenes based on your intent...");
    score_scenes(&mut scenes, &intent);
    
    // 4. Filter scenes to keep (score > 0.3)
    let keep_threshold = 0.3;
    let scenes_to_keep: Vec<&Scene> = scenes.iter()
        .filter(|s| s.score > keep_threshold)
        .collect();
    
    let total_kept = scenes_to_keep.len();
    let total_original = scenes.len();
    let removed = total_original - total_kept;
    
    log(&format!("[SMART] Keeping {}/{} scenes (removing {} boring segments)", 
                 total_kept, total_original, removed));
    
    if scenes_to_keep.is_empty() {
        return Err("All scenes were filtered out! Try a less aggressive intent.".into());
    }
    
    // 5. Generate concat file for FFmpeg
    let work_dir = input.parent().unwrap_or(Path::new("."));
    let segments_dir = work_dir.join("synoid_smart_edit_temp");
    if segments_dir.exists() {
        fs::remove_dir_all(&segments_dir)?;
    }
    fs::create_dir_all(&segments_dir)?;
    
    log("[SMART] ✂️ Extracting good segments...");
    
    // Extract each segment
    let mut segment_files = Vec::new();
    let total_segments = scenes_to_keep.len();
    for (i, scene) in scenes_to_keep.iter().enumerate() {
        let seg_path = segments_dir.join(format!("seg_{:04}.mp4", i));
        
        // Progress update every 10 segments or for the first few
        if i < 3 || i % 10 == 0 || i == total_segments - 1 {
            log(&format!("[SMART] ⏳ Extracting segment {}/{} ({:.1}s @ {:.1}s)", 
                        i + 1, total_segments, scene.duration, scene.start_time));
        }
        
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-i", input.to_str().unwrap(),
                "-ss", &scene.start_time.to_string(),
                "-t", &scene.duration.to_string(),
                "-c", "copy",
                "-avoid_negative_ts", "make_zero",
                seg_path.to_str().unwrap(),
            ])
            .output()?;
        
        if !status.status.success() {
            warn!("[SMART] Failed to extract segment {}", i);
            continue;
        }
        
        segment_files.push(seg_path);
    }
    
    if segment_files.is_empty() {
        fs::remove_dir_all(&segments_dir)?;
        return Err("Failed to extract any segments".into());
    }
    
    // 6. Create concat list file
    let concat_file = segments_dir.join("concat_list.txt");
    {
        let mut file = fs::File::create(&concat_file)?;
        for seg in &segment_files {
            writeln!(file, "file '{}'", seg.to_str().unwrap())?;
        }
    }
    
    log("[SMART] 🔗 Stitching segments together...");
    
    // 7. Concatenate segments
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f", "concat",
            "-safe", "0",
            "-i", concat_file.to_str().unwrap(),
            "-c", "copy",
            output.to_str().unwrap(),
        ])
        .output()?;
    
    // Cleanup temp directory
    fs::remove_dir_all(&segments_dir)?;
    
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
        let intent = EditIntent::from_text("Remove boring and lame bits");
        assert!(intent.remove_boring);
        assert!(!intent.keep_action);
        
        let intent2 = EditIntent::from_text("Keep only the action moments");
        assert!(intent2.keep_action);
    }
}
