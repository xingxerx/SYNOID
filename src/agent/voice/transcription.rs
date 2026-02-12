// SYNOID Sovereign Ear
// Native Rust implementation of Whisper for local, private transcription.

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
}

pub struct TranscriptionEngine {
    model_path: PathBuf,
}

impl TranscriptionEngine {
    pub async fn new(model_name: Option<String>) -> Result<Self> {
        let model_name = model_name.unwrap_or_else(|| "base.en".to_string());

        // Locate or download the model in blocking task
        let model_path = tokio::task::spawn_blocking(move || {
            Self::ensure_model(&model_name)
        }).await??;

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

        // 1. Prepare Audio
        // Running CPU-heavy audio processing in blocking thread
        let audio_path_buf = audio_path.to_path_buf();
        let model_path = self.model_path.clone();

        let segments = tokio::task::spawn_blocking(move || {
            Self::transcribe_blocking(&model_path, &audio_path_buf)
        })
        .await??;

        info!(
            "[SOVEREIGN] Transcription Complete: {} segments.",
            segments.len()
        );
        Ok(segments)
    }

    fn transcribe_blocking(model_path: &Path, audio_path: &Path) -> Result<Vec<TranscriptSegment>> {
        // Read audio
        let mut reader = hound::WavReader::open(audio_path).context("Open WAV")?;
        let spec = reader.spec();
        let samples: Vec<i16> = reader.samples().filter_map(|s| s.ok()).collect();

        // Manual conversion and resampling
        let mut pcm_data: Vec<f32> = Vec::new();
        let channels = spec.channels as usize;

        // Convert to float and mix to mono
        for chunk in samples.chunks(channels) {
            let sum: f32 = chunk.iter().map(|&s| s as f32).sum();
            pcm_data.push((sum / channels as f32) / 32768.0);
        }

        // Resample if needed (Naive linear)
        if spec.sample_rate != 16000 {
            let ratio = 16000.0 / spec.sample_rate as f32;
            let new_len = (pcm_data.len() as f32 * ratio) as usize;
            let mut resampled = Vec::with_capacity(new_len);
            for i in 0..new_len {
                let src_idx = (i as f32 / ratio) as usize;
                if src_idx < pcm_data.len() {
                    resampled.push(pcm_data[src_idx]);
                }
            }
            pcm_data = resampled;
        }

        // Initialize Whisper
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().unwrap(),
            WhisperContextParameters::default(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to load model: {:?}", e))?;

        let mut state = ctx.create_state().context("Create state")?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

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
            });
        }

        Ok(segments)
    }
}
