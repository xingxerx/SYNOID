// SYNOID Sovereign Ear
// Native Rust implementation of Whisper for local, private transcription.

use crate::agent::engines::process_utils::CommandExt;
use crate::gpu_backend::get_gpu_context;
use anyhow::{Context, Result};
use hf_hub::api::sync::Api;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    #[serde(default)]
    pub words: Vec<WordTimestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTimestamp {
    pub word: String,
    pub start: f64,
    pub end: f64,
}

pub struct TranscriptionEngine {
    model_path: PathBuf,
}

impl TranscriptionEngine {
    pub async fn new(model_name: Option<String>) -> Result<Self> {
        let model_name = model_name.unwrap_or_else(|| "base.en".to_string());

        // Locate or download the model in blocking task
        let model_path =
            tokio::task::spawn_blocking(move || Self::ensure_model(&model_name)).await??;

        Ok(Self { model_path })
    }

    /// Ensure the GGML model is present (Sovereign Ear - ModelDownloader)
    fn ensure_model(model_name: &str) -> Result<PathBuf> {
        // Use environment variable for cache dir if available
        let base_dir = if let Ok(cache_env) = std::env::var("SYNOID_CACHE_DIR") {
            PathBuf::from(cache_env).join("models")
        } else {
            dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("synoid")
                .join("models")
        };

        fs::create_dir_all(&base_dir)?;

        let filename = format!("ggml-{}.bin", model_name);
        let model_path = base_dir.join(&filename);

        if model_path.exists() {
            info!("[SOVEREIGN] Found cached Whisper model: {:?}", model_path);
            return Ok(model_path);
        }

        info!("[SOVEREIGN] Downloading Whisper model: {}...", filename);

        // Use hf-hub to fetch from ggerganov/whisper.cpp
        let api = Api::new()?;
        let repo = api.model("ggerganov/whisper.cpp".to_string());
        let downloaded_path = repo.get(&filename)?;

        // Copy/Move to our cache location for persistence/control
        fs::copy(&downloaded_path, &model_path)?;

        info!("[SOVEREIGN] Model secured: {:?}", model_path);
        Ok(model_path)
    }

    pub async fn transcribe(&self, audio_path: &Path) -> Result<Vec<TranscriptSegment>> {
        info!("[SOVEREIGN] Transcribing: {:?}", audio_path);

        // 1. Try Cloud Transcription (Groq Whisper) for highest accuracy and speed
        let agent = crate::agent::gpt_oss_bridge::SynoidAgent::new(
            "http://localhost:11434",
            "default",
        );

        if let Ok(json_str) = agent.transcribe_audio(audio_path).await {
            info!("[TRANSCRIBE] AI Transcription Succeeded.");
            let json_string: String = json_str;
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_string) {
                if let Some(segments_arr) = val.get("segments").and_then(|v| v.as_array()) {
                    let mut segments = Vec::new();
                    for seg in segments_arr {
                        if let (Some(start), Some(end), Some(text)) = (
                            seg.get("start").and_then(|v| v.as_f64()),
                            seg.get("end").and_then(|v| v.as_f64()),
                            seg.get("text").and_then(|v| v.as_str()),
                        ) {
                            // Extract word-level timestamps if available
                            let mut words = Vec::new();
                            if let Some(words_arr) = seg.get("words").and_then(|v| v.as_array()) {
                                for word_obj in words_arr {
                                    if let (Some(word), Some(w_start), Some(w_end)) = (
                                        word_obj.get("word").and_then(|v| v.as_str()),
                                        word_obj.get("start").and_then(|v| v.as_f64()),
                                        word_obj.get("end").and_then(|v| v.as_f64()),
                                    ) {
                                        words.push(WordTimestamp {
                                            word: word.to_string(),
                                            start: w_start,
                                            end: w_end,
                                        });
                                    }
                                }
                            }

                            segments.push(TranscriptSegment {
                                start,
                                end,
                                text: text.to_string(),
                                words,
                            });
                        }
                    }

                    if !segments.is_empty() {
                        let segments = filter_hallucinations(segments);
                        let word_count: usize = segments.iter().map(|s| s.words.len()).sum();
                        info!("[SOVEREIGN] ☁️ Cloud Transcription Complete: {} segments, {} word-level timestamps (via Groq Whisper). Subtitles enhanced.", segments.len(), word_count);
                        return Ok(segments);
                    }
                }
            }
        }

