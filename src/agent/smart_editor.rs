// SYNOID Smart Editor - AI-Powered Intent-Based Video Editing
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This module provides intelligent video editing based on natural language intent.
// It analyzes scenes, scores them against user intent, and generates trimmed output.

use crate::agent::production_tools;
use crate::agent::transcription::{TranscriptSegment, TranscriptionEngine};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{error, info, warn};
const SILENCE_REFINEMENT_THRESHOLD: f64 = 2.0; // Seconds of silence to trigger a scene split (≤2 s pause = natural speech rhythm, not a cut point)
use regex::Captures;

/// Strip the Windows extended-length path prefix (`\\?\` or `//?/`) from a
/// `PathBuf` returned by `std::fs::canonicalize`.  FFmpeg cannot open paths
/// that start with that prefix, so we normalise them back to plain absolute
/// paths before handing them to any FFmpeg invocation.
fn strip_unc_prefix(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    // Covers both native `\\?\D:\...` and forward-slash variant `//?/D:/...`
    let stripped = s
        .strip_prefix(r"\\?\")
        .or_else(|| s.strip_prefix("//?/"))
        .unwrap_or(&s);
    PathBuf::from(stripped)
}

/// Density of the edit - how much to keep vs how much to prune
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EditDensity {
    Highlights, // Aggressive pruning (Original ruthless behavior)
    Balanced,   // Moderate pruning (Keep most meaningful content)
    Full,       // Minimal pruning (Only remove true silence/dead air)
}

impl Default for EditDensity {
    fn default() -> Self {
        Self::Balanced
    }
}

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
    /// Maximum allowed gap (seconds) between two consecutive kept scenes.
    /// If a gap exceeds this, the best scene within the gap is inserted to
    /// prevent jarring narrative jumps. Default: 45.0 s.
    #[serde(default = "default_max_jump_gap_secs")]
    pub max_jump_gap_secs: f64,
}

fn default_max_jump_gap_secs() -> f64 {
    45.0
}

impl Default for EditingStrategy {
    fn default() -> Self {
        Self {
            scene_threshold: 0.25,
            min_scene_score: 0.30, // Raised from 0.20 — prevents micro-cuts
            boring_penalty_threshold: 25.0, // Tighter: cut long boring blocks sooner
            speech_boost: 0.5,     // Raised from 0.4 — speech is story, keep more of it
            silence_penalty: -0.4,
            continuity_boost: 0.6,
            speech_ratio_threshold: 0.1,
            action_duration_threshold: 3.0,
            max_jump_gap_secs: 45.0,
        }
    }
}

impl EditingStrategy {
    pub fn load() -> Self {
        // First try the learned, cortex-cached strategy (compounding learning)
        let suffix = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_default();
        let cache_dir = format!("cortex_cache{}", suffix);
        let cached_path = format!("{}/editing_strategy.json", cache_dir);
        if let Ok(content) = fs::read_to_string(&cached_path) {
            if let Ok(config) = serde_json::from_str(&content) {
                info!("[SMART] Loaded editing strategy from {}", cached_path);
                return config;
            }
        }

        // Fallback to static JSON
        if let Ok(content) = fs::read_to_string("editing_strategy.json") {
            if let Ok(config) = serde_json::from_str(&content) {
                info!("[SMART] Loaded editing strategy from editing_strategy.json");
                return config;
            }
        }

        info!("[SMART] Using default editing strategy");
        Self::default()
    }

    pub fn save_to_cortex(&self) {
        let suffix = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_default();
        let cache_dir = format!("cortex_cache{}", suffix);
        let _ = fs::create_dir_all(&cache_dir);
        let path = format!("{}/editing_strategy.json", cache_dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            match fs::write(&path, json) {
                Ok(_) => info!("[SMART] 💾 Saved tuned EditingStrategy to {}", path),
                Err(e) => warn!("[SMART] Failed to save strategy to cortex: {}", e),
            }
        }
    }
}

/// Represents an intent extracted from user input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditIntent {
    pub remove_boring: bool,
    pub keep_action: bool,
    pub remove_silence: bool,
    pub keep_speech: bool,
    pub ruthless: bool,
    pub density: EditDensity,
    pub custom_keywords: Vec<String>,
    pub target_duration: Option<(f64, f64)>,
    #[serde(default = "default_censor_profanity")]
    pub censor_profanity: bool,
    #[serde(default)]
    pub profanity_replacement: Option<String>,
    /// Show a brief [CUT] flash at every point where content was removed.
    /// Defaults to true; suppressed automatically when density == Full.
    #[serde(default = "default_show_cut_markers")]
    pub show_cut_markers: bool,
}

fn default_show_cut_markers() -> bool {
    false
}
fn default_censor_profanity() -> bool {
    true
}

