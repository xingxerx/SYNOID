use super::types::{Scene};
use tracing::{info, warn};
use std::path::Path;
use std::fs;
use tokio::process::Command;
use crate::agent::engines::process_utils::CommandExt;
// SYNOID Smart Editor Refactoring

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
        .stealth()
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
        .stealth()
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
            "-pix_fmt",
            "yuv420p",
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
    let gpu_ctx = crate::gpu_backend::get_gpu_context().await;
    let mut mark_cmd = Command::new("ffmpeg");
    mark_cmd.stealth();
    mark_cmd.args(["-y", "-hide_banner", "-loglevel", "error", "-nostdin"]);
    if let Some(hwaccel) = gpu_ctx.ffmpeg_hwaccel() {
        mark_cmd.args(["-hwaccel", hwaccel]);
    }
    mark_cmd.arg("-i").arg(output.to_str().unwrap_or(""));
    mark_cmd.args(["-vf", &flash_drawtext, "-pix_fmt", "yuv420p"]);
    mark_cmd.arg("-c:v").arg(gpu_ctx.ffmpeg_encoder());
    if gpu_ctx.has_gpu() {
        mark_cmd.args(["-rc", "vbr", "-b:v", "0", "-cq", "23", "-maxrate", "20M", "-bufsize", "40M"]);
    } else {
        mark_cmd.args(["-preset", "ultrafast", "-crf", "23"]);
    }
    mark_cmd.args(["-c:a", "copy"]);
    mark_cmd.arg(marked_path.to_str().unwrap_or(""));
    let mark_status = mark_cmd.status().await?;

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
pub fn generate_srt_for_kept_scenes(
    transcript: &[crate::agent::transcription::TranscriptSegment],
    kept_scenes: &[Scene],
    exact_durations: &[f64],
    xfade_dur: f64,
) -> String {
    const MIN_DISPLAY_SECS: f64 = 2.5; // Minimum subtitle display time — readable at normal viewing speed
    const MAX_DISPLAY_SECS: f64 = 12.0; // Cap — allows long sentences to breathe
    const MERGE_THRESHOLD_SECS: f64 = 0.8; // Merge entries shorter than this into prev
    const CAPTION_GAP_SECS: f64 = 0.05; // Minimum gap between consecutive captions

    // Build a time remapping: for each kept scene, compute its start position in the output video.
    // Output start = sum of durations of all previous kept scenes.
    let mut output_offsets: Vec<(f64, f64, f64)> = Vec::new(); // (src_start, src_end, out_start)
    let mut cursor = 0.0_f64;
    for (i, scene) in kept_scenes.iter().enumerate() {
        let actual_dur = exact_durations.get(i).copied().unwrap_or(scene.duration);
        output_offsets.push((scene.start_time, scene.end_time, cursor));
        cursor += actual_dur;
        if i < kept_scenes.len().saturating_sub(1) {
            cursor -= xfade_dur;
        }
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
            // removed break to allow subtitle segments to span across cut boundaries
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

    // --- Pass 3: Enforce min/max display duration ---
    for entry in merged.iter_mut() {
        let duration = entry.1 - entry.0;
        if duration < MIN_DISPLAY_SECS {
            entry.1 = entry.0 + MIN_DISPLAY_SECS;
        } else if duration > MAX_DISPLAY_SECS {
            entry.1 = entry.0 + MAX_DISPLAY_SECS;
        }
    }

    // --- Pass 4: Anti-overlap — clamp each end to next entry's start ---
    // Never clamp below MIN_DISPLAY_SECS — a slight overlap is better than an unreadable flash.
    for i in 0..merged.len().saturating_sub(1) {
        let next_start = merged[i + 1].0;
        let max_end = (next_start - CAPTION_GAP_SECS).max(merged[i].0 + MIN_DISPLAY_SECS);
        if merged[i].1 > max_end {
            merged[i].1 = max_end;
        }
    }

    // --- Pass 5: Write SRT ---
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
        "fucked",
        "fucker",
        "fucks",
        "fuckhead",
        "fuckface",
        "fuk",      // Common misspelling/phonetic
        "fck",      // Abbreviation
        "f*ck",     // Censored version
        "shit",
        "shitty",
        "shitting",
        "shithead",
        "shitface",
        "sht",      // Phonetic
        "sh*t",     // Censored
        "bitch",
        "bitches",
        "bitching",
        "bitchy",
        "cunt",
        "cunts",
        "dick",
        "dicks",
        "dickhead",
        "cock",
        "cocks",
        "cocksucker",
        "pussy",
        "pussies",
        "asshole",
        "assholes",
        "bastard",
        "bastards",
        "damn",
        "damned",
        "damnit",
        // "ass" as a standalone word — matched with exact boundaries in word_boundary_match
        // to prevent false positives (class, pass, passionate, etc.)
        "ass",
        "dumbass",
        "smartass",
        "asshat",
        "shithole",
        "clusterfuck",
        "arse",
        "arsehole",
        "motherfucker",
        "motherfucking",
        "motherfuckers",
        "bullshit",
        "bullshitting",
        "goddamn",
        "goddamnit",
        "dammit",
        "whore",
        "whores",
        "slut",
        "sluts",
        "slutty",
        "piss",
        "pissed",
        "pissing",
        "pisses",
        "wtf",
        "stfu",
        // Whisper phonetic / censored-audio transcription variants
        "effing",    // Whisper hears bleeps as "effing" for "fucking"
        "effin",
        "friggin",
        "frigging",
        "fricking",
        // NOTE: "hell" causes false positives (shell, hello, etc.) - only match specific phrases
        "what the hell",
        "go to hell",
        "hell yeah",
        "hell no",
        "as hell",
        "like hell",
        "douche",
        "douchebag",
        "jackass",
        "jackasses",
        "twat",
        "prick",
        "pricks",
        "wanker",
        "wank",
        "bollocks",
        "bollocks",
        "bugger",
        "crap",
        "crappy",
        "shag",
        "shagging",
        "tits",
        "tit",
        "titties",
        "boobs",
        "boob",
        "balls",
        "ballsack",
        "screw",
        "screwed",
        "screwing",
        // Racial slurs — n-word and variants
        "niggers",
        "nigger",
        "niggas",
        "nigga",
        "nigg",
        "n-word",
        "nig",       // Abbreviated
        "negro",
        "negroes",
        "negros",    // Common misspelling
        // Other racial/ethnic slurs
        "chink",
        "chinks",
        "gook",
        "gooks",
        "spic",
        "spics",
        "wetback",
        "wetbacks",
        "kike",
        "kikes",
        "cracker",
        "crackers",
        "beaner",
        "beaners",
        "raghead",
        "ragheads",
        "towelhead",
        "towelheads",
        "sandnigger",
        "sandniggers",
        "zipperhead",
        "zipperheads",
        "coon",
        "coons",
        "jigaboo",
        "jigaboos",
        "porch monkey",
        "jungle bunny",
        // Homophobic / transphobic slurs (ONLY actual slurs, NOT identity terms)
        "faggot",
        "faggots",
        "fag",
        "fags",
        "faggy",
        "dyke",
        "dykes",
        "tranny",
        "trannies",
        "shemale",
        "shemales",
        // "gay" added per user request — beep when used as slur/insult in context
        "gay",
        // NOTE: "lesbian", "queer", "homo", "homosexual" are identity terms, NOT slurs
        // Violent/threatening language (REMOVED - too many false positives in gaming context)
        // "kill", "murder", "die" are common gaming terms and cause too many false positives
        // Ableist slurs
        "retard",
        "retarded",
        "retards",
        "retardation",
        "spastic",
        "spaz",
        "midget",
        "midgets",
        "cripple",
        "crippled",
        "mongoloid",
        // NOTE: Proper names like "George Floyd" should NEVER be in a profanity list
        // These were removed as they are offensive to include
    ]
}