        info!("[SOVEREIGN] ⚠️ Cloud transcription unavailable or failed. Falling back to local Sovereign Ear.");

        // Check for GPU availability
        let gpu = get_gpu_context().await;
        let use_gpu = gpu.has_gpu();

        if use_gpu {
            info!("[SOVEREIGN] 🚀 GPU Acceleration ENABLED for Whisper");
        } else {
            info!("[SOVEREIGN] 🐌 Using CPU for transcription");
        }

        // 1. Prepare Audio
        // Running CPU-heavy audio processing in blocking thread
        let audio_path_buf = audio_path.to_path_buf();
        let model_path = self.model_path.clone();

        let segments = tokio::task::spawn_blocking(move || {
            Self::transcribe_blocking(&model_path, &audio_path_buf, use_gpu)
        })
        .await??;

        info!(
            "[SOVEREIGN] Local Transcription Complete: {} segments.",
            segments.len()
        );
        Ok(segments)
    }

    fn transcribe_blocking(
        model_path: &Path,
        audio_path: &Path,
        use_gpu: bool,
    ) -> Result<Vec<TranscriptSegment>> {
        // Read audio
        let mut reader = hound::WavReader::open(audio_path).context("Open WAV")?;
        let spec = reader.spec();

        let mut pcm_data: Vec<f32>;

        let is_16k_mono = spec.sample_rate == 16000 && spec.channels == 1;

        if is_16k_mono {
            info!("[SOVEREIGN] 🎧 Native 16kHz mono detected. Fast-path memory loading...");
            // Pre-allocate for exactly the number of samples
            pcm_data = Vec::with_capacity(reader.duration() as usize);

            // Read directly into f32 vec
            for sample in reader.samples::<i16>() {
                if let Ok(s) = sample {
                    pcm_data.push((s as f32) / 32768.0);
                }
            }
        } else {
            info!("[SOVEREIGN] 🐌 Downmixingxerx/resampling in memory. (Channels: {}, Rate: {}). This uses significant RAM.", spec.channels, spec.sample_rate);

            // Manual conversion and downmix to mono simultaneously
            let channels = spec.channels as usize;
            let mut f32_samples = Vec::with_capacity((reader.duration() as usize) / channels);
            let mut sample_iter = reader.samples::<i16>();

            while let Some(Ok(first_sample)) = sample_iter.next() {
                let mut sum = first_sample as f32;
                // Accumulate other channels
                for _ in 1..channels {
                    if let Some(Ok(s)) = sample_iter.next() {
                        sum += s as f32;
                    }
                }
                f32_samples.push((sum / channels as f32) / 32768.0);
            }

            // Resample if needed (Naive linear)
            if spec.sample_rate != 16000 {
                let ratio = 16000.0 / spec.sample_rate as f32;
                let new_len = (f32_samples.len() as f32 * ratio) as usize;
                pcm_data = Vec::with_capacity(new_len);
                for i in 0..new_len {
                    let src_idx = (i as f32 / ratio) as usize;
                    if src_idx < f32_samples.len() {
                        pcm_data.push(f32_samples[src_idx]);
                    }
                }
            } else {
                pcm_data = f32_samples;
            }
        }

        // Initialize Whisper with GPU parameters if requested
        let params = WhisperContextParameters {
            use_gpu,
            ..Default::default()
        };

        let ctx = WhisperContext::new_with_params(model_path.to_str().unwrap(), params)
            .map_err(|e| anyhow::anyhow!("Failed to load model: {:?}", e))?;

        let mut state = ctx.create_state().context("Create state")?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_special(false);
        params.set_no_context(true);
        // Enable progress logging so the user doesn't think the app is frozen
        params.set_print_progress(true);
        params.set_print_realtime(true);
        params.set_print_timestamps(true);

        // Maximize CPU threads (Even with GPU, parts of Whisper run on CPU)
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4) as i32;
        params.set_n_threads(num_threads);

        // Run
        state.full(params, &pcm_data).context("Running inference")?;

        // Extract
        let num_segments = state.full_n_segments().context("Get segments count")?;
        let mut segments = Vec::new();

        for i in 0..num_segments {
            let start = state.full_get_segment_t0(i).unwrap_or(0) as f64 / 100.0; // cs to s
            let end = state.full_get_segment_t1(i).unwrap_or(0) as f64 / 100.0;
            let text = state.full_get_segment_text(i).unwrap_or_default();

            segments.push(TranscriptSegment {
                start,
                end,
                text: text.to_string(),
                words: Vec::new(),
            });
        }

        Ok(filter_hallucinations(segments))
    }
}

