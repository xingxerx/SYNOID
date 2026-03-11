use tokio::process::Command;
use super::types::{Scene, EditIntent, EditingStrategy, EditDensity};
use crate::agent::transcription::TranscriptSegment;
use tracing::{info, warn};
use crate::agent::vision_tools;
use std::path::{Path, PathBuf};
// SYNOID Smart Editor Refactoring

const SILENCE_REFINEMENT_THRESHOLD: f64 = 2.0; // Seconds of silence to trigger a scene split (≤2 s pause = natural speech rhythm, not a cut point)
use regex::Captures;
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
            vision_tags: Vec::new(),
        });
    }

    // If no scenes detected, treat entire video as one scene
    if scenes.is_empty() {
        scenes.push(Scene {
            start_time: 0.0,
            end_time: total_duration,
            duration: total_duration,
            score: 1.0,
            vision_tags: Vec::new(),
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
            let min_speech_score = if is_ruthless { 0.35 } else { 0.45 };
            max_score = max_score.max(min_speech_score);

            for &i in &overlapping_indices {
                if scenes[i].score < max_score {
                    // In ruthless mode, we only boost if the gap isn't too large or score too low
                    // Trying to preserve flow without keeping dead air
                    let current_score = scenes[i].score;

                    if is_ruthless {
                        if current_score < 0.05 {
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
                    vision_tags: scene.vision_tags.clone(),
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
                    vision_tags: scene.vision_tags.clone(),
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
                vision_tags: scene.vision_tags.clone(),
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

        // Vision Heuristics
        let mut has_bad_app = false;
        let mut has_main_app = false;
        for tag in &scene.vision_tags {
            let t = tag.to_lowercase();
            if t.contains("discord") || t.contains("browser") || t.contains("desktop") {
                has_bad_app = true;
            }
            if t.contains("main_app") || t.contains("game") {
                has_main_app = true;
            }
        }

        if has_bad_app {
            score -= 1.0; // Huge penalty for secondary apps
            info!("[SMART] 🛑 Penalizing scene at {:.1}s due to detected background app.", scene.start_time);
        } else if has_main_app {
            score += 0.2; // Boost main app
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
                } else if speech_ratio > 0.1 {
                    score += config.speech_boost * 0.5;
                }
            }

            // NEW: Always preserve talking scenes by heavily boosting score
            if speech_ratio > 0.1 {
                score = score.max(0.95);
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

pub fn scene_has_speech(scene: &Scene, transcript: Option<&[TranscriptSegment]>) -> bool {
    if let Some(segments) = transcript {
        for seg in segments {
            let seg_start = seg.start.max(scene.start_time);
            let seg_end = seg.end.min(scene.end_time);
            if seg_end > seg_start + 0.1 {
                return true;
            }
        }
    }
    false
}
