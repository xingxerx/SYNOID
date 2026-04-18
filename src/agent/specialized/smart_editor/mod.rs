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
use crate::agent::engines::process_utils::CommandExt;
use crate::agent::tools::production_tools;
use crate::agent::tools::source_tools;
use crate::agent::tools::transcription::{TranscriptSegment, TranscriptionEngine};
use crate::agent::ai_systems::gpt_oss_bridge::SynoidAgent;
use crate::agent::video_processing::animator::Animator;
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
    learned_pattern: Option<crate::agent::learning::EditingPattern>,
    _animator: Option<Arc<Animator>>,
    enable_subtitles_override: bool,
    enable_censoring_override: bool,
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
    // UI checkboxes always win — override whatever the LLM/heuristic parsed
    intent.enable_subtitles = enable_subtitles_override;
    intent.censor_profanity = enable_censoring_override;

    // 1.1 Render Remotion elements if requested (commented out due to undefined variables)
    let remotion_segment: Option<PathBuf> = None;
    /* Remotion rendering disabled - needs work_dir and job_prefix context
    if intent.use_remotion {
        if let Some(ref anim) = animator {
            log("[SMART] 🎬 Intent requires Remotion — generating produced elements...");
            if let Some(template) = &intent.remotion_template {
                let payload_path = work_dir.join(format!("synoid_{}_remotion_payload.json", job_prefix));
                let remotion_output = work_dir.join(format!("synoid_{}_remotion.mp4", job_prefix));
                
                // Build a basic payload based on the template
                let mut payload = serde_json::json!({
                    "scenes": [
                        {
                            "id": "scene-1",
                            "type": template.to_lowercase(),
                            "duration": 150, // 5 seconds at 30fps
                            "content": {
                                "title": "SYNOID PRODUCTION",
                                "subtitle": intent_text,
                                "stats": [
                                    {"label": "Directorship", "value": "85%"},
                                    {"label": "AI Autonomy", "value": "15%"}
                                ]
                            }
                        }
                    ]
                });

                // If we have a transcript, maybe we can extract a better title/summary
                if let Some(t) = &pre_scanned_transcript {
                    if let Some(first) = t.first() {
                        payload["scenes"][0]["content"]["title"] = serde_json::json!(first.text);
                    }
                }

                if let Ok(json_str) = serde_json::to_string_pretty(&payload) {
                    let _ = fs::write(&payload_path, json_str);
                    match anim.render_animation("DynamicAnimation", &payload_path, &remotion_output).await {
                        Ok(path) => {
                            log(&format!("[SMART] ✅ Remotion render complete: {:?}", path));
                            remotion_segment = Some(path);
                        }
                        Err(e) => warn!("[SMART] Remotion render failed: {}", e),
                    }
                    let _ = fs::remove_file(payload_path);
                }
            }
        } else {
            warn!("[SMART] Intent requested Remotion, but no Animator was provided.");
        }
    }
    */


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

    // Use a deterministic prefix derived from the input path so segment dirs survive across runs.
    let job_prefix_owned: String = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        input.hash(&mut h);
        format!("{:08x}", h.finish() & 0xFFFFFFFF)
    };
    let job_prefix = job_prefix_owned.as_str();

    let input_parent = input.parent().ok_or("Input path has no parent")?;
    // Put all temp files inside a dedicated subdirectory so they don't clutter
    // the user's video folder.  The segments dir already lives here.
    let work_dir_buf = input_parent.join(format!("synoid_temp_{}", job_prefix));
    fs::create_dir_all(&work_dir_buf)
        .map_err(|e| format!("Could not create temp dir: {}", e))?;
    let work_dir: &Path = &work_dir_buf;
    // Store enhanced/censored WAVs next to the input (not in temp dir) so they survive
    // across runs and don't need to be rebuilt every time.
    let enhanced_audio_path = input_parent.join(format!("synoid_{}_audio_enhanced.wav", job_prefix));

    let enhanced_cached = fs::metadata(&enhanced_audio_path)
        .map(|m| m.len() > 0)
        .unwrap_or(false);
    if enhanced_cached {
        log(&format!("[SMART] ⚡ Reusing cached enhanced audio: {:?}", enhanced_audio_path));
    } else {
        log("[SMART] 🎙️ Enhancing audio (High-Pass + Compression + Normalization)...");
        match production_tools::enhance_audio(input, &enhanced_audio_path).await {
            Ok(_) => log("[SMART] Audio enhanced successfully."),
            Err(e) => {
                warn!("[SMART] Audio enhancement failed ({}), using original.", e);
            }
        }
    }

    let mut use_enhanced_audio = if let Ok(metadata) = fs::metadata(&enhanced_audio_path) {
        metadata.len() > 0
    } else {
        false
    };

    // Transcribe — Check for existing SRT files first, then attempt transcription
    // Fall back to extracting audio directly from the raw input if needed.
    log("[SMART] 📝 Checking for existing transcript/SRT files (this saves ~2-5 minutes!)...");
    let transcript = if let Some(t) = pre_scanned_transcript {
        log(&format!(
            "[SMART] Using pre-scanned transcript ({} segments)",
            t.len()
        ));
        Some(t)
    } else {
        // Try to find and reuse existing SRT file (saves massive time!)
        // IMPORTANT: Only load SRT files that are time-aligned to the INPUT video.
        // output.srt / synoid_subtitles.srt are remapped to the *edited* output
        // timeline — loading them for censorship would shift all beep timestamps.
        let possible_srt_paths = vec![
            input.with_extension("srt"),  // e.g., video/Outbound.srt — input-aligned
        ];

        let mut found_srt = None;
        for srt_path in &possible_srt_paths {
            if srt_path.exists() {
                log(&format!("[SMART] 📄 Found existing SRT file: {:?}", srt_path));
                match fs::read_to_string(srt_path) {
                    Ok(srt_content) => {
                        match crate::agent::tools::transcription::parse_srt(&srt_content) {
                            Ok(segments) => {
                                let segments = crate::agent::tools::transcription::filter_hallucinations(segments);
                                // Quick alignment check: last subtitle timestamp vs video duration.
                                // If the SRT was made for a different video or is badly truncated,
                                // re-transcribe rather than bleep the wrong timestamps.
                                let video_dur = source_tools::get_video_duration(input).await.unwrap_or(0.0);
                                let srt_end = segments.last().map(|s| s.end).unwrap_or(0.0);
                                let aligned = if video_dur > 1.0 && srt_end > 0.0 {
                                    // Accept if SRT covers at least 80% of the video (some silence at end is normal)
                                    // and doesn't overshoot by more than 10%
                                    let ratio = srt_end / video_dur;
                                    (0.80..=1.10).contains(&ratio)
                                } else {
                                    true // can't check — accept
                                };

                                if aligned {
                                    // Review the SRT quality before accepting it
                                    let word_count: usize = segments.iter()
                                        .map(|s| s.text.split_whitespace().count())
                                        .sum();
                                    let avg_seg_secs = if segments.is_empty() { 0.0 } else {
                                        segments.iter().map(|s| s.end - s.start).sum::<f64>() / segments.len() as f64
                                    };
                                    log(&format!(
                                        "[SMART] ✅ SRT review: {} segments, ~{} words, avg {:.1}s/seg, covers {:.0}% of video — reusing (no re-transcription)",
                                        segments.len(), word_count, avg_seg_secs,
                                        (srt_end / video_dur.max(0.001)) * 100.0
                                    ));
                                    found_srt = Some(segments);
                                    break;
                                } else {
                                    warn!(
                                        "[SMART] ⚠️ SRT end ({:.1}s) doesn't align with video duration ({:.1}s) — re-transcribing",
                                        srt_end, video_dur
                                    );
                                    // Delete the mismatched SRT so it doesn't keep blocking
                                    let _ = fs::remove_file(srt_path);
                                }
                            }
                            Err(e) => {
                                warn!("[SMART] Failed to parse SRT file: {}, will re-transcribe", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[SMART] Failed to read SRT file: {}, will re-transcribe", e);
                    }
                }
            }
        }

        if found_srt.is_some() {
            found_srt
        } else {
            log("[SMART] 🎤 No existing SRT found, transcribing audio for semantic understanding...");
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
                        // Cache as input-aligned SRT so future runs skip re-transcription.
                        // Named after the input file (e.g., Outbound.srt) — NOT the output SRT.
                        let input_srt_path = input.with_extension("srt");
                        let srt_content = crate::agent::tools::transcription::generate_srt(&t);
                        match fs::write(&input_srt_path, &srt_content) {
                            Ok(_) => log(&format!(
                                "[SMART] 💾 Saved input-aligned SRT to {:?} (reused next run)",
                                input_srt_path
                            )),
                            Err(e) => warn!("[SMART] Could not save input SRT cache: {}", e),
                        }
                        Some(t)
                    }
                    Err(e) => {
                        warn!("[SMART] Transcription failed: {}", e);
                        None
                    }
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
    // Use the enhanced audio if available; otherwise fall back to the raw input so
    // censoring is always applied to an existing file regardless of whether the
    // audio-enhancement step succeeded.
    let mut final_enhanced_audio_path = if use_enhanced_audio {
        enhanced_audio_path.clone()
    } else {
        input.to_path_buf()
    };
    if intent.censor_profanity {
        log(&format!("[SMART] 🤬 Profanity censorship enabled: {}", intent.censor_profanity));
        if let Some(t) = &transcript {
            log(&format!("[SMART] 🤬 Applying audio censorship pass based on transcript ({} segments)...", t.len()));
            // Stable path alongside the input so it survives across runs.
            let censored_path = input_parent.join(format!("synoid_{}_audio_censored.wav", job_prefix));

            // Reuse cached censored WAV if it already exists and has content.
            let mut censored_cached = fs::metadata(&censored_path)
                .map(|m| m.len() > 0)
                .unwrap_or(false);

            // Profanity list fingerprint — stored as a sidecar .meta file.
            // If the list changed (e.g. new words added), the cached WAV won't cover them.
            let censored_meta_path = input_parent.join(format!("synoid_{}_audio_censored.meta", job_prefix));
            let current_list_fingerprint = {
                let words = get_profanity_word_list();
                format!("n={}", words.len())
            };

            // Invalidate if the input SRT (transcript) was regenerated after the censored WAV.
            // Stale censored audio would miss newly detected profanity or have wrong timestamps.
            if censored_cached {
                let srt_path_check = input.with_extension("srt");
                let srt_newer = fs::metadata(&srt_path_check).ok()
                    .zip(fs::metadata(&censored_path).ok())
                    .and_then(|(srt_m, cen_m)| {
                        srt_m.modified().ok().zip(cen_m.modified().ok())
                    })
                    .map(|(srt_time, cen_time)| srt_time > cen_time)
                    .unwrap_or(false);
                if srt_newer {
                    log("[SMART] ♻️ Transcript SRT is newer than cached beep audio — regenerating censored audio for accurate beep timing...");
                    let _ = fs::remove_file(&censored_path);
                    let _ = fs::remove_file(&censored_meta_path);
                    censored_cached = false;
                }
            }

            // Invalidate if the profanity list has changed since the WAV was generated.
            if censored_cached {
                let stored_fingerprint = fs::read_to_string(&censored_meta_path).unwrap_or_default();
                if stored_fingerprint.trim() != current_list_fingerprint {
                    log("[SMART] ♻️ Profanity list changed — regenerating censored audio to include new words...");
                    let _ = fs::remove_file(&censored_path);
                    let _ = fs::remove_file(&censored_meta_path);
                    censored_cached = false;
                }
            }

            if censored_cached {
                log(&format!("[SMART] ⚡ Reusing cached censored audio: {:?}", censored_path));
                final_enhanced_audio_path = censored_path;
                use_enhanced_audio = true;
            } else {

            // Comprehensive list of words to bleep — racial slurs, hate speech, and profanity
            let profanity_words = get_profanity_word_list();
            let mut censor_timestamps: Vec<(f64, f64)> = Vec::new();
            let mut segments_with_profanity = 0;

            for seg in t {
                let text_lower = seg.text.to_lowercase();
                let mut found_in_segment = false;

                for bad_word in &profanity_words {
                    if word_boundary_match(&text_lower, bad_word) {
                        info!(
                            "[SMART] 🤬 Found profanity '{}' in segment: \"{}\" ({:.2}s-{:.2}s)",
                            bad_word, seg.text, seg.start, seg.end
                        );
                        found_in_segment = true;
                        // Use word-level timestamp (can have multiple occurrences in one segment)
                        let word_timestamps = estimate_word_timestamps(seg, bad_word);
                        censor_timestamps.extend(word_timestamps);
                    }
                }

                if found_in_segment {
                    segments_with_profanity += 1;
                }
            }

            log(&format!("[SMART] 📊 Profanity scan complete: found in {}/{} segments",
                segments_with_profanity, t.len()));
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
                // Validate replacement SFX path exists; fall back to built-in beep if not
                let replacement_sfx: Option<&str> =
                    intent.profanity_replacement.as_deref().and_then(|p| {
                        if Path::new(p).exists() {
                            Some(p)
                        } else {
                            warn!(
                                "[SMART] profanity_replacement '{}' not found, using built-in beep.",
                                p
                            );
                            None
                        }
                    });
                match production_tools::apply_audio_censor(
                    &final_enhanced_audio_path,
                    &censored_path,
                    &censor_timestamps,
                    replacement_sfx,
                )
                .await
                {
                    Ok(_) => {
                        log(&format!(
                            "[SMART] Successfully censored {} segments.",
                            censor_timestamps.len()
                        ));
                        // Write fingerprint so future runs know which list version produced this WAV.
                        let _ = fs::write(&censored_meta_path, &current_list_fingerprint);
                        final_enhanced_audio_path = censored_path;
                        use_enhanced_audio = true; // ensure censored track is used in segments
                    }
                    Err(e) => warn!(
                        "[SMART] Audio censorship failed: {}, using original audio.",
                        e
                    ),
                }
            } else {
                log("[SMART] ℹ️ No profanity detected in transcript.");
            }
            } // end else (censored_cached)
        } else {
            warn!("[SMART] ⚠️ Profanity censorship requested but no transcript available!");
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

    let agent = Arc::new(SynoidAgent::new("http://localhost:11434", "llava:latest"));

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

    // Combine strictly contiguous scenes in `scenes_to_keep` so we don't
    // chop the video up into identical contiguous parts during extraction.
    {
        let before_contig = scenes_to_keep.len();
        let mut merged: Vec<crate::agent::specialized::smart_editor::types::Scene> = Vec::new();
        for sc in scenes_to_keep {
            if let Some(last) = merged.last_mut() {
                // If the start of this scene is basically the end of the last one
                if sc.start_time - last.end_time <= 0.25 {
                    last.end_time = sc.end_time;
                    last.duration = last.end_time - last.start_time;
                    continue;
                }
            }
            merged.push(sc);
        }
        scenes_to_keep = merged;
        log(&format!(
            "[SMART] 🔗 Contiguous-merge: {} → {} physical segments for rendering",
            before_contig,
            scenes_to_keep.len()
        ));
    }

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

    // work_dir IS already the synoid_temp_{prefix} folder; segments live inside it.
    let segments_dir = work_dir.to_path_buf();
    let total_segments = scenes_to_keep.len();

    // Fingerprint the current scene selection so we can detect if scenes changed between runs.
    // Format: "start,end" per line, one line per scene — fast to compare with fs::read_to_string.
    let scene_fingerprint: String = scenes_to_keep.iter()
        .map(|s| format!("{:.6},{:.6}", s.start_time, s.end_time))
        .collect::<Vec<_>>()
        .join("\n");
    let fingerprint_path = segments_dir.join("scene_fingerprint.txt");
    let cached_fingerprint = if segments_dir.exists() {
        fs::read_to_string(&fingerprint_path).ok()
    } else {
        None
    };
    let fingerprint_matches = cached_fingerprint.as_deref() == Some(scene_fingerprint.as_str());

    // Check if all segments from a previous run already exist — reuse them to save time.
    // Fingerprint must match to ensure the cached segments correspond to the current edit.
    let all_segs_cached = fingerprint_matches && segments_dir.exists() && {
        (0..total_segments).all(|i| {
            let p = segments_dir.join(format!("seg_{:04}.mp4", i));
            p.exists() && fs::metadata(&p).map(|m| m.len() > 1000).unwrap_or(false)
        })
    };

    if all_segs_cached {
        log(&format!(
            "[SMART] ⚡ Reusing {} cached segments from previous run (skipping re-cut).",
            total_segments
        ));
    } else {
        if segments_dir.exists() {
            fs::remove_dir_all(&segments_dir)?;
        }
        fs::create_dir_all(&segments_dir)?;
    }

    log("[SMART] ✂️ Assembling segments with single-pass render...");

    // Commentary Generator removed (funny_engine deprecated)

    // NVENC hardware encoder session limits:
    // - GeForce consumer GPUs: max 2-3 concurrent sessions (driver enforced)
    // - Quadro/Tesla: higher limits (8+)
    // Use conservative limit to prevent "incompatible client key" and OOM errors
    let gpu_ctx = crate::gpu_backend::get_gpu_context().await;
    let max_concurrency = if gpu_ctx.has_gpu() {
        2  // Conservative limit for NVENC consumer GPUs
    } else {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(2, 6)
    };
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrency));
    let mut tasks = Vec::with_capacity(total_segments);

    for (i, scene) in scenes_to_keep.iter().enumerate() {
        let seg_path = segments_dir.join(format!("seg_{:04}.mp4", i));

        // Skip re-encoding segments that already exist from cache
        if all_segs_cached {
            tasks.push(tokio::spawn(async move {
                let dur = source_tools::get_video_duration(&seg_path).await.unwrap_or(0.0);
                Some((seg_path, dur))
            }));
            continue;
        }
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

            let gpu_ctx = crate::gpu_backend::get_gpu_context().await;

            // Enable hardware decode acceleration if available
            if let Some(hwaccel) = gpu_ctx.ffmpeg_hwaccel() {
                cmd.arg("-hwaccel").arg(hwaccel);
            }

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

            let neuro = crate::agent::neuroplasticity::Neuroplasticity::new();
            cmd.arg("-c:v").arg(gpu_ctx.ffmpeg_encoder());
            cmd.arg("-pix_fmt").arg("yuv420p");
            for flag in gpu_ctx.neuroplastic_ffmpeg_flags(neuro.current_speed()) {
                cmd.arg(flag);
            }

            // Ensure frame dimensions are even, which NVENC requires.
            cmd.arg("-vf").arg("scale=trunc(iw/2)*2:trunc(ih/2)*2");

            // High quality fixed quantization for intermediate clips if encoding supports it
            if gpu_ctx.has_gpu() {
                cmd.arg("-rc").arg("vbr"); // Required for NVENC -cq to work properly
                cmd.arg("-b:v").arg("0");
                cmd.arg("-cq").arg("23"); // NVENC constant quality
            } else {
                cmd.arg("-crf").arg("23"); // CPU
            }

            // Always re-encode audio to AAC to ensure format consistency
            cmd.arg("-c:a").arg("aac").arg("-b:a").arg("192k");

            cmd.arg("-avoid_negative_ts").arg("make_zero");
            cmd.arg(production_tools::safe_arg_path(&seg_path));

            let output_res = cmd.output().await;
            drop(permit); // Release concurrency slot

            if let Ok(s) = output_res {
                if s.status.success() {
                    return Some((seg_path, scene_duration));
                } else {
                    tracing::error!(
                        "[SMART] Segment extraction failed for {}: {}",
                        seg_path.display(),
                        String::from_utf8_lossy(&s.stderr)
                    );
                }
            } else if let Err(e) = output_res {
                tracing::error!("[SMART] Failed to spawn ffmpeg: {}", e);
            }
            None
        });

        tasks.push(handle);
    }

    // Await all segment-extraction tasks and collect successful results
    let mut segment_files: Vec<std::path::PathBuf> = Vec::new();
    
    // ADD PRODUCED REMOTION SEGMENTS IF ANY
    if let Some(path) = remotion_segment {
        segment_files.push(path);
    }

    for handle in tasks {
        if let Ok(Some((path, _dur))) = handle.await {
            segment_files.push(path);
        }
    }

    if segment_files.is_empty() {
        fs::remove_dir_all(&segments_dir).ok();
        return Err("Failed to extract any video segments".into());
    }

    // Persist scene fingerprint so future runs can validate the segment cache.
    if !all_segs_cached {
        let _ = fs::write(&fingerprint_path, &scene_fingerprint);
    }

    log(&format!(
        "[SMART] 🔗 Stitching {} segments together...",
        segment_files.len()
    ));

    // 7. Stitch segments — use crossfade transitions when feasible (≤ 30 segments),
    //    fall back to simple concat for very long edit lists.
    let xfade_dur = neuro_transition_dur.clamp(0.12, 0.25);
    let applied_xfade_dur = if segment_files.len() >= 2 && segment_files.len() <= 30 {
        xfade_dur
    } else {
        0.0
    };

    let status = if applied_xfade_dur > 0.0 {
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

        let gpu_ctx = crate::gpu_backend::get_gpu_context().await;

        // Enable hardware decode acceleration for all inputs if available
        if let Some(hwaccel) = gpu_ctx.ffmpeg_hwaccel() {
            cmd.arg("-hwaccel").arg(hwaccel);
        }

        // Add all segment files as inputs
        for seg in &segment_files {
            cmd.arg("-i").arg(production_tools::safe_arg_path(seg));
        }

        cmd.arg("-filter_complex").arg(&filter);
        cmd.arg("-map").arg("[outv]");
        cmd.arg("-map").arg("[outa]");

        let neuro = crate::agent::neuroplasticity::Neuroplasticity::new();
        cmd.arg("-c:v").arg(gpu_ctx.ffmpeg_encoder());
        cmd.arg("-pix_fmt").arg("yuv420p");
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
    // Only attempt if we have a transcript to work with and subtitles are enabled
    if let Some(ref t) = transcript {
        if !t.is_empty() && intent.enable_subtitles {
            log("[SMART] 📝 Generating remapped subtitles for edited video...");
            
            // Probe exact segment durations to prevent cumulative subtitle drift 
            // Chunked concurrency to avoid launching 1500+ ffprobe processes simultaneously
            let mut exact_durations = Vec::with_capacity(segment_files.len());
            for chunk in segment_files.chunks(50).enumerate() {
                let mut tasks = Vec::new();
                for (idx, p) in chunk.1.iter().enumerate() {
                    let global_i = chunk.0 * 50 + idx;
                    let path = p.clone();
                    let fallback = scenes_to_keep.get(global_i).map(|s| s.duration).unwrap_or(0.0);
                    tasks.push(tokio::spawn(async move {
                        let probe = Command::new("ffprobe")
                            .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1"])
                            .arg(production_tools::safe_arg_path(&path))
                            .output().await;
                        if let Ok(m) = probe {
                            String::from_utf8_lossy(&m.stdout).trim().parse::<f64>().unwrap_or(fallback)
                        } else {
                            fallback
                        }
                    }));
                }
                for task in tasks {
                    exact_durations.push(task.await.unwrap_or(0.0));
                }
            }

            let srt_content = generate_srt_for_kept_scenes(t, &scenes_to_keep, &exact_durations, applied_xfade_dur);

            if !srt_content.trim().is_empty() {
                // Resolve the output to an absolute path first so we write the temp SRT directly
                // to a stable location alongside it, preventing 'os error 3' if work_dir was lost.
                let abs_output = strip_unc_prefix(
                    fs::canonicalize(output).unwrap_or_else(|_| output.to_path_buf()),
                );
                
                let srt_path = abs_output.with_extension("temp.srt");
                let output_srt = abs_output.with_extension("srt");
                let sub_output = abs_output.with_extension("sub.mp4");

                match fs::write(&srt_path, &srt_content) {
                    Ok(_) => {
                        log(&format!(
                            "[SMART] 📄 SRT written: {} entries",
                            srt_content.lines().filter(|l| l.contains(" --> ")).count()
                        ));

                        log("[SMART] 🔥 Burning subtitles into video...");
                        match production_tools::burn_subtitles(&abs_output, &srt_path, &sub_output)
                            .await
                        {
                            Ok(_) => {
                                // Validate the subtitled output was successfully created and is not corrupted
                                match fs::metadata(&sub_output) {
                                    Ok(metadata) if metadata.len() > 1_000_000 => {
                                        // File exists and is at least 1MB - likely valid
                                        // Verify it's a valid video by checking duration
                                        let sub_duration = source_tools::get_video_duration(&sub_output).await.unwrap_or(0.0);
                                        if sub_duration > 1.0 {
                                            // Use copy + remove instead of rename to handle cross-device moves on WSL mounts.
                                            match fs::copy(&sub_output, &abs_output) {
                                                Ok(_) => {
                                                    let _ = fs::remove_file(&sub_output);
                                                    log("[SMART] ✅ Subtitles burned into final video.");
                                                }
                                                Err(e) => warn!("[SMART] Could not replace output with subtitled version: {}", e),
                                            }
                                        } else {
                                            warn!("[SMART] Subtitled video appears corrupted (duration: {:.2}s), keeping original", sub_duration);
                                            let _ = fs::remove_file(&sub_output);
                                        }
                                    }
                                    _ => {
                                        warn!("[SMART] Subtitled output file is missing or too small, keeping original");
                                        let _ = fs::remove_file(&sub_output);
                                    }
                                }
                            }
                            Err(e) => warn!("[SMART] Subtitle burning failed (non-fatal): {}", e),
                        }

                        // Keep the raw SRT alongside the output for reference and clean up the temp
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

    // Clean up — remove entire temp dir (segments + WAVs).  Non-fatal so a
    // missing dir from a previous run doesn't abort an otherwise-complete edit.
    let _ = fs::remove_dir_all(&work_dir_buf);

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

    // Use CPU encoding in fallback mode to avoid NVENC issues entirely
    let max_concurrency = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 4);  // Reduced from 6 to 4
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
                words: Vec::new(),
            },
            TranscriptSegment {
                start: 7.0,
                end: 9.0,
                text: "World".to_string(),
                words: Vec::new(),
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
        use crate::agent::tools::transcription::TranscriptSegment;
        // Segment: "hello world" from 0.0–4.0 s
        // "world" is word index 1 of 2, so it occupies the second half: ~2.0–4.0 s
        let seg = TranscriptSegment {
            start: 0.0,
            end: 4.0,
            text: "hello world".to_string(),
            words: Vec::new(),
        };
        let timestamps = estimate_word_timestamps(&seg, "world");
        assert_eq!(timestamps.len(), 1, "should find exactly one occurrence");
        let (s, e) = timestamps[0];
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
        use crate::agent::tools::transcription::TranscriptSegment;
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
            words: Vec::new(),
        }];
        assert!(scene_has_speech(&scene, Some(&transcript)));

        let disjoint_transcript = vec![TranscriptSegment {
            start: 5.0,
            end: 6.0,
            text: "later speech".to_string(),
            words: Vec::new(),
        }];
        assert!(!scene_has_speech(&scene, Some(&disjoint_transcript)));
    }
}