/// Detect and strip Whisper hallucination loops.
///
/// Whisper sometimes gets stuck repeating the same phrase for the rest of a
/// transcript. We detect this by scanning for a run of N segments whose
/// normalised text is identical, then truncate everything from the start of
/// that run onward.
pub fn filter_hallucinations(mut segments: Vec<TranscriptSegment>) -> Vec<TranscriptSegment> {
    const MIN_RUN: usize = 5; // consecutive identical lines → hallucination

    if segments.len() < MIN_RUN {
        return segments;
    }

    let normalise = |s: &str| -> String {
        s.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };

    let mut i = 0;
    while i < segments.len().saturating_sub(MIN_RUN) {
        let anchor = normalise(&segments[i].text);
        if anchor.is_empty() {
            i += 1;
            continue;
        }

        let mut run_len = 0;
        for j in i..segments.len() {
            if normalise(&segments[j].text) == anchor {
                run_len += 1;
            } else {
                break;
            }
        }

        if run_len >= MIN_RUN {
            tracing::warn!(
                "[SOVEREIGN] 🚨 Hallucination loop detected at segment {}: \
                 \"{}\" repeated {} times — stripping these specific segments.",
                i,
                segments[i].text.trim(),
                run_len,
            );
            // Drain the repeated hallucinated lines instead of truncating the whole remainder
            segments.drain(i..i + run_len);
        } else {
            i += 1;
        }
    }

    segments
}

pub fn generate_srt(segments: &[TranscriptSegment]) -> String {
    let mut srt_out = String::new();
    for (i, seg) in segments.iter().enumerate() {
        let start = format_srt_time(seg.start);
        let end = format_srt_time(seg.end);
        srt_out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            start,
            end,
            seg.text.trim()
        ));
    }
    srt_out
}

/// Parse an SRT file into TranscriptSegments
/// This allows reusing existing SRT files instead of re-transcribing
pub fn parse_srt(srt_content: &str) -> Result<Vec<TranscriptSegment>> {
    let mut segments = Vec::new();

    // Split by double newlines to get individual subtitle blocks
    let separator = if srt_content.contains("\r\n\r\n") {
        "\r\n\r\n"
    } else {
        "\n\n"
    };

    for block in srt_content.trim().split(separator) {
        let lines: Vec<&str> = block.trim().lines().collect();
        if lines.len() < 3 {
            continue; // Skip malformed blocks
        }

        // Line 0: index (skip)
        // Line 1: timestamp "HH:MM:SS,mmm --> HH:MM:SS,mmm"
        // Lines 2+: text content

        let timestamp_line = lines[1];
        let parts: Vec<&str> = timestamp_line.split(" --> ").collect();
        if parts.len() != 2 {
            continue; // Skip malformed timestamp
        }

        let start = parse_srt_timestamp(parts[0].trim())?;
        let end = parse_srt_timestamp(parts[1].trim())?;
        let text = lines[2..].join("\n");

        segments.push(TranscriptSegment {
            start,
            end,
            text,
            words: Vec::new(), // SRT files don't contain word-level timestamps
        });
    }

    if segments.is_empty() {
        anyhow::bail!("No valid segments found in SRT file");
    }

    Ok(segments)
}

/// Parse SRT timestamp "HH:MM:SS,mmm" into seconds
fn parse_srt_timestamp(timestamp: &str) -> Result<f64> {
    let parts: Vec<&str> = timestamp.split(',').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid timestamp format: {}", timestamp);
    }

    let time_parts: Vec<&str> = parts[0].split(':').collect();
    if time_parts.len() != 3 {
        anyhow::bail!("Invalid time format: {}", parts[0]);
    }

    let hours: f64 = time_parts[0].parse().context("Parse hours")?;
    let minutes: f64 = time_parts[1].parse().context("Parse minutes")?;
    let seconds: f64 = time_parts[2].parse().context("Parse seconds")?;
    let millis: f64 = parts[1].parse().context("Parse milliseconds")?;

    Ok(hours * 3600.0 + minutes * 60.0 + seconds + millis / 1000.0)
}