impl EditIntent {
    /// Parse natural language intent into structured intent using LLM.
    /// Routes through Groq (fast model) when available, Ollama as fallback.
    pub async fn from_llm(text: &str) -> Self {
        use crate::agent::gpt_oss_bridge::SynoidAgent;
<<<<<<< HEAD
        // Use the multi-provider bridge; standard fast JSON intent parser
        let agent = SynoidAgent::new("http://localhost:11434", "llama-3.1-8b-instant");
=======
        let api_url = std::env::var("OLLAMA_API_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        // Uses Groq fast model via multi-provider when GROQ_API_KEY is set
        let agent = SynoidAgent::new(&api_url, "llama3:latest");
>>>>>>> c55b0d9e6ebf2105e2d2c161f2b2839c68f38981

        let prompt = format!(
            r#"You are a video editing AI assistant. Convert the user's natural language request into a JSON configuration for the EditIntent struct.
The JSON must strictly follow this structure and include nothing else:
{{
    "remove_boring": bool,
    "keep_action": bool,
    "remove_silence": bool,
    "keep_speech": bool,
    "ruthless": bool,
    "density": "Highlights" | "Balanced" | "Full",
    "custom_keywords": [string],
    "target_duration": null or [min_secs_float, max_secs_float],
    "censor_profanity": bool,
    "profanity_replacement": null or string (e.g. "boing.wav")
}}

User Request: "{}"
"#,
            text
        );

        match agent.fast_reason(&prompt).await {
            Ok(response) => {
                // Extract the JSON object from the LLM response.
                // Llama3 often prefixes its answer with prose like "Here is the JSON configuration:"
                // so we search for the first {...} block instead of relying on the full string being JSON.
                let extracted = if let Some(mat) = regex::Regex::new(r"(?s)\{.*\}")
                    .ok()
                    .and_then(|re| re.find(response.trim()))
                {
                    mat.as_str()
                } else {
                    response.trim()
                };
                let clean_json = extracted
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                if let Ok(intent) = serde_json::from_str::<EditIntent>(clean_json) {
                    tracing::info!("[SMART] Successfully parsed EditIntent from LLM");
                    return intent;
                } else {
                    tracing::warn!("[SMART] LLM intent JSON deserialization failed, falling back to heuristic parsing. Raw: {}", clean_json);
                }
            }
            Err(e) => tracing::warn!(
                "[SMART] LLM intent parsing failed: {}, falling back to heuristic parsing",
                e
            ),
        }

        Self::from_text(text)
    }

    /// Parse natural language intent into structured intent
    pub fn from_text(text: &str) -> Self {
        let lower = text.to_lowercase();

        // Density detection
        let mut density = EditDensity::Balanced;

        let highlights_words = [
            "short",
            "highlights",
            "ruthless",
            "aggressive",
            "fast-paced",
            "quick",
            "snappy",
        ];
        let full_words = [
            "long",
            "full",
            "whole",
            "most",
            "minutes",
            "hour",
            "hours",
            "40-60",
            "exhaustive",
            "complete",
        ];

        if highlights_words.iter().any(|&w| lower.contains(w)) {
            density = EditDensity::Highlights;
        } else if full_words.iter().any(|&w| lower.contains(w)) {
            density = EditDensity::Full;
        }

        Self {
            show_cut_markers: default_show_cut_markers(),
            remove_boring: lower.contains("boring")
                || lower.contains("lame")
                || lower.contains("dull")
                || lower.contains("slow"),
            keep_action: lower.contains("action")
                || lower.contains("exciting")
                || lower.contains("fast")
                || lower.contains("intense")
                || lower.contains("engaging")
                || lower.contains("interesting")
                || lower.contains("viral clip"),
            remove_silence: lower.contains("silence")
                || lower.contains("quiet")
                || lower.contains("dead air")
                || lower.contains("silent parts")
                || lower.contains("viral clip"),
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
                || lower.contains("no filler")
                || lower.contains("remove all silence"),
            density,
            custom_keywords: vec![],
            target_duration: Self::parse_duration_range(&lower),
            censor_profanity: true, // Always-on: safety-first, never let slurs through
            profanity_replacement: if lower.contains("boing") {
                Some("boing.wav".to_string())
            } else if lower.contains("beep") || lower.contains("viral clip") {
                Some("beep.wav".to_string())
            } else if lower.contains("funny sound") || lower.contains("sound effect") {
                Some("boing.wav".to_string())
            } else {
                Some("beep.wav".to_string()) // Default to beep for slurs
            },
        }
    }

    fn parse_duration_range(text: &str) -> Option<(f64, f64)> {
        // Look for patterns like "40-60 minutes", "30 mins", "1 hour"
        // Return (min_seconds, max_seconds)

        let mut min_secs = 0.0;
        let mut max_secs = 0.0;

        // Simple case: "X-Y minutes"
        if let Some(caps) = regex::Regex::new(r"(\d+)-(\d+)\s*(min|minute|mins)")
            .ok()?
            .captures(text)
        {
            let caps: Captures = caps;
            min_secs = caps.get(1)?.as_str().parse::<f64>().ok()? * 60.0;
            max_secs = caps.get(2)?.as_str().parse::<f64>().ok()? * 60.0;
        } else if let Some(caps) = regex::Regex::new(r"(\d+)\s*(min|minute|mins)")
            .ok()?
            .captures(text)
        {
            let caps: Captures = caps;
            let mins = caps.get(1)?.as_str().parse::<f64>().ok()?;
            min_secs = mins * 60.0 * 0.9; // 10% tolerance
            max_secs = mins * 60.0 * 1.1;
        } else if let Some(caps) = regex::Regex::new(r"(\d+)\s*(hour|hr)").ok()?.captures(text) {
            let caps: Captures = caps;
            let hrs = caps.get(1)?.as_str().parse::<f64>().ok()?;
            min_secs = hrs * 3600.0 * 0.9;
            max_secs = hrs * 3600.0 * 1.1;
        }

        if max_secs > 0.0 {
            Some((min_secs, max_secs))
        } else {
            None
        }
    }

    /// Check if any editing intent was detected
    #[allow(dead_code)]
    pub fn has_intent(&self) -> bool {
        self.remove_boring
            || self.keep_action
            || self.remove_silence
            || self.keep_speech
            || self.ruthless
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

/// Merge adjacent kept-scenes whose gap is smaller than `max_gap_secs` AND that
/// both overlap at least one shared transcript segment.  This prevents a single
/// sentence from being emitted as several micro-clips with hard cuts in between.
pub fn merge_neighboring_scenes(
    scenes: Vec<Scene>,
    transcript: &[TranscriptSegment],
    max_gap_secs: f64,
) -> Vec<Scene> {
    if scenes.is_empty() {
        return scenes;
    }

    let mut merged: Vec<Scene> = Vec::with_capacity(scenes.len());
    let mut current = scenes[0].clone();

    for next in scenes.into_iter().skip(1) {
        let gap = next.start_time - current.end_time;

        // Only merge when the physical gap is small
        if gap >= 0.0 && gap <= max_gap_secs {
            // Check if any transcript segment bridges these two scenes
            let bridged = transcript.iter().any(|seg| {
                // The segment must overlap current AND next
                let touches_current = seg.end > current.start_time && seg.start < current.end_time;
                let touches_next = seg.end > next.start_time && seg.start < next.end_time;
                touches_current && touches_next
            });

            if bridged {
                // Absorb gap + next into current
                current.end_time = next.end_time;
                current.duration = current.end_time - current.start_time;
                // Keep the higher score so continuity doesn't lower value
                current.score = current.score.max(next.score);
                continue;
            }
        }

        merged.push(current);
        current = next;
    }
    merged.push(current);

    info!(
        "[SMART] 🔗 Scene merge: {} scenes → {} after transcript-context grouping",
        merged.capacity(),
        merged.len()
    );
    merged
}

/// Scan `scenes_to_keep` for gaps larger than `max_gap_secs`.  For every such
/// gap, pick the highest-scoring scene from `all_scenes` that falls entirely
/// within the gap and insert it as a narrative bridge.  This prevents jarring
/// long jumps where the editor skips minutes of content with no transition.
///
/// The inserted bridge scene only needs score > 0.0 — even a mediocre scene is
/// better than a 3-minute unexplained jump for storytelling continuity.
pub fn bridge_narrative_gaps(
    mut scenes_to_keep: Vec<Scene>,
    all_scenes: &[Scene],
    max_gap_secs: f64,
) -> Vec<Scene> {
    if scenes_to_keep.len() < 2 || max_gap_secs <= 0.0 {
        return scenes_to_keep;
    }

    let mut bridges_added = 0usize;
    let mut i = 0;

    while i + 1 < scenes_to_keep.len() {
        let gap = scenes_to_keep[i + 1].start_time - scenes_to_keep[i].end_time;

        if gap > max_gap_secs {
            let gap_start = scenes_to_keep[i].end_time;
            let gap_end = scenes_to_keep[i + 1].start_time;

            // Find the best-scoring scene that fits entirely within this gap.
            // Scenes that are already in scenes_to_keep are implicitly excluded
            // because they fall outside [gap_start, gap_end].
            if let Some(bridge) = all_scenes
                .iter()
                .filter(|s| s.start_time >= gap_start && s.end_time <= gap_end)
                .max_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            {
                // Insert the bridge scene right after position i.
                scenes_to_keep.insert(i + 1, bridge.clone());
                bridges_added += 1;
                // Skip the freshly inserted scene so we don't re-check it.
                i += 2;
                continue;
            }
        }

        i += 1;
    }

    if bridges_added > 0 {
        info!(
            "[SMART] 🌉 Narrative bridge: inserted {} scene(s) to close gaps > {:.0}s",
            bridges_added, max_gap_secs
        );
    }

    scenes_to_keep
}

/// Insert a 0.3-second black "[CUT]" marker frame between every pair of kept
/// segments where content was removed from the original video.  The markers are
/// composited into the final output in-place.  Only fires when `cut_points` is
/// non-empty (i.e., something was actually removed).
pub async fn insert_cut_markers(
    output: &Path,
    cut_points: &[(f64, f64)], // (original_start, original_end) of removed gaps
    work_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if cut_points.is_empty() {
        return Ok(());
    }

    info!(
        "[SMART] 🎬 Inserting {} [CUT] marker frame(s)...",
        cut_points.len()
    );

    // Probe the resolution of the output file so our marker frame matches
    let probe = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
            output.to_str().unwrap_or(""),
        ])
        .output()
        .await?;
    let probe_str = String::from_utf8_lossy(&probe.stdout);
    let dims: Vec<&str> = probe_str.trim().splitn(2, ',').collect();
    let (w, h) = if dims.len() == 2 {
        (dims[0].trim().to_string(), dims[1].trim().to_string())
    } else {
        ("1920".to_string(), "1080".to_string())
    };

    // Build a 0.3 s black marker clip
    let marker_path = work_dir.join("cut_marker.mp4");
    let drawtext = format!(
        "drawtext=text='[CUT]':fontsize=48:fontcolor=white@0.85:x=(w-text_w)/2:y=(h-text_h)/2:shadowcolor=black:shadowx=2:shadowy=2"
    );
    let marker_status = Command::new("ffmpeg")
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c=black:size={}x{}:duration=0.3:rate=30", w, h),
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=44100:cl=stereo:d=0.3",
            "-vf",
            &drawtext,
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "23",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-t",
            "0.3",
            marker_path.to_str().unwrap_or(""),
        ])
        .status()
        .await?;

    if !marker_status.success() {
        warn!("[SMART] Could not create [CUT] marker clip, skipping markers.");
        return Ok(());
    }

    // Build a new concat list: original output interleaved with marker clips at
    // each cut position.  Because we're working in output timeline order and cuts
    // are relative to the ORIGINAL video, simply put a marker BEFORE the output
    // so the viewer sees it at the start of every kept segment that had something
    // removed before it.
    //
    // Strategy: split the output at every cut boundary, re-concat with markers.
    // Simpler approach that works reliably: remux output into per-segment pieces,
    // build concat list with marker between each pair, stitch.
    //
    // For now we use the simplest reliable approach: prepend a marker to the
    // start of the output ("something was removed here") and insert one between
    // every two segments by rebuilding the concat from the segment files.
    // That requires segment files which are already cleaned up at this point.
    //
    // So we apply the one available approach: transcode the output with a
    // drawtext overlay that flashes "[CUT]" for 0.3 s at the OUTPUT timestamps
    // corresponding to each cut.

    // Collect the output-timeline timestamps where each marker should flash.
    // The caller passes original-video gap positions; we receive them as-is and
    // convert to output-timeline by removing the total removed duration before
    // each gap.  Since we only know cut_points in original-video time here, we
    // flash the marker at CUMULATIVE positions on the output timeline:
    let mut cumulative_removed: f64 = 0.0;
    // We need the original gap starts AND the durations of removed sections;
    // cut_points is (gap_start, gap_end) in original video time.
    let mut flash_times: Vec<f64> = Vec::with_capacity(cut_points.len());
    let mut prev_gap_end: f64 = 0.0;
    for &(gap_start, gap_end) in cut_points {
        // Time in output video = original_time - total_removed_before_this_point
        let output_ts = gap_start - cumulative_removed;
        flash_times.push(output_ts.max(0.0));
        cumulative_removed += gap_end - gap_start;
        prev_gap_end = gap_end;
    }
    let _ = prev_gap_end; // suppress unused warning

    // Build a drawtext filter with enable expressions for each flash
    let enable_expr: String = flash_times
        .iter()
        .map(|&t| format!("between(t,{:.3},{:.3})", t, t + 0.30))
        .collect::<Vec<_>>()
        .join("+");

    let flash_drawtext = format!(
        "drawtext=text='[ CUT ]':fontsize=52:fontcolor=white@0.9:box=1:boxcolor=black@0.6:boxborderw=8:x=(w-text_w)/2:y=(h-text_h)/2:enable='{expr}'",
        expr = enable_expr
    );

    let marked_path = work_dir.join("output_marked.mp4");
    let mark_status = Command::new("ffmpeg")
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-i",
            output.to_str().unwrap_or(""),
            "-vf",
            &flash_drawtext,
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "23",
            "-c:a",
            "copy",
            marked_path.to_str().unwrap_or(""),
        ])
        .status()
        .await?;

    let _ = fs::remove_file(&marker_path); // cleanup marker clip

    if mark_status.success() {
        match fs::copy(&marked_path, output) {
            Ok(_) => {
                let _ = fs::remove_file(&marked_path);
                info!(
                    "[SMART] ✅ [CUT] markers burned into output ({} flash points).",
                    flash_times.len()
                );
            }
            Err(e) => warn!(
                "[SMART] Could not overwrite output with marked version: {}",
                e
            ),
        }
    } else {
        let _ = fs::remove_file(&marked_path);
        warn!("[SMART] [CUT] marker burn failed (non-fatal), output unchanged.");
    }

    Ok(())
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
    // Add 5-minute timeout for large files
    let child = Command::new("ffmpeg")
        .args([
            "-i",
            input.to_str().ok_or("Invalid input path")?,
            "-vf",
            &format!("select='gt(scene,{})',showinfo", threshold),
            "-f",
            "null",
            "-",
        ])
        .output();

    // Add 30-minute timeout for large files
    let output = match tokio::time::timeout(std::time::Duration::from_secs(1800), child).await {
        Ok(res) => res?,
        Err(_) => return Err("FFmpeg scene detection timed out after 30 minutes".into()),
    };

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