/// Words that must use exact word-boundary matching to avoid false positives.
/// e.g. "ass" would match "assign"/"assets" with prefix matching.
fn needs_exact_match(word: &str) -> bool {
    matches!(word.to_lowercase().as_str(), "ass" | "tit" | "crap" | "balls" | "prick" | "cock" | "gay" | "fag" | "nig")
}

pub fn word_boundary_match(text: &str, bad_word: &str) -> bool {
    let bad_lower = bad_word.to_lowercase();

    // First, try exact regex matching with word boundaries
    let escaped = regex::escape(bad_word);
    let pattern = if bad_word.contains(' ') || needs_exact_match(bad_word) {
        // Multi-word phrases and exact-match words need strict word boundaries
        format!(r"(?i)\b{}\b", escaped)
    } else {
        // Single words can match as prefix (e.g., "fuck" matches "fucking")
        format!(r"(?i)\b{}\w*", escaped)
    };

    if let Ok(re) = regex::Regex::new(&pattern) {
        if re.is_match(text) {
            return true;
        }
    }

    // Enhanced: Also check for asterisk-censored versions (e.g., "f***", "sh*t")
    // Whisper sometimes transcribes censored audio as asterisks
    if bad_lower.len() >= 3 {
        let first_char = bad_lower.chars().next().unwrap();
        let last_char = bad_lower.chars().last().unwrap();

        // Match patterns like "f***" or "f**k" for "fuck"
        let asterisk_pattern = format!(r"(?i)\b{}[\*]+{}?\b",
            regex::escape(&first_char.to_string()),
            regex::escape(&last_char.to_string()));

        if let Ok(re) = regex::Regex::new(&asterisk_pattern) {
            if re.is_match(text) {
                return true;
            }
        }
    }

    // REMOVED fallback to fuzzy contains matching to prevent false positives
    // like "hell" matching "shell" or "ass" matching "passionate"
    // If the regex didn't match, the word isn't there
    false
}