fn format_srt_time(seconds: f64) -> String {
    let hours = (seconds / 3600.0) as u32;
    let mins = ((seconds % 3600.0) / 60.0) as u32;
    let secs = (seconds % 60.0) as u32;
    let millis = ((seconds.fract()) * 1000.0) as u32;

    format!("{:02}:{:02}:{:02},{:03}", hours, mins, secs, millis)
}

// ─────────────────────────────────────────────────────────────────────────────
// Script-Based Editing (Feature 1)
// Users delete sentences from the transcript; SYNOID converts those removals
// into precise FFmpeg cut-points, mimicking Descript / SYNOID IntelliScript.
// ─────────────────────────────────────────────────────────────────────────────

/// A user-editable view of the transcript that tracks which segments have been
/// marked for removal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEditor {
    /// All original segments, with their keep/delete flag.
    pub segments: Vec<EditableSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditableSegment {
    pub segment: TranscriptSegment,
    /// When `true` this segment will be cut out of the video.
    pub deleted: bool,
}

impl ScriptEditor {
    /// Build a ScriptEditor from a raw transcript.
    pub fn from_transcript(segments: Vec<TranscriptSegment>) -> Self {
        Self {
            segments: segments
                .into_iter()
                .map(|s| EditableSegment {
                    segment: s,
                    deleted: false,
                })
                .collect(),
        }
    }

    /// Mark a segment as deleted by index.
    pub fn delete_segment(&mut self, index: usize) {
        if let Some(seg) = self.segments.get_mut(index) {
            seg.deleted = true;
        }
    }

    /// Restore a previously deleted segment.
    pub fn restore_segment(&mut self, index: usize) {
        if let Some(seg) = self.segments.get_mut(index) {
            seg.deleted = false;
        }
    }

    /// Collect the time-ranges that should be *kept* (inverse of deletions).
    /// Each entry is `(start_secs, end_secs)`.
    pub fn kept_ranges(&self) -> Vec<(f64, f64)> {
        let mut ranges: Vec<(f64, f64)> = Vec::new();

        for seg in &self.segments {
            if seg.deleted {
                continue;
            }
            let s = seg.segment.start;
            let e = seg.segment.end;
            // Merge with previous range if contiguous (gap < 0.05 s)
            if let Some(last) = ranges.last_mut() {
                if s - last.1 < 0.05 {
                    last.1 = e;
                    continue;
                }
            }
            ranges.push((s, e));
        }

        ranges
    }

    /// Build an FFmpeg concat-demuxer script that keeps only the un-deleted
    /// segments.  Returns the script text.
    pub fn build_ffmpeg_concat_script(&self, input_path: &std::path::Path) -> String {
        let mut script = String::new();
        for (start, end) in self.kept_ranges() {
            script.push_str(&format!(
                "file '{}'\ninpoint {:.6}\noutpoint {:.6}\n",
                input_path.display(),
                start,
                end,
            ));
        }
        script
    }

    /// Execute the script-driven edit: writes a temp concat file, runs FFmpeg,
    /// and saves the result to `output_path`.
    pub async fn apply_edits(
        &self,
        input_path: &std::path::Path,
        output_path: &std::path::Path,
    ) -> Result<()> {
        use tokio::process::Command;

        let concat_script = self.build_ffmpeg_concat_script(input_path);
        if concat_script.is_empty() {
            anyhow::bail!("All segments are deleted – nothing to keep.");
        }

        // Write the concat script to a temp file
        let tmp_dir = std::env::temp_dir();
        let concat_file = tmp_dir.join(format!("synoid_concat_{}.txt", uuid_simple()));
        std::fs::write(&concat_file, &concat_script).context("Writing concat script")?;

        info!(
            "[SCRIPT-EDITOR] Applying {} kept ranges → {:?}",
            self.kept_ranges().len(),
            output_path
        );

        let status = Command::new("ffmpeg")
            .stealth()
            .args(["-y", "-f", "concat", "-safe", "0", "-i"])
            .arg(&concat_file)
            .args(["-c", "copy"])
            .arg(output_path)
            .status()
            .await
            .context("Launching FFmpeg for script edit")?;

        let _ = std::fs::remove_file(&concat_file);

        if !status.success() {
            anyhow::bail!("FFmpeg script-edit failed with status: {}", status);
        }

        info!(
            "[SCRIPT-EDITOR] Script-based edit complete: {:?}",
            output_path
        );
        Ok(())
    }
}

/// Generate a short random hex string for temp file names (no external crate needed).
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{:x}", t)
}