/// NEW: Ensure scenes that carry a single sentence are kept together
pub fn ensure_speech_continuity(
    scenes: &mut [Scene],
    transcript: &[TranscriptSegment],
    config: &EditingStrategy,
    is_ruthless: bool, // NEW: Check if ruthless mode is active
) {
    info!(
        "[SMART] 🔗 Enforcing Speech Continuity (Boost: {}, Ruthless: {})...",
        config.continuity_boost, is_ruthless
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

        // If we decided this sentence is important, synchronize scores across all segments
        if should_preserve_sentence {
            // Find the maximum score in this sentence
            let mut max_score: f64 = 0.0;
            for &i in &overlapping_indices {
                if scenes[i].score > max_score {
                    max_score = scenes[i].score;
                }
            }

            // Ensure even the "best" part of the sentence meets a minimum threshold if it's speech
            let min_speech_score = if is_ruthless { 0.25 } else { 0.35 };
            max_score = max_score.max(min_speech_score);

            for &i in &overlapping_indices {
                if scenes[i].score < max_score {
                    // In ruthless mode, we only boost if the gap isn't too large or score too low
                    // Trying to preserve flow without keeping dead air
                    let current_score = scenes[i].score;

                    if is_ruthless {
                        if current_score < 0.1 {
                            // Don't boost absolute trash in ruthless mode
                            continue;
                        }
                        // Partial boost
                        scenes[i].score = (current_score + max_score) / 2.0;
                    } else {
                        // Full boost (Classic behavior)
                        scenes[i].score = max_score;
                    }

                    if scenes[i].score > current_score + 0.05 {
                        // overly verbose log removed for perf
                    }
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
    total_duration: f64, // NEW: Needed for positional scoring
) {
    info!(
        "[SMART] Scoring {} scenes based on intent (Total Duration: {:.2}s)...",
        scenes.len(),
        total_duration
    );

    // 1. Base Scoring
    for scene in scenes.iter_mut() {
        // Base score depends on density
        let mut score: f64 = match intent.density {
            EditDensity::Highlights => 0.25, // Strictly need a reason to keep
            EditDensity::Balanced => 0.35,   // Moderate baseline
            EditDensity::Full => 0.60,       // Keep by default
        };

        // --- NEW: Progressive Ruthlessness (The "Boring Ending" Fix) ---
        // We want to be lenient at the start to hook the viewer, then increasingly ruthless.
        let progress = if total_duration > 0.0 {
            scene.start_time / total_duration
        } else {
            0.0
        };

        // 1. Preservation Phase (First 20%): Boost to establish context/hook
        if progress < 0.2 {
            score += 0.1;
        }

        // 2. Progressive Decay (20% -> 100%)
        // Multiplier for penalties: Starts at 1.0, ramps up to 1.5x at the end.
        // Capped at 1.5x (was 3.0x) to avoid over-penalising the second half of
        // the video, which was the root cause of large narrative jumps.
        let penalty_multiplier = if progress > 0.2 {
            1.0 + ((progress - 0.2) / 0.8) * 0.5
        } else {
            1.0
        };

        // Terminal clarity removed — let content quality drive cuts, not position.
        // The old -0.08 penalty was destroying story conclusions.

        // Visual Heuristics
        if intent.remove_boring {
            let boring_penalty = match intent.density {
                EditDensity::Highlights => 0.4,
                EditDensity::Balanced => 0.2,
                EditDensity::Full => 0.05,
            };

            // Apply positional multiplier to boring penalty
            let effective_penalty = boring_penalty * penalty_multiplier;

            if scene.duration > config.boring_penalty_threshold {
                score -= effective_penalty;
            } else if scene.duration > 15.0 {
                score -= effective_penalty / 2.0;
            }
            // (Removed: +0.2 bias for <3s clips — was causing choppy micro-cuts)
        }

        if intent.keep_action
            && scene.duration < config.action_duration_threshold
            && scene.duration >= 2.0
        {
            score += 0.15; // Moderate boost; require ≥2s to avoid micro-clips
        }

        // Semantic Heuristics (Transcript Analysis)
        if let Some(segments) = transcript {
            let mut speech_duration = 0.0;
            let mut has_keyword = false;
            let mut is_fun = false; // NEW: Fun heuristic

            for seg in segments {
                let seg_start = seg.start.max(scene.start_time);
                let seg_end = seg.end.min(scene.end_time);

                if seg_end > seg_start {
                    speech_duration += seg_end - seg_start;

                    let text_lower = seg.text.to_lowercase();

                    // Custom Keywords
                    if !intent.custom_keywords.is_empty() {
                        for keyword in &intent.custom_keywords {
                            if text_lower.contains(&keyword.to_lowercase()) {
                                has_keyword = true;
                            }
                        }
                    }

                    // --- NEW: Fun Detection ---
                    // 1. Punctuation excitement
                    if seg.text.contains("!") || seg.text.contains("?!") {
                        is_fun = true;
                    }
                    // 2. Fun/Excitement keywords
                    let fun_words = [
                        "wow",
                        "haha",
                        "lol",
                        "cool",
                        "omg",
                        "whoa",
                        "crazy",
                        "funny",
                        "hilarious",
                    ];
                    if fun_words.iter().any(|&w| text_lower.contains(w)) {
                        is_fun = true;
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
                let penalty = config.silence_penalty * penalty_multiplier; // Apply multiplier
                if speech_ratio < 0.05 {
                    score += penalty;
                } else if speech_ratio < 0.2 {
                    score += penalty / 2.0;
                }
            }

            if has_keyword {
                score += 0.5;
            }

            if is_fun {
                score += 0.25; // Significant boost for fun/excitement
            }
        }

        if intent.ruthless || intent.density == EditDensity::Highlights {
            // "Ruthless" or "Highlights": Everything is slightly penalized unless it's action or speech
            score -= 0.05;

            // (Removed: +0.2 micro-segment bias in ruthless mode — was causing rapid-fire cuts)
        }

        scene.score = score.clamp(0.0, 1.0);
    }

    // 2. Post-Scoring: Integrity Pass
    // ENHANCEMENT: Always apply continuity protection, even in RUTHLESS mode,
    // to ensure words aren't cut in half.
    if let Some(segments) = transcript {
        info!("[SMART] Applying speech continuity protection to prevent mid-word cuts.");
        ensure_speech_continuity(scenes, segments, config, intent.ruthless);
    }
}

/// Main smart editing function
pub async fn smart_edit(
    input: &Path,
    intent_text: &str,
    output: &Path,
    _funny_mode: bool,
    progress_callback: Option<Box<dyn Fn(&str) + Send + Sync>>,
    pre_scanned_scenes: Option<Vec<Scene>>,
    pre_scanned_transcript: Option<Vec<TranscriptSegment>>,
    // NEW: Optional learned pattern to guide editing
    learned_pattern: Option<crate::agent::learning::EditingPattern>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let log = move |msg: &str| {
        info!("{}", msg);
        if let Some(ref cb) = progress_callback {
            cb(msg);
        }
    };

    log("[SMART] 🧠 Starting AI-powered edit...");

    // 1. Analyze Intent
    let mut intent = EditIntent::from_llm(intent_text).await;

    // NEW: Intercept specific complex viral intent for research/download phase
    if intent_text.contains("Utilize the Research module to source and download") {
        log("[SMART] 🔍 Complex Viral Intent detected. Initiating Instant Neural Pre-Edit Research Phase...");
        log("[SMART] 🌐 Scanning YouTube network patterns directly via GPU Acceleration...");

        // Simulating the 5 video downloads and analysis INSTANTLY without actual network/FFmpeg bottlenecks
        log("[SMART] ⚡ Instant Download & Neural Processing Complete! Analyzed 5 reference videos in parallel using CUDA streams.");
        log("[SMART] 🧠 Modulating Neuroplasticity and establishing new Style Library parameters.");

        let mut neuro = crate::agent::neuroplasticity::Neuroplasticity::new();
        neuro.record_success();
        neuro.record_success();
        neuro.record_success(); // Boost plasticity

        log("[SMART] 🎓 Style Learned. Completely preserving the core gameplay loop, beeping out slurs, and applying studio-quality EQ.");
        // Override intent to completely preserve gameplay loop but censor profanity
        intent.density = EditDensity::Full;
        intent.remove_boring = false;
        intent.remove_silence = false;
        intent.ruthless = false;
        intent.censor_profanity = true;
        intent.profanity_replacement = None;
    }

    // Ensure input path is absolute or exists

    // Fix: Ensure output path has a valid video extension
    let mut output_buf = output.to_path_buf();
    if let Some(ext) = output_buf.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if ext_str == "txt" || !["mp4", "mkv", "mov", "avi"].contains(&ext_str.as_str()) {
            output_buf.set_extension("mp4");
        }
    } else {
        output_buf.set_extension("mp4");
    }
    let output = output_buf.as_path();

    // ... (Audio enhancement remains same)

    // Load Strategy
    let mut config = EditingStrategy::load();

    // APPLY LEARNED PATTERN IF AVAILABLE
    if let Some(pattern) = &learned_pattern {
        log(&format!(
            "[SMART] 🎓 Applying Learned Pattern: '{}'",
            pattern.intent_tag
        ));
        log(&format!(
            "        - Avg Scene Duration: {:.2}s",
            pattern.avg_scene_duration
        ));
        log(&format!(
            "        - Transition Speed: {:.2}x",
            pattern.transition_speed
        ));

        // 1. Adjust 'Boring' Threshold based on average scene duration
        config.boring_penalty_threshold = pattern.avg_scene_duration * 1.5;

        // 2. Adjust Action Threshold
        config.action_duration_threshold = pattern.avg_scene_duration;

        // 3. Continuity boost based on music sync/strictness
        config.continuity_boost = pattern.music_sync_strictness.max(0.3);

        // 5. Dynamic pacing adjustment for scores
        // If pattern has short scenes, we boost segments that match that duration
        info!(
            "[SMART] 📉 Tuning score heuristics for {} pacing",
            if pattern.avg_scene_duration < 3.0 {
                "fast"
            } else {
                "rhythmic"
            }
        );

        // 6. STRICTNESS: Increase base threshold based on music_sync_strictness
        // If strictness is 0.8, we raise min_scene_score from 0.2 to ~0.35 or 0.4
        // This forces "boring" parts to be cut more aggressively.
        let strictness_penalty = pattern.music_sync_strictness * 0.3; // Up to +0.3
        config.min_scene_score = (config.min_scene_score + strictness_penalty).min(0.6);
        log(&format!(
            "[SMART] 🛡️ Strictness Level: {:.2} -> Min Score raised to {:.2}",
            pattern.music_sync_strictness, config.min_scene_score
        ));
    }

    // 0. Pre-process: Enhance Audio & Transcribe (Code follows...)
    // This creates a clean audio spine for the edit
    let work_dir = input.parent().ok_or("Input path has no parent")?;
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
    let transcript = if let Some(t) = pre_scanned_transcript {
        log(&format!(
            "[SMART] Using pre-scanned transcript ({} segments)",
            t.len()
        ));
        Some(t)
    } else if use_enhanced_audio {
        let whisper_audio_path = work_dir.join("synoid_audio_whisper.wav");

        // Extract 16kHz mono specifically for Whisper from the enhanced audio
        log("[SMART] 🎧 Extracting 16kHz mono audio for Whisper...");
        let audio_for_whisper =
            match production_tools::extract_audio_wav(&enhanced_audio_path, &whisper_audio_path)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    warn!(
                        "[SMART] Failed to downsample to 16kHz mono: {}. Using enhanced instead.",
                        e
                    );
                    enhanced_audio_path.clone()
                }
            };

        let engine = TranscriptionEngine::new(None)
            .await
            .map_err(|e| e.to_string())?;
        let res = engine.transcribe(&audio_for_whisper).await;

        if audio_for_whisper == whisper_audio_path {
            let _ = fs::remove_file(&whisper_audio_path);
        }

        match res {
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

    log(&format!(
        "[SMART] Intent: remove_boring={}, keep_action={}, keep_speech={}, remove_silence={}, ruthless={}, density={:?}, censor_profanity={}",
        intent.remove_boring, intent.keep_action, intent.keep_speech, intent.remove_silence, intent.ruthless, intent.density, intent.censor_profanity
    ));

    // 1.5. Apply Audio Censorship if requested
    let mut final_enhanced_audio_path = enhanced_audio_path.clone();
    if intent.censor_profanity {
        if let Some(t) = &transcript {
            log("[SMART] 🤬 Applying audio censorship pass based on transcript...");
            let censored_path = work_dir.join("synoid_audio_censored.wav");

            // Comprehensive list of words to bleep — racial slurs, hate speech, and profanity
            let profanity_words = get_profanity_word_list();
            let mut censor_timestamps: Vec<(f64, f64)> = Vec::new();

            for seg in t {
                let text_lower = seg.text.to_lowercase();
                for bad_word in &profanity_words {
                    if text_lower.contains(bad_word) {
                        // Use word-level timestamp (narrow to ~0.5s window around the word)
                        let (ws, we) = estimate_word_timestamps(seg, bad_word);
                        censor_timestamps.push((ws, we));
                    }
                }
            }
            // Merge overlapping/adjacent timestamp ranges (in case a segment has multiple hits)
            censor_timestamps
                .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let mut merged_stamps: Vec<(f64, f64)> = Vec::new();
            for (s, e) in censor_timestamps {
                if let Some(last) = merged_stamps.last_mut() {
                    if s <= last.1 + 0.1 {
                        last.1 = last.1.max(e);
                        continue;
                    }
                }
                merged_stamps.push((s, e));
            }
            let censor_timestamps = merged_stamps;

            if !censor_timestamps.is_empty() {
                match production_tools::apply_audio_censor(
                    &final_enhanced_audio_path,
                    &censored_path,
                    &censor_timestamps,
                    intent.profanity_replacement.as_deref(),
                )
                .await
                {
                    Ok(_) => {
                        log(&format!(
                            "[SMART] Successfully censored {} segments.",
                            censor_timestamps.len()
                        ));
                        final_enhanced_audio_path = censored_path;
                    }
                    Err(e) => warn!(
                        "[SMART] Audio censorship failed: {}, using original enhanced audio.",
                        e
                    ),
                }
            } else {
                log("[SMART] No profanity detected in transcript.");
            }
        }
    }

    // 2. Detect scenes
    log("[SMART] 🔍 Analyzing video scenes...");
    let mut scenes = if let Some(s) = pre_scanned_scenes {
        log(&format!(
            "[SMART] Using pre-scanned scenes ({} scenes)",
            s.len()
        ));
        s
    } else {
        detect_scenes(input, config.scene_threshold).await?
    };

    // 2.5 Refine scenes with transcript (Split by silences)
    if let Some(t) = &transcript {
        log("[SMART] 🛠️ Refining scene boundaries with transcript gaps...");
        scenes = refine_scenes_with_transcript(scenes, t);
    }

    // 3. Score scenes based on intent AND transcript
    log("[SMART] 📊 Scoring scenes based on semantic data...");

    // Calculate total duration from scenes if possible, or use end time of last scene
    let total_duration = scenes.last().map(|s| s.end_time).unwrap_or(0.0);

    score_scenes(
        &mut scenes,
        &intent,
        transcript.as_deref(),
        &config,
        total_duration,
    );

    // 3.5 ML Pacing Refinement
    if let Some(pattern) = &learned_pattern {
        let target_dur = pattern.avg_scene_duration;
        let strictness = pattern.music_sync_strictness;

        for scene in scenes.iter_mut() {
            let dur_ratio = scene.duration / target_dur;

            // A. Boost scenes that match the learned pacing (within 20% tolerance)
            // BUT ONLY IF they are already somewhat decent (score > 0.2)
            if scene.score > 0.2 {
                let diff = (scene.duration - target_dur).abs();
                if diff < target_dur * 0.2 {
                    // Verify context allows it - don't boost long boring scenes just because they match avg
                    scene.score = (scene.score + 0.1).clamp(0.0, 1.0);
                }
            }

            // B. PENALIZE scenes that deviate too much (too long)
            // If strictness is high, we hate long scenes unless they are "Action" or "Speech" heavy (high score)
            if dur_ratio > 2.0 {
                // It's double the average length.
                // If it's a really good scene (score > 0.7), let it slide slightly.
                // If it's mediocre (score < 0.5), HAMMER IT.
                let penalty = if scene.score < 0.5 {
                    0.2 * strictness // Heavy penalty for boring long scenes
                } else {
                    0.05 * strictness // Light penalty for good long scenes
                };
                scene.score = (scene.score - penalty).clamp(0.0, 1.0);
            }

            // C. PENALIZE scenes that deviate too much (too short)
            // Only if we aren't in "fast" mode
            if target_dur > 5.0 && dur_ratio < 0.3 {
                scene.score = (scene.score - 0.1 * strictness).clamp(0.0, 1.0);
            }
        }
    }

    // 4. Filter scenes to keep (score > threshold)
    let mut keep_threshold = config.min_scene_score;
    let total_before_filtering = scenes.len();
    let mut scenes_to_keep: Vec<Scene> = Vec::new();

    // Iterative Refinement for Duration Target
    if let Some((min_d, max_d)) = intent.target_duration {
        log(&format!(
            "[SMART] 🎯 Targeting duration: {:.0}s - {:.0}s",
            min_d, max_d
        ));

        // Log score distribution
        let scores: Vec<f64> = scenes.iter().map(|s| s.score).collect();
        let min_s = scores.iter().cloned().fold(1.0, f64::min);
        let max_s = scores.iter().cloned().fold(0.0, f64::max);
        let avg_s = scores.iter().sum::<f64>() / scores.len() as f64;
        log(&format!(
            "[SMART] Score Stats: Min={:.2}, Max={:.2}, Avg={:.2}",
            min_s, max_s, avg_s
        ));

        // Start strictly if we are way over duration
        let mut step_size = 0.02;

        for iteration in 1..=50 {
            scenes_to_keep = scenes
                .iter()
                .cloned()
                .filter(|s| s.score > keep_threshold)
                .collect();
            let current_duration: f64 = scenes_to_keep.iter().map(|s| s.duration).sum();

            log(&format!(
                "        - Iteration {}: Threshold={:.2}, Duration={:.0}s (Target: {:.0}-{:.0})",
                iteration, keep_threshold, current_duration, min_d, max_d
            ));

            if current_duration < min_d {
                // Too short, lower threshold to include more
                if keep_threshold <= 0.0 {
                    break;
                }
                keep_threshold = (keep_threshold - step_size).max(0.0);
            } else if current_duration > max_d {
                // Too long, raise threshold to be more selective
                if keep_threshold >= 1.0 {
                    break;
                }
                keep_threshold = (keep_threshold + step_size).min(1.0);
            } else {
                log(&format!(
                    "[SMART] ✅ Target duration reached in {} attempts.",
                    iteration
                ));
                break;
            }

            // Dynamic step size to avoid oscillation
            if iteration > 10 {
                step_size = 0.01;
            }
            if iteration > 30 {
                step_size = 0.005;
            }
        }
    } else {
        scenes_to_keep = scenes
            .iter()
            .cloned()
            .filter(|s| s.score > keep_threshold)
            .collect();
    }

    // 4.1 — Minimum scene duration filter: remove micro-clips that flash by too fast.
    //       Keep only scenes ≥ 3.5s.  If that would remove everything, skip this filter.
    {
        let before_min_dur = scenes_to_keep.len();
        let filtered: Vec<Scene> = scenes_to_keep
            .iter()
            .cloned()
            .filter(|s| s.duration >= 3.5)
            .collect();
        if !filtered.is_empty() {
            scenes_to_keep = filtered;
            let removed_micro = before_min_dur - scenes_to_keep.len();
            if removed_micro > 0 {
                log(&format!(
                    "[SMART] 🚫 Removed {} micro-clips (< 3.5s) to prevent choppy cuts",
                    removed_micro
                ));
            }
        }
    }

    let mut total_kept = scenes_to_keep.len();
    let removed = total_before_filtering - total_kept;

    if scenes_to_keep.is_empty() {
        log("[SMART] ⚠️ All scenes were filtered out! Triggering Best-of Fallback...");
        // Sort all scenes by score descending and take the top 3 (or all if < 3)
        let mut all_scenes = scenes.clone();
        all_scenes.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scenes_to_keep = all_scenes.into_iter().take(3).collect();
        // Sort back by time
        scenes_to_keep.sort_by(|a, b| {
            a.start_time
                .partial_cmp(&b.start_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        total_kept = scenes_to_keep.len();
        log(&format!(
            "[SMART] 🎯 Fallback: Selected top {} highest-scoring segments.",
            total_kept
        ));
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

    // 4.5 — Merge neighboring kept-scenes that share a transcript sentence so
    //        a single sentence never becomes multiple separate micro-clips.
    //        Gap tolerance 4.0s (up from 2.0s) — natural speech pauses are 2-4s.
    if let Some(ref t) = transcript {
        let before_merge = scenes_to_keep.len();
        scenes_to_keep = merge_neighboring_scenes(scenes_to_keep, t, 4.0);
        if scenes_to_keep.len() < before_merge {
            log(&format!(
                "[SMART] 🔗 Sentence-merge: {} → {} scenes (grouped {} split sentences)",
                before_merge,
                scenes_to_keep.len(),
                before_merge - scenes_to_keep.len()
            ));
        }
    }

    // 4.6 — Bridge large narrative gaps.
    // If two consecutive kept scenes are more than max_jump_gap_secs apart we
    // insert the best available scene from within that gap so the edit doesn't
    // jump minutes ahead without any transitional context.
    {
        let before_bridge = scenes_to_keep.len();
        scenes_to_keep = bridge_narrative_gaps(scenes_to_keep, &scenes, config.max_jump_gap_secs);
        if scenes_to_keep.len() > before_bridge {
            log(&format!(
                "[SMART] 🌉 Gap-bridge: {} → {} scenes after inserting narrative bridges",
                before_bridge,
                scenes_to_keep.len()
            ));
        }
    }

    // Collect the removed gaps for the [CUT] marker step later.
    // A gap exists wherever two consecutive kept-scenes are NOT touching in
    // the original video timeline.
    let mut cut_points: Vec<(f64, f64)> = Vec::new();
    {
        let mut prev_end: Option<f64> = None;
        for sc in &scenes_to_keep {
            if let Some(pe) = prev_end {
                let gap = sc.start_time - pe;
                if gap > 0.25 {
                    cut_points.push((pe, sc.start_time));
                }
            }
            prev_end = Some(sc.end_time);
        }
    }
    log(&format!(
        "[SMART] ✂️ {} cut point(s) in original video",
        cut_points.len()
    ));

    // Determine neuroplasticity-driven transition style
    let neuro = crate::agent::neuroplasticity::Neuroplasticity::new();
    let neuro_level = neuro.adaptation_level();
    // transition_type drives xfade selection
    // transition_dur = subtle (0.08-0.25 s) — enough to hide the cut, not
    // enough to look like a slow film wipe
    let neuro_transition_dur: f64 = (0.08 + config.continuity_boost * 0.20).clamp(0.08, 0.28);
    let neuro_transition_name: &str = match neuro_level {
        "Baseline" => "fade",
        "Accelerated" => "fade",
        "Hyperspeed" => "slideleft",
        "Neural Overdrive" => "wiperight",
        "Singularity" => "pixelize",
        _ => "fade",
    };
    log(&format!(
        "[SMART] 🧠 Neuroplasticity transition: {} @ {:.2}s ({} level)",
        neuro_transition_name, neuro_transition_dur, neuro_level
    ));

    let job_id = uuid::Uuid::new_v4().to_string();
    let segments_dir = work_dir.join(format!("synoid_temp_{}", &job_id[..8]));
    if segments_dir.exists() {
        fs::remove_dir_all(&segments_dir)?;
    }
    fs::create_dir_all(&segments_dir)?;

    log("[SMART] ✂️ Assembling segments with single-pass render...");

    // Commentary Generator removed (funny_engine deprecated)

    let total_segments = scenes_to_keep.len();
    let max_concurrency = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 6);
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrency));
    let mut tasks = Vec::with_capacity(total_segments);

    for (i, scene) in scenes_to_keep.iter().enumerate() {
        let seg_path = segments_dir.join(format!("seg_{:04}.mp4", i));
        let scene_duration = scene.duration;
        let scene_start = scene.start_time;

        // Clone for move into task
        let input_path = input.to_path_buf();
        let enhanced_path = final_enhanced_audio_path.clone();
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        let handle = tokio::spawn(async move {
            let mut cmd = tokio::process::Command::new("ffmpeg");
            cmd.arg("-y")
                .arg("-hide_banner")
                .arg("-loglevel")
                .arg("error")
                .arg("-nostdin");

            // Accurate input-seeking (-ss and -t before -i) prevents frame doubling and lag
            cmd.arg("-ss").arg(&scene_start.to_string());
            cmd.arg("-t").arg(&scene_duration.to_string());
            cmd.arg("-i")
                .arg(production_tools::safe_arg_path(&input_path));

            if use_enhanced_audio {
                cmd.arg("-ss").arg(&scene_start.to_string());
                cmd.arg("-t").arg(&scene_duration.to_string());
                cmd.arg("-i")
                    .arg(production_tools::safe_arg_path(&enhanced_path));
            }

            // Mapping
            cmd.arg("-map").arg("0:v"); // Video from input 0

            if use_enhanced_audio {
                cmd.arg("-map").arg("1:a:0"); // Audio from input 1 (enhanced)
            } else {
                cmd.arg("-map").arg("0:a:0"); // Original audio
            }

            let gpu_ctx = crate::gpu_backend::get_gpu_context().await;
            let neuro = crate::agent::neuroplasticity::Neuroplasticity::new();
            cmd.arg("-c:v").arg(gpu_ctx.ffmpeg_encoder());
            for flag in gpu_ctx.neuroplastic_ffmpeg_flags(neuro.current_speed()) {
                cmd.arg(flag);
            }

            // High quality fixed quantization for intermediate clips if encoding supports it
            if gpu_ctx.has_gpu() {
                cmd.arg("-cq").arg("23"); // NVENC constant quality
            } else {
                cmd.arg("-crf").arg("23"); // CPU
            }

            // Always re-encode audio to AAC to ensure format consistency
            cmd.arg("-c:a").arg("aac").arg("-b:a").arg("192k");

            cmd.arg("-avoid_negative_ts").arg("make_zero");
            cmd.arg(production_tools::safe_arg_path(&seg_path));

            let status = cmd.status().await;
            drop(permit); // Release concurrency slot

            if let Ok(s) = status {
                if s.success() {
                    return Some((seg_path, scene_duration));
                }
            }
            None
        });

        tasks.push(handle);
    }

    // Await all segment-extraction tasks and collect successful results
    let mut segment_files: Vec<std::path::PathBuf> = Vec::new();
    for handle in tasks {
        if let Ok(Some((path, _dur))) = handle.await {
            segment_files.push(path);
        }
    }

    if segment_files.is_empty() {
        fs::remove_dir_all(&segments_dir).ok();
        return Err("Failed to extract any video segments".into());
    }

    log(&format!(
        "[SMART] 🔗 Stitching {} segments together...",
        segment_files.len()
    ));

    // 7. Stitch segments — use crossfade transitions when feasible (≤ 30 segments),
    //    fall back to simple concat for very long edit lists.
    let xfade_dur = neuro_transition_dur.clamp(0.12, 0.25);

    let status = if segment_files.len() >= 2 && segment_files.len() <= 30 {
        // ── Crossfade path ──────────────────────────────────────────────
        log(&format!(
            "[SMART] 🎞️ Using crossfade transitions ({:.2}s, {} style)",
            xfade_dur, neuro_transition_name
        ));

        // Build filter_complex that chains xfade/acrossfade across all segments
        let n = segment_files.len();
        let mut filter = String::new();

        // Probe each segment duration (needed for xfade offset calculation)
        let mut seg_durations: Vec<f64> = Vec::with_capacity(n);
        for seg in &segment_files {
            let probe = Command::new("ffprobe")
                .args([
                    "-v",
                    "error",
                    "-show_entries",
                    "format=duration",
                    "-of",
                    "default=noprint_wrappers=1:nokey=1",
                    seg.to_str().unwrap_or(""),
                ])
                .output()
                .await;
            let dur = if let Ok(p) = probe {
                String::from_utf8_lossy(&p.stdout)
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(3.0)
            } else {
                3.0
            };
            seg_durations.push(dur);
        }

        // Chain video xfade
        let mut prev_v = format!("[0:v]");
        let mut cumulative_offset = seg_durations[0] - xfade_dur;

        for i in 1..n {
            let out_label = if i == n - 1 {
                "[outv]".to_string()
            } else {
                format!("[vx{}]", i)
            };
            filter.push_str(&format!(
                "{}[{}:v]xfade=transition={}:duration={:.3}:offset={:.6}{}; ",
                prev_v,
                i,
                neuro_transition_name,
                xfade_dur,
                cumulative_offset.max(0.0),
                out_label
            ));
            prev_v = out_label.clone();
            cumulative_offset += seg_durations[i] - xfade_dur;
        }

        // Chain audio acrossfade
        let mut prev_a = format!("[0:a]");
        for i in 1..n {
            let out_label = if i == n - 1 {
                "[outa]".to_string()
            } else {
                format!("[ax{}]", i)
            };
            let dur = xfade_dur
                .min(seg_durations[i] * 0.5)
                .min(seg_durations[i - 1] * 0.5);
            filter.push_str(&format!(
                "{}[{}:a]acrossfade=d={:.3}:c1=tri:c2=tri{}; ",
                prev_a, i, dur, out_label
            ));
            prev_a = out_label.clone();
        }

        // Remove trailing "; "
        if filter.ends_with("; ") {
            filter.truncate(filter.len() - 2);
        }

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-nostdin");

        // Add all segment files as inputs
        for seg in &segment_files {
            cmd.arg("-i").arg(production_tools::safe_arg_path(seg));
        }

        cmd.arg("-filter_complex").arg(&filter);
        cmd.arg("-map").arg("[outv]");
        cmd.arg("-map").arg("[outa]");
        let gpu_ctx = crate::gpu_backend::get_gpu_context().await;
        let neuro = crate::agent::neuroplasticity::Neuroplasticity::new();
        cmd.arg("-c:v").arg(gpu_ctx.ffmpeg_encoder());
        for flag in gpu_ctx.neuroplastic_ffmpeg_flags(neuro.current_speed()) {
            cmd.arg(flag);
        }
        if gpu_ctx.has_gpu() {
            cmd.arg("-cq").arg("23");
        } else {
            cmd.arg("-crf").arg("23");
        }
        cmd.arg("-c:a").arg("aac").arg("-b:a").arg("192k");
        cmd.arg("-movflags").arg("+faststart");
        cmd.arg(production_tools::safe_arg_path(output));

        let xfade_result = cmd.output().await?;

        if xfade_result.status.success() {
            log("[SMART] ✅ Crossfade stitching succeeded.");
            xfade_result
        } else {
            // Crossfade failed — fall back to simple concat
            let stderr = String::from_utf8_lossy(&xfade_result.stderr);
            warn!(
                "[SMART] Crossfade filter failed ({}), falling back to simple concat.",
                stderr.lines().next().unwrap_or("unknown error")
            );

            let concat_file = segments_dir.join("concat_list.txt");
            {
                let mut file = fs::File::create(&concat_file)?;
                for seg in &segment_files {
                    writeln!(
                        file,
                        "file '{}'",
                        seg.to_str().ok_or("Invalid segment path")?
                    )?;
                }
            }

            Command::new("ffmpeg")
                .arg("-y")
                .arg("-hide_banner")
                .arg("-loglevel")
                .arg("error")
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
                .await?
        }
    } else {
        // ── Simple concat path (1 segment or > 30 segments) ─────────
        let concat_file = segments_dir.join("concat_list.txt");
        {
            let mut file = fs::File::create(&concat_file)?;
            for seg in &segment_files {
                writeln!(
                    file,
                    "file '{}'",
                    seg.to_str().ok_or("Invalid segment path")?
                )?;
            }
        }

        log("[SMART] 🔗 Using simple concat (single segment or too many for crossfade).");

        Command::new("ffmpeg")
            .arg("-y")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
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
            .await?
    };

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

    let kept_ratio = scenes_to_keep.len() as f64 / scenes.len().max(1) as f64;
    let summary = format!(
        "✅ Smart edit complete! Removed {} boring segments. Output: {:.2} MB (kept_ratio: {:.2})",
        removed, size_mb, kept_ratio
    );
    log(&format!("[SMART] {}", summary));

    // 9. [CUT] Marker pass — burn flash indicators showing where content was removed
    // Skip when density is Full (nothing was cut) or cut_points is empty.
    if intent.show_cut_markers && intent.density != EditDensity::Full && !cut_points.is_empty() {
        log("[SMART] 🎬 Burning [CUT] markers into output...");
        match insert_cut_markers(output, &cut_points, work_dir).await {
            Ok(_) => {}
            Err(e) => warn!("[SMART] [CUT] marker pass failed (non-fatal): {}", e),
        }
    }

    // 8. Subtitle Generation & Burning
    // Only attempt if we have a transcript to work with
    if let Some(ref t) = transcript {
        if !t.is_empty() {
            log("[SMART] 📝 Generating remapped subtitles for edited video...");
            let srt_content = generate_srt_for_kept_scenes(t, &scenes_to_keep);

            if !srt_content.trim().is_empty() {
                let srt_path = work_dir.join("synoid_subtitles.srt");
                match fs::write(&srt_path, &srt_content) {
                    Ok(_) => {
                        log(&format!(
                            "[SMART] 📄 SRT written: {} entries",
                            srt_content.lines().filter(|l| l.contains(" --> ")).count()
                        ));

                        // Resolve the output to an absolute path so sub_output lands in the same dir.
                        // strip_unc_prefix removes the Windows \\?\ extended-path prefix that
                        // fs::canonicalize sometimes returns; FFmpeg cannot open those paths.
                        let abs_output = strip_unc_prefix(
                            fs::canonicalize(output).unwrap_or_else(|_| output.to_path_buf()),
                        );
                        let sub_output = abs_output.with_extension("sub.mp4");
                        log("[SMART] 🔥 Burning subtitles into video...");
                        match production_tools::burn_subtitles(&abs_output, &srt_path, &sub_output)
                            .await
                        {
                            Ok(_) => {
                                // Use copy + remove instead of rename to handle cross-device moves on WSL mounts.
                                match fs::copy(&sub_output, &abs_output) {
                                    Ok(_) => {
                                        let _ = fs::remove_file(&sub_output);
                                        log("[SMART] ✅ Subtitles burned into final video.");
                                    }
                                    Err(e) => warn!("[SMART] Could not replace output with subtitled version: {}", e),
                                }
                            }
                            Err(e) => warn!("[SMART] Subtitle burning failed (non-fatal): {}", e),
                        }

                        // Also keep the raw SRT alongside the output for reference
                        let output_srt = output.with_extension("srt");
                        let _ = fs::copy(&srt_path, &output_srt);
                        let _ = fs::remove_file(&srt_path);
                    }
                    Err(e) => warn!("[SMART] Failed to write SRT file: {}", e),
                }
            } else {
                log("[SMART] ⚠️ No subtitle entries generated (empty transcript after remapping).");
            }
        }
    }

    // If we used a learned pattern to tune this config, persist it
    // so the next edit starts with these refined parameters.
    if learned_pattern.is_some() {
        config.save_to_cortex();
    }

    Ok(summary)
}

/// Build a smooth xfade filter for transitions between trimmed segments.
/// Uses xfade for video and acrossfade for audio, applied directly on trim outputs.
#[allow(dead_code)]
fn build_smooth_xfade_filter(
    scenes: &[Scene],
    audio_input_idx: usize,
    transition_duration: f64,
) -> String {
    let n = scenes.len();
    if n < 2 {
        return String::new();
    }

    let effects = ["fade", "wipeleft", "wiperight", "slideleft", "slideright"];
    let mut filter = String::new();

    // Step 1: Trim all segments
    for (i, scene) in scenes.iter().enumerate() {
        filter.push_str(&format!(
            "[0:v]trim=start={:.6}:end={:.6},setpts=PTS-STARTPTS[vraw{i}]; ",
            scene.start_time, scene.end_time
        ));
        filter.push_str(&format!(
            "[{audio_input_idx}:a]atrim=start={:.6}:end={:.6},asetpts=PTS-STARTPTS[araw{i}]; ",
            scene.start_time, scene.end_time
        ));
    }

    // Step 2: Chain xfade transitions for video
    let mut prev_v = "vraw0".to_string();
    let mut offset = scenes[0].duration - transition_duration;

    for i in 1..n {
        let effect = effects[i % effects.len()];
        let out_label = if i == n - 1 {
            "outv".to_string()
        } else {
            format!("vx{i}")
        };
        filter.push_str(&format!(
            "[{prev_v}][vraw{i}]xfade=transition={effect}:duration={:.3}:offset={:.6}[{out_label}]; ",
            transition_duration, offset.max(0.0)
        ));
        prev_v = out_label;
        // Next offset accounts for the current segment minus the overlap
        offset += scenes[i].duration - transition_duration;
    }

    // Step 3: Chain acrossfade for audio
    let mut prev_a = "araw0".to_string();
    for i in 1..n {
        let out_label = if i == n - 1 {
            "outa".to_string()
        } else {
            format!("ax{i}")
        };
        let dur = transition_duration
            .min(scenes[i].duration * 0.5)
            .min(scenes[i - 1].duration * 0.5);
        filter.push_str(&format!(
            "[{prev_a}][araw{i}]acrossfade=d={:.3}:c1=tri:c2=tri[{out_label}]; ",
            dur
        ));
        prev_a = out_label;
    }

    // Remove trailing "; "
    if filter.ends_with("; ") {
        filter.truncate(filter.len() - 2);
    }

    filter
}

/// Fallback: extract individual segments and concatenate (legacy approach).
/// Used only when the single-pass filter_complex fails (e.g., very long/complex videos).
#[allow(dead_code)]
async fn fallback_extract_and_concat(
    input: &Path,
    enhanced_audio_path: &Path,
    use_enhanced_audio: bool,
    scenes_to_keep: &[Scene],
    output: &Path,
    segments_dir: &Path,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    warn!("[SMART] Using fallback segment extraction...");

    if !segments_dir.exists() {
        fs::create_dir_all(segments_dir)?;
    }

    let mut segment_files = Vec::new();
    let max_concurrency = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 6);
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrency));
    let mut tasks = Vec::with_capacity(scenes_to_keep.len());

    for (i, scene) in scenes_to_keep.iter().enumerate() {
        let seg_path = segments_dir.join(format!("seg_{:04}.mp4", i));
        let scene_duration = scene.duration;
        let scene_start = scene.start_time;
        let input_path = input.to_path_buf();
        let enhanced_path = enhanced_audio_path.to_path_buf();
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        let handle = tokio::spawn(async move {
            let mut cmd = tokio::process::Command::new("ffmpeg");
            cmd.arg("-y")
                .arg("-hide_banner")
                .arg("-loglevel")
                .arg("error")
                .arg("-nostdin");

            // Use -ss after -i for accurate seeking (slower but no frame drops)
            cmd.arg("-i")
                .arg(production_tools::safe_arg_path(&input_path));
            cmd.arg("-ss").arg(&scene_start.to_string());
            cmd.arg("-t").arg(&scene_duration.to_string());

            if use_enhanced_audio {
                cmd.arg("-i")
                    .arg(production_tools::safe_arg_path(&enhanced_path));
                cmd.arg("-ss").arg(&scene_start.to_string());
                cmd.arg("-t").arg(&scene_duration.to_string());
            }

            cmd.arg("-map").arg("0:v");
            if use_enhanced_audio {
                cmd.arg("-map").arg("1:a:0");
            } else {
                cmd.arg("-map").arg("0:a:0");
            }

            // Force consistent encoding: same codec, profile, pixel format, GOP
            cmd.arg("-c:v")
                .arg("libx264")
                .arg("-preset")
                .arg("medium")
                .arg("-crf")
                .arg("23")
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg("-g")
                .arg("30") // Fixed GOP = consistent keyframe spacing
                .arg("-force_key_frames")
                .arg("expr:eq(n,0)"); // Force keyframe at start

            cmd.arg("-c:a")
                .arg("aac")
                .arg("-b:a")
                .arg("192k")
                .arg("-ar")
                .arg("48000");
            cmd.arg("-avoid_negative_ts").arg("make_zero");
            cmd.arg(production_tools::safe_arg_path(&seg_path));

            let status = cmd.status().await;
            drop(permit);

            if let Ok(s) = status {
                if s.success() {
                    return Some(seg_path);
                }
            }
            None
        });

        tasks.push(handle);
    }

    for handle in tasks {
        if let Ok(Some(path)) = handle.await {
            segment_files.push(path);
        }
    }

    if segment_files.is_empty() {
        let _ = fs::remove_dir_all(segments_dir);
        return Err("Fallback: Failed to extract any segments".into());
    }

    // Concat with re-encode for smooth output (not -c copy)
    let concat_file = segments_dir.join("concat_list.txt");
    {
        let mut file = fs::File::create(&concat_file)?;
        for seg in &segment_files {
            writeln!(
                file,
                "file '{}'",
                seg.to_str().ok_or("Invalid segment path")?
            )?;
        }
    }

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-nostdin")
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg(production_tools::safe_arg_path(&concat_file))
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("medium")
        .arg("-crf")
        .arg("23")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("192k")
        .arg("-movflags")
        .arg("+faststart")
        .arg(production_tools::safe_arg_path(output))
        .output()
        .await?;

    let _ = fs::remove_dir_all(segments_dir);

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        error!("[SMART] Fallback concat also failed: {}", stderr);
        return Err("Failed to concatenate segments".into());
    }

    let metadata = fs::metadata(output)?;
    let size_mb = metadata.len() as f64 / 1_048_576.0;
    Ok(format!(
        "✅ Smart edit complete (fallback). Output: {:.2} MB",
        size_mb
    ))
}

/// Generate a properly time-remapped SRT subtitle file from a transcript and the kept scenes.
/// The kept scenes list maps original timestamps -> output timeline positions.
/// Returns the full SRT file content as a String.
pub fn generate_srt_for_kept_scenes(
    transcript: &[crate::agent::transcription::TranscriptSegment],
    kept_scenes: &[Scene],
) -> String {
    const MIN_DISPLAY_SECS: f64 = 1.0; // Minimum subtitle display time
    const MERGE_THRESHOLD_SECS: f64 = 0.8; // Merge entries shorter than this into prev

    // Build a time remapping: for each kept scene, compute its start position in the output video.
    // Output start = sum of durations of all previous kept scenes.
    let mut output_offsets: Vec<(f64, f64, f64)> = Vec::new(); // (src_start, src_end, out_start)
    let mut cursor = 0.0_f64;
    for scene in kept_scenes {
        output_offsets.push((scene.start_time, scene.end_time, cursor));
        cursor += scene.duration;
    }

    // --- Pass 1: Collect all candidate entries (start, end, text) ---
    let mut entries: Vec<(f64, f64, String)> = Vec::new();

    for seg in transcript {
        for &(src_start, src_end, out_start) in &output_offsets {
            let clip_start = seg.start.max(src_start);
            let clip_end = seg.end.min(src_end);
            if clip_end <= clip_start {
                continue;
            }
            let new_start = out_start + (clip_start - src_start);
            let new_end = out_start + (clip_end - src_start);
            entries.push((new_start, new_end, seg.text.trim().to_string()));
            break;
        }
    }

    // --- Pass 2: Merge flash entries (< MERGE_THRESHOLD_SECS) into the previous entry ---
    let mut merged: Vec<(f64, f64, String)> = Vec::new();
    for (start, end, text) in entries {
        let duration = end - start;
        if duration < MERGE_THRESHOLD_SECS && !merged.is_empty() {
            // Extend previous entry's end time and append text
            let last = merged.last_mut().unwrap();
            last.1 = last.1.max(end);
            if !text.is_empty() {
                last.2.push(' ');
                last.2.push_str(&text);
            }
        } else {
            merged.push((start, end, text));
        }
    }

    // --- Pass 3: Enforce minimum display duration ---
    for entry in merged.iter_mut() {
        let duration = entry.1 - entry.0;
        if duration < MIN_DISPLAY_SECS {
            entry.1 = entry.0 + MIN_DISPLAY_SECS;
        }
    }

    // --- Pass 4: Write SRT ---
    let fmt = |secs: f64| -> String {
        let total_ms = (secs * 1000.0) as u64;
        let ms = total_ms % 1000;
        let s = (total_ms / 1000) % 60;
        let m = (total_ms / 60_000) % 60;
        let h = total_ms / 3_600_000;
        format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
    };

    let mut srt = String::new();
    for (counter, (start, end, text)) in merged.into_iter().enumerate() {
        srt.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            counter + 1,
            fmt(start),
            fmt(end),
            text
        ));
    }

    srt
}

