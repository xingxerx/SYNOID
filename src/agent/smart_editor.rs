<<<<<<< HEAD
// SYNOID Smart Editor - AI-Powered Intent-Based Video Editing
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This module provides intelligent video editing based on natural language intent.
// It analyzes scenes, scores them against user intent, and generates trimmed output.

use std::path::Path;
use std::process::Command;
use std::fs;
use std::io::Write;
use tracing::{info, warn, error};
use crate::agent::production_tools;
use crate::agent::voice::transcription::{TranscriptionEngine, TranscriptSegment};

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
    #[allow(dead_code)]
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
            "--",
        ])
        .arg(input)
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
        .arg("-i")
        .arg(input)
        .args([
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
/// Score scenes based on user intent and transcript
pub fn score_scenes(scenes: &mut [Scene], intent: &EditIntent, transcript: Option<&[TranscriptSegment]>) {
    info!("[SMART] Scoring {} scenes based on intent and semantic data", scenes.len());
    
    for scene in scenes.iter_mut() {
        let mut score: f64 = 0.5; // Start neutral
        
        // --- Visual Heuristics ---
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

        // --- Semantic Heuristics (Transcript Analysis) ---
        if let Some(segments) = transcript {
            // Find speech segments overlapping this scene
            let mut speech_duration = 0.0;
            let mut has_keyword = false;
            
            for seg in segments {
                // Check overlap
                let seg_start = seg.start.max(scene.start_time);
                let seg_end = seg.end.min(scene.end_time);
                
                if seg_end > seg_start {
                    speech_duration += seg_end - seg_start;
                    
                    // Keyword boost
                    if !intent.custom_keywords.is_empty() {
                         let text_lower = seg.text.to_lowercase();
                         for keyword in &intent.custom_keywords {
                             if text_lower.contains(&keyword.to_lowercase()) {
                                 has_keyword = true;
                             }
                         }
                    }
                }
            }
            
            let speech_ratio = speech_duration / scene.duration;
            
            if speech_ratio > 0.3 {
                // If sufficient speech, boost significantly (User wants to hear voice)
                score += 0.4;
            } else if speech_ratio < 0.1 {
                // Almost silent
                if intent.remove_silence {
                    score -= 0.4;
                }
            }
            
            if has_keyword {
                score += 0.5; // Strong boost for keywords
                info!("[SMART] Keyword found in scene {:.1}-{:.1}", scene.start_time, scene.end_time);
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

    // 0. Pre-process: Enhance Audio & Transcribe
    // This creates a clean audio spine for the edit
    let work_dir = input.parent().unwrap_or(Path::new("."));
    let enhanced_audio_path = work_dir.join("synoid_audio_enhanced.wav");
    
    log("[SMART] 🎙️ Enhancing audio (High-Pass + Compression + Normalization)...");
    match production_tools::enhance_audio(input, &enhanced_audio_path).await {
        Ok(_) => log("[SMART] Audio enhanced successfully."),
        Err(e) => {
            warn!("[SMART] Audio enhancement failed ({}), using original.", e);
            // Fallback: Just use original input as audio source if possible, or skip enhancement
        }
    }
    
    let use_enhanced_audio = if let Ok(metadata) = fs::metadata(&enhanced_audio_path) {
        metadata.len() > 0
    } else {
        false
    };
    
    // Transcribe
    log("[SMART] 📝 Transcribing audio for semantic understanding...");
    let transcript = if use_enhanced_audio {
        match TranscriptionEngine::new() {
            Ok(engine) => {
                match engine.transcribe(&enhanced_audio_path) {
                    Ok(t) => {
                        log(&format!("[SMART] Transcription complete: {} segments", t.len()));
                        Some(t)
                    },
                    Err(e) => {
                        warn!("[SMART] Transcription failed: {}", e);
                        None
                    }
                }
            },
            Err(e) => {
                warn!("[SMART] Failed to initialize transcription engine: {}", e);
                None
            }
        }
    } else {
        None
    };

    // 1. Parse intent
    let mut intent = EditIntent::from_text(intent_text);
    // Explicitly add "voice" intent if we have a transcript, to leverage speech scoring
    if transcript.is_some() {
        intent.keep_speech = true;
        intent.remove_silence = true;
    }
    
    log(&format!("[SMART] Intent: remove_boring={}, keep_action={}, keep_speech={}", 
                 intent.remove_boring, intent.keep_action, intent.keep_speech));
    
    // 2. Detect scenes
    log("[SMART] 🔍 Analyzing video scenes...");
    let mut scenes = detect_scenes(input)?;
    
    // 3. Score scenes based on intent AND transcript
    log("[SMART] 📊 Scoring scenes based on semantic data...");
    score_scenes(&mut scenes, &intent, transcript.as_deref());
    
    // 4. Filter scenes to keep (score > 0.3)
    let keep_threshold = 0.3;
    let scenes_to_keep: Vec<&Scene> = scenes.iter()
        .filter(|s| s.score > keep_threshold)
        .collect();
    
    let total_kept = scenes_to_keep.len();
    let total_original = scenes.len();
    let removed = total_original - total_kept;
    
    log(&format!("[SMART] Keeping {}/{} scenes (removing {} boring/silent segments)", 
                 total_kept, total_original, removed));
    
    if scenes_to_keep.is_empty() {
        return Err("All scenes were filtered out! Try a less aggressive intent.".into());
    }
    
    // 5. Generate concat file for FFmpeg
    let segments_dir = work_dir.join("synoid_smart_edit_temp");
    if segments_dir.exists() {
        fs::remove_dir_all(&segments_dir)?;
    }
    fs::create_dir_all(&segments_dir)?;
    
    log("[SMART] ✂️ Extracting good segments (muxing enhanced audio)...");
    
    // Extract each segment
    let mut segment_files = Vec::new();
    let total_segments = scenes_to_keep.len();
    for (i, scene) in scenes_to_keep.iter().enumerate() {
        let seg_path = segments_dir.join(format!("seg_{:04}.mp4", i));
        
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y").arg("-nostdin");
        
        // Input 0: Video
        cmd.arg("-i").arg(input);
        
        // Input 1: Enhanced Audio (if available)
        if use_enhanced_audio {
            cmd.arg("-i").arg(&enhanced_audio_path);
        }
        
        cmd.arg("-ss").arg(&scene.start_time.to_string());
        cmd.arg("-t").arg(&scene.duration.to_string());
        
        // Mapping
        cmd.arg("-map").arg("0:v"); // Video from input 0
        if use_enhanced_audio {
            cmd.arg("-map").arg("1:a:0"); // Audio from input 1 (enhanced)
            cmd.arg("-c:v").arg("copy");  // Copy video stream (fast)
            cmd.arg("-c:a").arg("aac").arg("-b:a").arg("192k"); // Re-encode audio to mux
        } else {
            cmd.arg("-map").arg("0:a:0"); // Original audio
            cmd.arg("-c").arg("copy");    // Copy both
        }
        
        cmd.arg("-avoid_negative_ts").arg("make_zero");
        cmd.arg(&seg_path);

        let status = cmd.output()?;
        
        if !status.status.success() {
            // warn!("[SMART] Failed to extract segment {}", i);
            // Retry without enhanced audio if that was the cause?
            // For now, simple fail safe
           continue;
        }
        
        segment_files.push(seg_path);
        
        if i < 3 || i % 10 == 0 || i == total_segments - 1 {
             log(&format!("[SMART] ⏳ Segment {}/{} processed", i + 1, total_segments));
        }
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
            if let Some(s) = seg.to_str() {
                writeln!(file, "file '{}'", s)?;
            } else {
                return Err("Invalid path encoding for segment file".into());
            }
        }
    }
    
    log("[SMART] 🔗 Stitching segments together...");
    
    // 7. Concatenate segments
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-nostdin",
            "-f", "concat",
            "-safe", "0",
        ])
        .arg("-i")
        .arg(&concat_file)
        .args([
            "-c", "copy",
        ])
        .arg(output)
        .output()?;
        
    // Clean up
    fs::remove_dir_all(&segments_dir)?;
    if use_enhanced_audio {
        // fs::remove_file(enhanced_audio_path)?; // Keep for debug if needed, or delete
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

        .output()?;
        
    // Clean up
    fs::remove_dir_all(&segments_dir)?;
    if use_enhanced_audio {
        // fs::remove_file(enhanced_audio_path)?; // Keep for debug if needed, or delete
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
        let intent = EditIntent::from_text("Remove boring and lame bits");
        assert!(intent.remove_boring);
        assert!(!intent.keep_action);
        
        let intent2 = EditIntent::from_text("Keep only the action moments");
        assert!(intent2.keep_action);
    }
}
// SYNOID Smart Editor - AI-Powered Intent-Based Video Editing
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This module provides intelligent video editing based on natural language intent.
// It analyzes scenes, scores them against user intent, and generates trimmed output.

use crate::agent::production_tools;
use crate::agent::voice::transcription::{TranscriptSegment, TranscriptionEngine};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tracing::{error, info, warn};

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
            remove_boring: lower.contains("boring")
                || lower.contains("lame")
                || lower.contains("dull")
                || lower.contains("slow"),
            keep_action: lower.contains("action")
                || lower.contains("exciting")
                || lower.contains("fast")
                || lower.contains("intense"),
            remove_silence: lower.contains("silence")
                || lower.contains("quiet")
                || lower.contains("dead air"),
            keep_speech: lower.contains("speech")
                || lower.contains("talking")
                || lower.contains("dialogue")
                || lower.contains("conversation"),
            custom_keywords: vec![],
        }
    }

    /// Check if any editing intent was detected
    #[allow(dead_code)]
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
    pub score: f64, // 0.0 = definitely remove, 1.0 = definitely keep
}

