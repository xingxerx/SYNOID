// SYNOID Smart Editor - AI-Powered Intent-Based Video Editing
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This module provides intelligent video editing based on natural language intent.
// It analyzes scenes, scores them against user intent, and generates trimmed output.

use crate::agent::production_tools;
use crate::agent::voice::transcription::{TranscriptSegment, TranscriptionEngine};
use crate::funny_engine::commentator::CommentaryGenerator;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use tokio::process::Command;
use tracing::{error, info, warn};
const SILENCE_REFINEMENT_THRESHOLD: f64 = 0.5; // Seconds of silence to trigger a scene split

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
pub async fn detect_scenes(
    input: &Path,
    threshold: f64,
) -> Result<Vec<Scene>, Box<dyn std::error::Error>> {
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
            input.to_str().unwrap(),
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
            input.to_str().unwrap(),
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

/// NEW: Ensure scenes that carry a single sentence are kept together
fn ensure_speech_continuity(
    scenes: &mut [Scene],
    transcript: &[TranscriptSegment],
    config: &EditingStrategy,
) {
    info!(
        "[SMART] 🔗 Enforcing Speech Continuity (Boost: {})...",
        config.continuity_boost
    );

    // 1. Map sentences to scenes
    // If a sentence overlaps multiple scenes, and ANY of those scenes is 'kept' (score > 0.3),
    // we must force ALL overlapping scenes to be kept.

    for segment in transcript {
        // Find all scenes this segment touches
        let mut overlapping_indices = Vec::new();
        let mut should_preserve_sentence = false;

        for (i, scene) in scenes.iter().enumerate() {
            let overlap_start = segment.start.max(scene.start_time);
            let overlap_end = segment.end.min(scene.end_time);

            if overlap_end > overlap_start {
                overlapping_indices.push(i);
                // If any part of this sentence is already good enough to keep, save the whole thing
                if scene.score > 0.3 {
                    should_preserve_sentence = true;
                }
            }
        }

        // If we decided this sentence is important, boost all involved scenes
        if should_preserve_sentence {
            for i in overlapping_indices {
                if scenes[i].score <= 0.3 {
                    info!(
                        "[SMART] 🩹 Healing cut at {:.2}s to preserve speech: \"{}\"",
                        scenes[i].start_time, segment.text
                    );
                    scenes[i].score = config.continuity_boost; // Force keep above threshold
                }
            }
        }
    }
}

/// Refine visually detected scenes by splitting them based on transcript timestamps and gaps.
pub fn refine_scenes_with_transcript(
    scenes: Vec<Scene>,
    transcript: &[TranscriptSegment],
) -> Vec<Scene> {
    if transcript.is_empty() {
        return scenes;
    }

    let mut refined = Vec::new();
    let mut transcript_iter = transcript.iter().peekable();

    for scene in scenes {
        let mut current_start = scene.start_time;

        while let Some(segment) = transcript_iter.peek() {
            if segment.start >= scene.end_time {
                break;
            }

            // If there's a significant gap between current_start and segment.start, it's a silence
            if segment.start > current_start + SILENCE_REFINEMENT_THRESHOLD {
                refined.push(Scene {
                    start_time: current_start,
                    end_time: segment.start,
                    duration: segment.start - current_start,
                    score: 0.0, // Silence/Gap
                });
                current_start = segment.start;
            }

            // Case: Segment is within or partially within the scene
            let seg_end_bounded = segment.end.min(scene.end_time);
            if seg_end_bounded > current_start {
                refined.push(Scene {
                    start_time: current_start,
                    end_time: seg_end_bounded,
                    duration: seg_end_bounded - current_start,
                    score: 0.5, // Initial neutral score
                });
                current_start = seg_end_bounded;
            }

            // Move to next segment if we've fully consumed this one
            if segment.end <= scene.end_time {
                transcript_iter.next();
            } else {
                // Segment spans across to next visual scene, don't consume it yet
                break;
            }
        }

        // Add remaining tail of the visual scene as silence/gap if it's long enough
        if scene.end_time > current_start + 0.1 {
            refined.push(Scene {
                start_time: current_start,
                end_time: scene.end_time,
                duration: scene.end_time - current_start,
                score: 0.0,
            });
        }
    }

    // Merge adjacent segments that are both low-score/silence if needed?
    // For now, just return as is.
    refined
}

/// Score scenes based on user intent and transcript
pub fn score_scenes(
    scenes: &mut [Scene],
    intent: &EditIntent,
    transcript: Option<&[TranscriptSegment]>,
    config: &EditingStrategy,
) {
    info!("[SMART] Scoring {} scenes based on intent...", scenes.len());

    // 1. Base Scoring
    for scene in scenes.iter_mut() {
        let mut score: f64 = 0.3; // Base neutral-to-keep score

        // Visual Heuristics
        if intent.remove_boring {
            let boring_penalty = 0.3;
            if scene.duration > config.boring_penalty_threshold {
                score -= boring_penalty;
            } else if scene.duration > 15.0 {
                score -= boring_penalty / 2.0;
            } else if scene.duration < 3.0 {
                score += 0.2; // Prefer shorter segments for "not boring"
            }
        }

        if intent.keep_action && scene.duration < config.action_duration_threshold {
            score += 0.3;
        }

        // Semantic Heuristics (Transcript Analysis)
        if let Some(segments) = transcript {
            let mut speech_duration = 0.0;
            let mut has_keyword = false;

            for seg in segments {
                let seg_start = seg.start.max(scene.start_time);
                let seg_end = seg.end.min(scene.end_time);

                if seg_end > seg_start {
                    speech_duration += seg_end - seg_start;
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

            // More nuanced speech scoring
            if intent.keep_speech {
                if speech_ratio > config.speech_ratio_threshold {
                    score += config.speech_boost;
                }
            } else {
                if speech_ratio > 0.3 {
                    score += config.speech_boost;
                }
            }

            if intent.remove_silence {
                let penalty = config.silence_penalty;
                if speech_ratio < 0.05 {
                    score += penalty;
                } else if speech_ratio < 0.2 {
                    score += penalty / 2.0;
                }
            }

            if has_keyword {
                score += 0.5;
            }
        }

        scene.score = score.clamp(0.0, 1.0);
    }

    // 2. Post-Scoring: Integrity Pass
    // Always apply continuity protection unless we specifically find a reason to skip it
    if let Some(segments) = transcript {
        ensure_speech_continuity(scenes, segments, config);
    }
}

/// Main smart editing function
pub async fn smart_edit(
    input: &Path,
    intent_text: &str,
    output: &Path,
    funny_mode: bool,
    progress_callback: Option<Box<dyn Fn(&str) + Send + Sync>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let log = move |msg: &str| {
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
        let engine = TranscriptionEngine::new().map_err(|e| e.to_string())?;
        match engine.transcribe(&enhanced_audio_path).await {
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

    // Load Strategy
    let config = EditingStrategy::load();

    // 1. Parse intent
    // 1. Parse intent
    let intent = EditIntent::from_text(intent_text);
    // REMOVED: Implicit override that protected all speech.
    // User intent (e.g. "ruthless") should now override transcript protection if desired.

    log(&format!(
        "[SMART] Intent: remove_boring={}, keep_action={}, keep_speech={}",
        intent.remove_boring, intent.keep_action, intent.keep_speech
    ));

    // 2. Detect scenes
    log("[SMART] 🔍 Analyzing video scenes...");
    let mut scenes = detect_scenes(input, config.scene_threshold).await?;

    // 2.5 Refine scenes with transcript (Split by silences)
    if let Some(t) = &transcript {
        log("[SMART] 🛠️ Refining scene boundaries with transcript gaps...");
        scenes = refine_scenes_with_transcript(scenes, t);
    }

    // 3. Score scenes based on intent AND transcript
    log("[SMART] 📊 Scoring scenes based on semantic data...");
    score_scenes(&mut scenes, &intent, transcript.as_deref(), &config);

    // 4. Filter scenes to keep (score > threshold)
    let keep_threshold = config.min_scene_score;
    let total_before_filtering = scenes.len();
    let scenes_to_keep: Vec<Scene> = scenes.into_iter().filter(|s| s.score > keep_threshold).collect();

    let total_kept = scenes_to_keep.len();
    let removed = total_before_filtering - total_kept;

    log(&format!(
        "[SMART] Keeping {}/{} segments after refinement. Final duration: {:.2}s",
        total_kept,
        total_before_filtering,
        scenes_to_keep.iter().map(|s| s.duration).sum::<f64>()
    ));

    if scenes_to_keep.is_empty() {
        return Err("All scenes were filtered out! Try a less aggressive intent.".into());
    }

    // 5. Generate concat file or transition Inputs
    let segments_dir = work_dir.join("synoid_smart_edit_temp");
    if segments_dir.exists() {
        fs::remove_dir_all(&segments_dir)?;
    }
    fs::create_dir_all(&segments_dir)?;

    log("[SMART] ✂️ Extracting good segments (muxing enhanced audio)...");

    // Initialize Commentary Generator if needed
    let commentator = if funny_mode {
        match CommentaryGenerator::new("http://localhost:11434/v1") {
            Ok(c) => Some(c),
            Err(e) => {
                warn!("[SMART] Failed to init Funny Engine: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Extract each segment
    let mut segment_files = Vec::new();
    let mut commentary_files = Vec::new(); // (index, path)
    let mut segment_durations = Vec::new();

    let total_segments = scenes_to_keep.len();
    for (i, scene) in scenes_to_keep.iter().enumerate() {
        let seg_path = segments_dir.join(format!("seg_{:04}.mp4", i));

        // Generate Commentary
        if let Some(gen) = &commentator {
            // Only commentate on longer scenes to avoid clutter
            if scene.duration > 4.0 {
                let context = if let Some(t) = &transcript {
                    t.iter()
                        .filter(|s| s.end > scene.start_time && s.start < scene.end_time)
                        .map(|s| s.text.clone())
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    "Visual scene".to_string()
                };

                // Generate asynchronously (blocking here for simplicity in this iteration)
                if let Ok(Some(audio_path)) = gen
                    .generate_commentary(scene, &context, &segments_dir, i)
                    .await
                {
                    commentary_files.push((i, audio_path));
                }
            }
        }

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y").arg("-nostdin");

        // Accurate input-seeking (-ss and -t before -i) prevents frame doubling and lag
        cmd.arg("-ss").arg(&scene.start_time.to_string());
        cmd.arg("-t").arg(&scene.duration.to_string());
        cmd.arg("-i").arg(input.to_str().unwrap());

        if use_enhanced_audio {
            cmd.arg("-ss").arg(&scene.start_time.to_string());
            cmd.arg("-t").arg(&scene.duration.to_string());
            cmd.arg("-i").arg(enhanced_audio_path.to_str().unwrap());
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
        cmd.arg(seg_path.to_str().unwrap());

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

    if funny_mode {
        log("[SMART] 🎭 Funny Mode: Rendering transitions and commentary...");

        // 6a. Complex Logic for Funny Mode
        let transition_duration = 0.5;
        let filter_complex = production_tools::build_transition_filter(
            segment_files.len(),
            transition_duration,
            &segment_durations,
        );

        if filter_complex.is_empty() {
            // Fallback to simple concat if only 1 clip
            log("[SMART] Only 1 clip, skipping transitions.");
        } else {
            let mut cmd = Command::new("ffmpeg");
            cmd.arg("-y").arg("-nostdin");

            // Inputs (Video Segments)
            for seg in &segment_files {
                cmd.arg("-i").arg(seg);
            }

            // Inputs (Commentary Audio)
            // We need to mix these in.
            // Complex mixing logic omitted for brevity in this step,
            // focusing on Visual Transitions first as requested.
            // (Commentary overlay would require amix or adelay filter injection)

            // Apply Transition Filter
            cmd.arg("-filter_complex").arg(&filter_complex);

            // Map output from filter (v{last}, a{last})
            let last_idx = segment_files.len();
            cmd.arg("-map").arg(format!("[v{}]", last_idx));
            cmd.arg("-map").arg(format!("[a{}]", last_idx));

            cmd.arg("-c:v")
                .arg("libx264")
                .arg("-preset")
                .arg("medium")
                .arg("-crf")
                .arg("23");
            cmd.arg("-c:a").arg("aac").arg("-b:a").arg("192k");
            cmd.arg(output.to_str().unwrap());

            let status = cmd.output().await?;
            if !status.status.success() {
                let stderr = String::from_utf8_lossy(&status.stderr);
                // Fallback to simple concat if complex filter fails (e.g. too many inputs)
                error!(
                    "[SMART] Transition render failed: {}. Falling back to simple cut.",
                    stderr
                );
            } else {
                // Success path
                fs::remove_dir_all(&segments_dir)?;
                if use_enhanced_audio {
                    let _ = fs::remove_file(enhanced_audio_path);
                }

                let metadata = fs::metadata(output)?;
                let size_mb = metadata.len() as f64 / 1_048_576.0;
                return Ok(format!("✅ Funny Edit complete! Output: {:.2} MB", size_mb));
            }
        }
    }

    // 6b. Simple Concat (Default or Fallback)
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
    fn test_refine_scenes_with_transcript() {
        let scenes = vec![Scene {
            start_time: 0.0,
            end_time: 10.0,
            duration: 10.0,
            score: 0.5,
        }];

        let transcript = vec![
            TranscriptSegment {
                start: 1.0,
                end: 3.0,
                text: "Hello".to_string(),
            },
            TranscriptSegment {
                start: 7.0,
                end: 9.0,
                text: "World".to_string(),
            },
        ];

        let refined = refine_scenes_with_transcript(scenes, &transcript);
        
        // Expected:
        // 0-1: Silence (score: 0.0)
        // 1-3: Speech (score: 0.5 initially)
        // 3-7: Silence (score: 0.0)
        // 7-9: Speech
        // 9-10: Silence
        
        assert_eq!(refined.len(), 5);
        assert_eq!(refined[0].score, 0.0);
        assert_eq!(refined[1].score, 0.5);
        assert_eq!(refined[2].score, 0.0);
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
        
        score_scenes(&mut scenes, &intent, None, &config);
        
        // No transcript provided, neutral score should remain around 0.3-0.5
        assert!(scenes[0].score >= 0.3);
    }
}
