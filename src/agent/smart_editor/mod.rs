pub mod types;
pub mod scene_ops;
pub mod filter_ops;
pub mod transition_ops;
pub use types::*;
pub use scene_ops::*;
pub use filter_ops::*;
pub use transition_ops::*;
// SYNOID Smart Editor Refactoring

// SYNOID Smart Editor - AI-Powered Intent-Based Video Editing
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// This module provides intelligent video editing based on natural language intent.
// It analyzes scenes, scores them against user intent, and generates trimmed output.

use std::sync::Arc;
use crate::agent::process_utils::CommandExt;
use crate::agent::production_tools;
use crate::agent::transcription::{TranscriptSegment, TranscriptionEngine};
use crate::agent::gpt_oss_bridge::SynoidAgent;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{error, info, warn};

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
    let intent = EditIntent::from_llm(intent_text).await;


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
    let job_id = uuid::Uuid::new_v4().to_string();
    let job_prefix = &job_id[..8];

    let work_dir = input.parent().ok_or("Input path has no parent")?;
    let enhanced_audio_path = work_dir.join(format!("synoid_{}_audio_enhanced.wav", job_prefix));

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

    // Transcribe — always attempt, even if audio enhancement failed.
    // Fall back to extracting audio directly from the raw input if needed.
    log("[SMART] 📝 Transcribing audio for semantic understanding...");
    let transcript = if let Some(t) = pre_scanned_transcript {
        log(&format!(
            "[SMART] Using pre-scanned transcript ({} segments)",
            t.len()
        ));
        Some(t)
    } else {
        let whisper_audio_path = work_dir.join(format!("synoid_{}_audio_whisper.wav", job_prefix));

        // Prefer the enhanced WAV; fall back to extracting directly from raw input.
        let audio_source = if use_enhanced_audio {
            enhanced_audio_path.clone()
        } else {
            log("[SMART] ⚠️ Audio enhancement unavailable — extracting audio from raw footage for transcription...");
            input.to_path_buf()
        };

        log("[SMART] 🎧 Extracting 16kHz mono audio for Whisper...");
        let audio_for_whisper =
            match production_tools::extract_audio_wav(&audio_source, &whisper_audio_path).await {
                Ok(p) => p,
                Err(e) => {
                    warn!(
                        "[SMART] Failed to extract 16kHz mono audio: {}. Attempting transcription from source directly.",
                        e
                    );
                    audio_source.clone()
                }
            };

        match TranscriptionEngine::new(None).await {
            Err(e) => {
                warn!("[SMART] Transcription engine init failed: {}", e);
                None
            }
            Ok(engine) => {
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
            }
        }
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
            let censored_path = work_dir.join(format!("synoid_{}_audio_censored.wav", job_prefix));

            // Comprehensive list of words to bleep — racial slurs, hate speech, and profanity
            let profanity_words = get_profanity_word_list();
            let mut censor_timestamps: Vec<(f64, f64)> = Vec::new();

            for seg in t {
                let text_lower = seg.text.to_lowercase();
                for bad_word in &profanity_words {
                    if word_boundary_match(&text_lower, bad_word) {
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

    // 2.8 Semantic Vision Scan (rate-limited, sampled)
    // Cap at 40 frames to stay within Gemini free-tier (1500 req/day, 15 RPM).
    // Sample evenly across all eligible scenes so the whole video is represented.
    const MAX_VISION_FRAMES: usize = 40;
    log("[SMART] 👁️ Performing sampled vision scan on scenes...");

    let agent = Arc::new(SynoidAgent::new("", "gemini-2.0-flash"));

    let all_eligible: Vec<(usize, f64, f64)> = scenes.iter().enumerate()
        .filter(|(_, s)| s.duration >= 2.0)
        .map(|(i, s)| (i, s.start_time, s.end_time))
        .collect();

    // Even stride sampling: pick at most MAX_VISION_FRAMES spread across the whole list
    let stride = (all_eligible.len() / MAX_VISION_FRAMES).max(1);
    let scenes_to_scan: Vec<(usize, f64, f64)> = all_eligible
        .into_iter()
        .enumerate()
        .filter(|(idx, _)| idx % stride == 0)
        .map(|(_, v)| v)
        .take(MAX_VISION_FRAMES)
        .collect();

    let total_to_scan = scenes_to_scan.len();
    log(&format!(
        "[SMART] Vision Scan: sampling {}/{} scenes (stride {})",
        total_to_scan, scenes.len(), stride
    ));

    // Sequential with a small inter-call delay to stay under 15 RPM
    for (completed, (i, start_time, end_time)) in scenes_to_scan.into_iter().enumerate() {
        let mid_time = start_time + (end_time - start_time) / 2.0;
        let frame_path = format!("temp_frame_{}_{}.jpg", start_time.to_bits(), end_time.to_bits());
        let input_path = input.to_path_buf();

        let extract_status = tokio::process::Command::new("ffmpeg")
            .stealth()
            .args(["-y", "-ss", &mid_time.to_string(), "-i",
                   input_path.to_str().unwrap_or_default(),
                   "-frames:v", "1", "-q:v", "2", &frame_path])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;

        if let Ok(st) = extract_status {
            if st.success() {
                if let Ok(desc) = crate::agent::vision_tools::describe_frame_multi_provider(
                    &agent, &PathBuf::from(&frame_path), mid_time,
                ).await {
                    if !desc.tags.is_empty() {
                        scenes[i].vision_tags = desc.tags;
                    }
                }
            }
        }
        let _ = tokio::fs::remove_file(&frame_path).await;

        if (completed + 1) % 10 == 0 || completed + 1 == total_to_scan {
            log(&format!("[SMART] Vision progress: {}/{} frames analyzed", completed + 1, total_to_scan));
        }
        // ~4s gap between calls keeps us safely under 15 RPM (= 1 req/4s)
        if completed + 1 < total_to_scan {
            tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
        }
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
            .filter(|s| s.duration >= 3.5 || scene_has_speech(s, transcript.as_deref()))
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

    let segments_dir = work_dir.join(format!("synoid_temp_{}", job_prefix));
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
            cmd.stealth();
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
                .stealth()
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
        cmd.stealth();
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
                .stealth()
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
            .stealth()
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
            cmd.stealth();
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
        .stealth()
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
            vision_tags: Vec::new(),
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
                vision_tags: Vec::new(),
            },
            Scene {
                start_time: 900.0,
                end_time: 910.0,
                duration: 10.0,
                score: 0.5,
                vision_tags: Vec::new(),
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
            vision_tags: Vec::new(),
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
        // New words present
        assert!(words.contains(&"negro"));
        assert!(words.contains(&"motherfucker"));
        assert!(words.contains(&"bullshit"));
        // List is comprehensive (>20 entries)
        assert!(
            words.len() > 30,
            "profanity list should have >30 entries, has {}",
            words.len()
        );
        // Racial slur present
        assert!(words.contains(&"nigger"));
        // Homophobic slur present
        assert!(words.contains(&"faggot"));
    }

    #[test]
    fn test_word_boundary_matching() {
        assert!(word_boundary_match("What the fuck is this", "fuck"));
        assert!(word_boundary_match("Fucking hell!", "fuck")); // starts_with check
        assert!(word_boundary_match("asshole", "ass"));
        assert!(!word_boundary_match("I have a class today", "ass"));
        assert!(!word_boundary_match("He passed the ball", "ass"));
        assert!(word_boundary_match("this is bullshit.", "bullshit"));
    }

    #[test]
    fn test_scene_has_speech() {
        use crate::agent::transcription::TranscriptSegment;
        let scene = Scene {
            start_time: 2.0,
            end_time: 4.0,
            duration: 2.0,
            score: 0.5,
            vision_tags: Vec::new(),
        };
        let transcript = vec![TranscriptSegment {
            start: 2.5,
            end: 3.5,
            text: "speech here".to_string(),
        }];
        assert!(scene_has_speech(&scene, Some(&transcript)));

        let disjoint_transcript = vec![TranscriptSegment {
            start: 5.0,
            end: 6.0,
            text: "later speech".to_string(),
        }];
        assert!(!scene_has_speech(&scene, Some(&disjoint_transcript)));
    }
}