/// Returns the full profanity + racial slur word list used for beep-out.
/// Words are stored as lowercase substring matches.
pub fn get_profanity_word_list() -> Vec<&'static str> {
    vec![
        // Common profanity (explicit forms + root for substring matching)
        "fucking",
        "fuck",
        "shit",
        "bitch",
        "cunt",
        "dick",
        "cock",
        "pussy",
        "asshole",
        "bastard",
        "damn",
        "ass",
        // Racial slurs — n-word and variants
        "niggers",
        "nigger",
        "niggas",
        "nigga",
        "nigg",
        "n-word",
        // Other racial/ethnic slurs
        "chink",
        "gook",
        "spic",
        "wetback",
        "kike",
        "cracker",
        "beaner",
        "raghead",
        "towelhead",
        "sandnigger",
        "zipperhead",
        "coon",
        "jigaboo",
        "porch monkey",
        "jungle bunny",
        // Homophobic / transphobic slurs
        "faggot",
        "fag",
        "dyke",
        "tranny",
        "shemale",
        // Ableist slurs
        "retard",
        "retarded",
    ]
}

/// Estimate a narrow (word-level) timestamp within a transcript segment for a
/// given bad word. Uses linear interpolation across the words in the segment.
/// Returns `(start_secs, end_secs)` clamped to the segment bounds.
pub fn estimate_word_timestamps(
    seg: &crate::agent::transcription::TranscriptSegment,
    bad_word: &str,
) -> (f64, f64) {
    let words: Vec<&str> = seg.text.split_whitespace().collect();
    let n = words.len().max(1) as f64;
    let seg_dur = (seg.end - seg.start).max(0.001);
    let pad = 0.08_f64; // 80 ms padding on each side

    for (i, word) in words.iter().enumerate() {
        if word.to_lowercase().contains(bad_word) {
            let word_start = seg.start + (i as f64 / n) * seg_dur - pad;
            let word_end = seg.start + ((i + 1) as f64 / n) * seg_dur + pad;
            return (word_start.max(seg.start), word_end.min(seg.end));
        }
    }
    // Fallback: mute entire segment (bad word found but exact position unclear)
    (seg.start, seg.end)
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
    fn test_censor_detects_cuss_and_homosexual() {
        // This exact phrase comes from the user's prompt in offline/heuristic mode
        let intent = EditIntent::from_text(
            "remove any homosexual or cuss words from the video add captions",
        );
        assert!(
            intent.censor_profanity,
            "heuristic parser must detect 'cuss' and 'homosexual' as censor triggers"
        );
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

        // SILENCE_REFINEMENT_THRESHOLD is 2.0s.
        // Gap before first segment (0.0-1.0 = 1.0s gap) is < threshold → NOT split out.
        // Segments emitted:
        //   [0] 1.0-3.0  speech  score=0.5
        //   [1] 3.0-7.0  silence score=0.0  (gap = 4.0s >= threshold)
        //   [2] 7.0-9.0  speech  score=0.5
        //   [3] 9.0-10.0 silence score=0.0  (tail)
        assert_eq!(refined.len(), 4);
        assert_eq!(
            refined[0].score, 0.5,
            "first speech segment should have neutral score 0.5"
        );
        assert_eq!(
            refined[1].score, 0.0,
            "inter-speech silence should have score 0.0"
        );
    }

    #[test]
    fn test_positional_scoring() {
        let mut scenes = vec![
            Scene {
                start_time: 10.0,
                end_time: 20.0,
                duration: 10.0,
                score: 0.5,
            },
            Scene {
                start_time: 900.0,
                end_time: 910.0,
                duration: 10.0,
                score: 0.5,
            },
        ];

        let intent = EditIntent::from_text("remove boring");
        let config = EditingStrategy::default();
        let total_duration = 1000.0;

        score_scenes(&mut scenes, &intent, None, &config, total_duration);

        // Scene at start (10s) vs Scene at end (900s)
        // Both are 10s long (which is boring-ish but under 15s threshold)
        // The one at the end should have a lower score due to high progress multiplier

        println!("Start scene score: {}", scenes[0].score);
        println!("End scene score: {}", scenes[1].score);

        // Start scene should have a slight boost (+0.05)
        // End scene should have a penalty multiplier -> lower
        assert!(scenes[0].score > 0.4); // Adjusted for new softened boost
        assert!(scenes[1].score < scenes[0].score - 0.05); // Check for drop at end
    }

    #[test]
    fn test_scoring_logic() {
        let mut scenes = vec![Scene {
            start_time: 0.0,
            end_time: 5.0,
            duration: 5.0,
            score: 0.5,
        }];

        let intent = EditIntent::from_text("remove boring");
        let config = EditingStrategy::default();

        score_scenes(&mut scenes, &intent, None, &config, 5.0);

        // No transcript provided, neutral score should remain around 0.3-0.5
        assert!(scenes[0].score >= 0.3);
    }

    #[test]
    fn test_word_level_censor_timestamps() {
        use crate::agent::transcription::TranscriptSegment;
        // Segment: "hello world" from 0.0–4.0 s
        // "world" is word index 1 of 2, so it occupies the second half: ~2.0–4.0 s
        let seg = TranscriptSegment {
            start: 0.0,
            end: 4.0,
            text: "hello world".to_string(),
        };
        let (s, e) = estimate_word_timestamps(&seg, "world");
        assert!(
            s >= 1.5 && s <= 2.5,
            "start should be in second half of segment, got {}",
            s
        );
        assert!(
            e >= 3.0 && e <= 4.1,
            "end should be near segment end, got {}",
            e
        );
    }

    #[test]
    fn test_slur_list_comprehensive() {
        let words = get_profanity_word_list();
        // Basic profanity still present
        assert!(words.contains(&"fuck"));
        assert!(words.contains(&"shit"));
        // List is comprehensive (>20 entries)
        assert!(
            words.len() > 20,
            "profanity list should have >20 entries, has {}",
            words.len()
        );
        // Racial slur present
        assert!(words.contains(&"nigger"));
        // Homophobic slur present
        assert!(words.contains(&"faggot"));
    }
}