/// Get precise word-level timestamps for profanity censoring.
/// First tries to use word-level timestamps from the transcript (if available from Groq API),
/// then falls back to estimation using linear interpolation across the words in the segment.
/// Returns a list of `(start_secs, end_secs)` pairs for all occurrences.
pub fn estimate_word_timestamps(
    seg: &crate::agent::transcription::TranscriptSegment,
    bad_word: &str,
) -> Vec<(f64, f64)> {
    let mut occurrences = Vec::new();

    // Strategy 1: Use word-level timestamps if available (from Groq API)
    if !seg.words.is_empty() {
        info!("[CENSOR] Using word-level timestamps for segment {:.2}s-{:.2}s", seg.start, seg.end);
        for word_ts in &seg.words {
            if word_boundary_match(&word_ts.word, bad_word) {
                // Use actual word timestamps with minimal padding
                let pre_pad = 0.05_f64;  // 50ms lead (precise timing)
                let post_pad = 0.05_f64; // 50ms trail

                let beep_start = (word_ts.start - pre_pad).max(seg.start);
                let beep_end = (word_ts.end + post_pad).min(seg.end);

                info!(
                    "[CENSOR] ✓ Exact match '{}' → beep {:.2}s-{:.2}s (word: {:.2}s-{:.2}s, lead: {:.2}s)",
                    bad_word, beep_start, beep_end, word_ts.start, word_ts.end, word_ts.start - beep_start
                );

                occurrences.push((beep_start, beep_end));
            }
        }
        if !occurrences.is_empty() {
            return occurrences;
        }
    }

    // Strategy 2: Fall back to estimation (for local Whisper or SRT files)
    info!("[CENSOR] No word-level timestamps, using estimation for segment {:.2}s-{:.2}s", seg.start, seg.end);
    let words: Vec<&str> = seg.text.split_whitespace().collect();
    let seg_dur = (seg.end - seg.start).max(0.001);

    // Char-count ratio for estimating word position within the segment.
    let char_lengths: Vec<usize> = words.iter()
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).count().max(1))
        .collect();
    let total_chars: usize = char_lengths.iter().sum::<usize>().max(1);

    let mut char_offset = 0usize;
    for (i, word) in words.iter().enumerate() {
        if word_boundary_match(word, bad_word) {
            let start_ratio = char_offset as f64 / total_chars as f64;
            let end_ratio = (char_offset + char_lengths[i]) as f64 / total_chars as f64;

            // Whisper segments often have leading AND trailing silence, and mid-sentence
            // pauses. Estimation error is typically ±300ms. Strategy: use segment
            // length to decide how wide the beep window should be, and bias the window
            // toward covering "after" the estimate since pauses push words later.
            //
            // Short segment (<3s):  beep the whole segment — no point estimating
            // Medium (3-6s):        1.0s window, slight forward bias
            // Long (>6s):           1.4s window, stronger forward bias (more pause accumulation)
            let (pre_pad, post_pad, lag) = if seg_dur < 3.0 {
                (seg_dur * start_ratio, seg_dur * (1.0 - end_ratio), 0.0)
            } else if seg_dur < 6.0 {
                (0.15, 0.65, 0.10)
            } else {
                (0.15, 0.85, 0.20)
            };

            let estimated_center = seg.start + start_ratio * seg_dur + lag;
            let beep_start = (estimated_center - pre_pad).max(seg.start);
            let beep_end = (seg.start + end_ratio * seg_dur + lag + post_pad)
                .min(seg.end);

            info!(
                "[CENSOR] ~ Est '{}' seg={:.2}-{:.2}({:.1}s) ratio={:.2}-{:.2} → beep {:.2}-{:.2}",
                bad_word, seg.start, seg.end, seg_dur, start_ratio, end_ratio, beep_start, beep_end
            );

            occurrences.push((beep_start, beep_end));
        }
        char_offset += char_lengths[i];
    }

    // Fallback: multi-word phrase matched segment text but no individual word matched.
    if occurrences.is_empty() && word_boundary_match(&seg.text, bad_word) {
        let text_lower = seg.text.to_lowercase();
        let phrase_lower = bad_word.to_lowercase();
        let char_pos = text_lower.find(&phrase_lower).unwrap_or(0);
        let text_len = seg.text.len().max(1);
        let phrase_len = bad_word.len();
        let start_ratio = char_pos as f64 / text_len as f64;
        let end_ratio = (char_pos + phrase_len) as f64 / text_len as f64;
        let (pre_pad, post_pad, lag) = if seg_dur < 3.0 {
            (0.10, 0.30, 0.0)
        } else if seg_dur < 6.0 {
            (0.15, 0.65, 0.10)
        } else {
            (0.15, 0.85, 0.20)
        };
        let phrase_start = seg.start + start_ratio * seg_dur + lag;
        let phrase_end = seg.start + end_ratio * seg_dur + lag;
        let beep_start = (phrase_start - pre_pad).max(seg.start);
        let beep_end = (phrase_end + post_pad).min(seg.end);
        info!(
            "[CENSOR] ~ Phrase '{}' in segment {:.2}s-{:.2}s → beep {:.2}s-{:.2}s (phrase estimate)",
            bad_word, seg.start, seg.end, beep_start, beep_end
        );
        occurrences.push((beep_start, beep_end));
    }

    occurrences
}