/// Detect scenes in a video using FFmpeg scene detection
pub fn detect_scenes(input: &Path) -> Result<Vec<Scene>, Box<dyn std::error::Error>> {
    info!("[SMART] Detecting scenes in {:?}", input);

    // Get total duration first
    let duration_output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            input.to_str().unwrap(),
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
            "-i",
            input.to_str().unwrap(),
            "-vf",
            "select='gt(scene,0.25)',showinfo",
            "-f",
            "null",
            "-",
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
/// Score scenes based on user intent and transcript
pub fn score_scenes(
    scenes: &mut [Scene],
    intent: &EditIntent,
    transcript: Option<&[TranscriptSegment]>,
) {
    info!(
        "[SMART] Scoring {} scenes based on intent and semantic data",
        scenes.len()
    );

    for scene in scenes.iter_mut() {
        let mut score: f64 = 0.5; // Start neutral

        // --- Visual Heuristics ---
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

        // --- Semantic Heuristics (Transcript Analysis) ---
        if let Some(segments) = transcript {
            // Find speech segments overlapping this scene
            let mut speech_duration = 0.0;
            let mut has_keyword = false;

            for seg in segments {
                // Check overlap
                let seg_start = seg.start.max(scene.start_time);
                let seg_end = seg.end.min(scene.end_time);

                if seg_end > seg_start {
                    speech_duration += seg_end - seg_start;

                    // Keyword boost
                    if !intent.custom_keywords.is_empty() {
                        let text_lower = seg.text.to_lowercase();
                        for keyword in &intent.custom_keywords {
                            if text_lower.contains(&keyword.to_lowercase()) {
                                has_keyword = true;
                            }
                        }
                    }
                }
            }

            let speech_ratio = speech_duration / scene.duration;

            if speech_ratio > 0.3 {
                // If sufficient speech, boost significantly (User wants to hear voice)
                score += 0.4;
            } else if speech_ratio < 0.1 {
                // Almost silent
                if intent.remove_silence {
                    score -= 0.4;
                }
            }

            if has_keyword {
                score += 0.5; // Strong boost for keywords
                info!(
                    "[SMART] Keyword found in scene {:.1}-{:.1}",
                    scene.start_time, scene.end_time
                );
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
            log(&format!(
                "[SMART] ⚠️ Correcting output extension to .mp4: {:?}",
                output_buf
            ));
        }
    } else {
        output_buf.set_extension("mp4");
        log(&format!(
            "[SMART] ⚠️ Adding .mp4 extension: {:?}",
            output_buf
        ));
    }
    let output = output_buf.as_path();

    // 0. Pre-process: Enhance Audio & Transcribe
    // This creates a clean audio spine for the edit
    let work_dir = input.parent().unwrap_or(Path::new("."));
    let enhanced_audio_path = work_dir.join("synoid_audio_enhanced.wav");

    log("[SMART] 🎙️ Enhancing audio (High-Pass + Compression + Normalization)...");
    match production_tools::enhance_audio(input, &enhanced_audio_path).await {
        Ok(_) => log("[SMART] Audio enhanced successfully."),
        Err(e) => {
            warn!("[SMART] Audio enhancement failed ({}), using original.", e);
            // Fallback: Just use original input as audio source if possible, or skip enhancement
        }
    }

    let use_enhanced_audio = if let Ok(metadata) = fs::metadata(&enhanced_audio_path) {
        metadata.len() > 0
    } else {
        false
    };

    // Transcribe
    log("[SMART] 📝 Transcribing audio for semantic understanding...");
    let transcript = if use_enhanced_audio {
        let engine = TranscriptionEngine::new();
        match engine.transcribe(&enhanced_audio_path) {
            Ok(t) => {
                log(&format!(
                    "[SMART] Transcription complete: {} segments",
                    t.len()
                ));
                Some(t)
            }
            Err(e) => {
                warn!("[SMART] Transcription failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    // 1. Parse intent
    let mut intent = EditIntent::from_text(intent_text);
    // Explicitly add "voice" intent if we have a transcript, to leverage speech scoring
    if transcript.is_some() {
        intent.keep_speech = true;
        intent.remove_silence = true;
    }

    log(&format!(
        "[SMART] Intent: remove_boring={}, keep_action={}, keep_speech={}",
        intent.remove_boring, intent.keep_action, intent.keep_speech
    ));

    // 2. Detect scenes
    log("[SMART] 🔍 Analyzing video scenes...");
    let mut scenes = detect_scenes(input)?;

    // 3. Score scenes based on intent AND transcript
    log("[SMART] 📊 Scoring scenes based on semantic data...");
    score_scenes(&mut scenes, &intent, transcript.as_deref());

    // 4. Filter scenes to keep (score > 0.3)
    let keep_threshold = 0.3;
    let scenes_to_keep: Vec<&Scene> = scenes.iter().filter(|s| s.score > keep_threshold).collect();

    let total_kept = scenes_to_keep.len();
    let total_original = scenes.len();
    let removed = total_original - total_kept;

    log(&format!(
        "[SMART] Keeping {}/{} scenes (removing {} boring/silent segments)",
        total_kept, total_original, removed
    ));

    if scenes_to_keep.is_empty() {
        return Err("All scenes were filtered out! Try a less aggressive intent.".into());
    }

    // 5. Generate concat file for FFmpeg
    let segments_dir = work_dir.join("synoid_smart_edit_temp");
    if segments_dir.exists() {
        fs::remove_dir_all(&segments_dir)?;
    }
    fs::create_dir_all(&segments_dir)?;

    log("[SMART] ✂️ Extracting good segments (muxing enhanced audio)...");

    // Extract each segment
    let mut segment_files = Vec::new();
    let total_segments = scenes_to_keep.len();
    for (i, scene) in scenes_to_keep.iter().enumerate() {
        let seg_path = segments_dir.join(format!("seg_{:04}.mp4", i));

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y").arg("-nostdin");

        // Input 0: Video
        cmd.arg("-i").arg(input.to_str().unwrap());

        // Input 1: Enhanced Audio (if available)
        if use_enhanced_audio {
            cmd.arg("-i").arg(enhanced_audio_path.to_str().unwrap());
        }

        cmd.arg("-ss").arg(&scene.start_time.to_string());
        cmd.arg("-t").arg(&scene.duration.to_string());

        // Mapping
        cmd.arg("-map").arg("0:v"); // Video from input 0
        if use_enhanced_audio {
            cmd.arg("-map").arg("1:a:0"); // Audio from input 1 (enhanced)
            cmd.arg("-c:v").arg("copy"); // Copy video stream (fast)
            cmd.arg("-c:a").arg("aac").arg("-b:a").arg("192k"); // Re-encode audio to mux
        } else {
            cmd.arg("-map").arg("0:a:0"); // Original audio
            cmd.arg("-c").arg("copy"); // Copy both
        }

        cmd.arg("-avoid_negative_ts").arg("make_zero");
        cmd.arg(seg_path.to_str().unwrap());

        let status = cmd.output()?;

        if !status.status.success() {
            // warn!("[SMART] Failed to extract segment {}", i);
            // Retry without enhanced audio if that was the cause?
            // For now, simple fail safe
            continue;
        }

        segment_files.push(seg_path);

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
            "-nostdin",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            concat_file.to_str().unwrap(),
            "-c",
            "copy",
            output.to_str().unwrap(),
        ])
        .output()?;

    // Clean up
    fs::remove_dir_all(&segments_dir)?;
    if use_enhanced_audio {
        // fs::remove_file(enhanced_audio_path)?; // Keep for debug if needed, or delete
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
        let intent = EditIntent::from_text("Remove boring and lame bits");
        assert!(intent.remove_boring);
        assert!(!intent.keep_action);

        let intent2 = EditIntent::from_text("Keep only the action moments");
        assert!(intent2.keep_action);
    }
}
>>>>>>> pr-7
