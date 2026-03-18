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
        .stealth()
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
pub fn generate_srt_for_kept_scenes(
    transcript: &[crate::agent::transcription::TranscriptSegment],
    kept_scenes: &[Scene],
) -> String {
    const MIN_DISPLAY_SECS: f64 = 2.5; // Minimum subtitle display time (increased for readability)
    const MERGE_THRESHOLD_SECS: f64 = 1.2; // Merge entries shorter than this into prev (adjusted)

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
        "fucked",
        "fucker",
        "fucks",
        "fuckhead",
        "fuckface",
        "shit",
        "shitty",
        "shitting",
        "shithead",
        "shitface",
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
        "ass",
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
        "hell",
        "hells",
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
        "negro",
        "negroes",
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
        // Homophobic / transphobic slurs
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
        "homo",
        "homos",
        "queer",
        "queers",
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
        "george floyd",
        "georgefloyd",
        "floyd",
    ]
}

pub fn word_boundary_match(text: &str, bad_word: &str) -> bool {
    let escaped = regex::escape(bad_word);
    // For single words without spaces, we allow them to be a prefix of a longer word (e.g., "fuck" matches "fucking").
    // For phrases with spaces, we require word boundaries.
    let pattern = if bad_word.contains(' ') {
        format!(r"(?i)\b{}\b", escaped)
    } else {
        format!(r"(?i)\b{}\w*", escaped)
    };

    if let Ok(re) = regex::Regex::new(&pattern) {
        re.is_match(text)
    } else {
        // Fallback to simple contains if regex fails
        text.to_lowercase().contains(&bad_word.to_lowercase())
    }
}

/// Estimate a narrow (word-level) timestamp within a transcript segment for a
/// given bad word. Uses linear interpolation across the words in the segment.
/// Returns a list of `(start_secs, end_secs)` pairs for all occurrences.
///
/// TIMING FIX: Beep starts slightly BEFORE the word to ensure it covers the
/// entire spoken word, with extended padding to handle speech recognition delays.
pub fn estimate_word_timestamps(
    seg: &crate::agent::transcription::TranscriptSegment,
    bad_word: &str,
) -> Vec<(f64, f64)> {
    let mut occurrences = Vec::new();
    let words: Vec<&str> = seg.text.split_whitespace().collect();
    let n = words.len().max(1) as f64;
    let seg_dur = (seg.end - seg.start).max(0.001);

    // Extended padding: start beep 150ms BEFORE estimated word position
    // and extend 100ms after to ensure complete coverage
    let pre_pad = 0.15_f64;  // 150 ms lead time
    let post_pad = 0.10_f64; // 100 ms trail time

    for (i, word) in words.iter().enumerate() {
        if word_boundary_match(word, bad_word) {
            // Calculate word boundaries with extended padding
            let estimated_word_start = seg.start + (i as f64 / n) * seg_dur;
            let estimated_word_end = seg.start + ((i + 1) as f64 / n) * seg_dur;

            // Start beep BEFORE the word, end AFTER the word
            let beep_start = (estimated_word_start - pre_pad).max(seg.start);
            let beep_end = (estimated_word_end + post_pad).min(seg.end);

            occurrences.push((beep_start, beep_end));
        }
    }

    // Fallback: if individual words didn't match (e.g. multi-word phrase)
    // but the whole segment does, return the whole segment.
    if occurrences.is_empty() && word_boundary_match(&seg.text, bad_word) {
        occurrences.push((seg.start, seg.end));
    }

    occurrences
}

