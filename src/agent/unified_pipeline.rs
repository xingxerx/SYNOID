// SYNOID Unified Pipeline - GPU-Accelerated Processing Orchestrator
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Combines all processing stages into a single, GPU-accelerated pipeline.

use crate::agent::production_tools::safe_arg_path;
use crate::gpu_backend::{get_gpu_context, GpuBackend, GpuContext};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tracing::{info, warn};

/// Pipeline stages that can be executed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    Download,   // Download from YouTube/URL
    Transcribe, // Speech-to-text transcription
    SmartEdit,  // Intent-based smart editing
    Vectorize,  // Convert to vector graphics
    Upscale,    // Upscale via vector rendering
    Enhance,    // Audio enhancement
    Encode,     // Final video encoding
    VoiceTts,   // Text-to-speech synthesis
}

impl PipelineStage {
    /// Parse stage from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "download" => Some(Self::Download),
            "transcribe" => Some(Self::Transcribe),
            "smart_edit" | "smartedit" | "edit" => Some(Self::SmartEdit),
            "vectorize" | "vector" => Some(Self::Vectorize),
            "upscale" => Some(Self::Upscale),
            "enhance" | "audio" => Some(Self::Enhance),
            "encode" | "render" => Some(Self::Encode),
            "voice" | "tts" | "voice_tts" => Some(Self::VoiceTts),
            _ => None,
        }
    }

    /// Parse comma-separated stage list
    pub fn parse_list(s: &str) -> Vec<Self> {
        if s.to_lowercase() == "all" {
            return vec![
                Self::Transcribe,
                Self::SmartEdit,
                Self::Enhance,
                Self::Encode,
            ];
        }

        s.split(',')
            .filter_map(|part| Self::from_str(part.trim()))
            .collect()
    }
}

/// Configuration for pipeline execution
pub struct PipelineConfig {
    /// Stages to execute
    pub stages: Vec<PipelineStage>,
    /// User intent for smart editing
    pub intent: Option<String>,
    /// Scale factor for upscaling
    pub scale_factor: f64,
    /// Target size in MB for compression (0 = no compression)
    pub target_size_mb: f64,
    /// Enable Funny Mode (commentary + transitions)
    pub funny_mode: bool,
    /// Progress callback
    pub progress_callback: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            stages: vec![PipelineStage::Encode],
            intent: None,
            scale_factor: 2.0,
            target_size_mb: 0.0,
            funny_mode: false,
            progress_callback: None,
        }
    }
}

/// Unified processing pipeline
pub struct UnifiedPipeline {
    gpu: &'static GpuContext,
}

impl UnifiedPipeline {
    /// Create new pipeline with GPU context
    pub async fn new() -> Self {
        let gpu = get_gpu_context().await;
        info!("[PIPELINE] Initialized with backend: {}", gpu.backend);
        Self { gpu }
    }

    /// Execute the full pipeline
    pub async fn process(
        &self,
        input: &Path,
        output: &Path,
        config: PipelineConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut current_input = input.to_path_buf();
        let work_dir = input
            .parent()
            .unwrap_or(Path::new("."))
            .join(".synoid_work");
        std::fs::create_dir_all(&work_dir)?;

        self.report_progress(
            &config,
            &format!("Starting pipeline with {} stages", config.stages.len()),
        );
        self.report_progress(&config, &format!("GPU Backend: {}", self.gpu.backend));

        for (i, stage) in config.stages.iter().enumerate() {
            let stage_output = work_dir.join(format!("stage_{:02}_{:?}.mp4", i, stage));

            self.report_progress(
                &config,
                &format!("Stage {}/{}: {:?}", i + 1, config.stages.len(), stage),
            );

            match stage {
                PipelineStage::Transcribe => {
                    // Transcription doesn't modify video, just extracts data
                    self.run_transcribe(&current_input, &config).await?;
                }
                PipelineStage::SmartEdit => {
                    if let Some(ref intent) = config.intent {
                        current_input = self
                            .run_smart_edit(&current_input, &stage_output, intent, &config)
                            .await?;
                    } else {
                        warn!("[PIPELINE] SmartEdit skipped: no intent provided");
                    }
                }
                PipelineStage::Vectorize => {
                    current_input = self
                        .run_vectorize(&current_input, &stage_output, &config)
                        .await?;
                }
                PipelineStage::Upscale => {
                    current_input = self
                        .run_upscale(&current_input, &stage_output, config.scale_factor, &config)
                        .await?;
                }
                PipelineStage::Enhance => {
                    current_input = self
                        .run_enhance(&current_input, &stage_output, &config)
                        .await?;
                }
                PipelineStage::Encode => {
                    current_input = self
                        .run_encode(&current_input, &stage_output, &config)
                        .await?;
                }
                _ => {
                    info!("[PIPELINE] Stage {:?} not yet implemented", stage);
                }
            }
        }

        // Move final output
        std::fs::copy(&current_input, output)?;

        // Cleanup work directory
        if let Err(e) = std::fs::remove_dir_all(&work_dir) {
            warn!("[PIPELINE] Cleanup warning: {}", e);
        }

        self.report_progress(&config, "Pipeline complete!");
        Ok(output.to_path_buf())
    }

    fn report_progress(&self, config: &PipelineConfig, msg: &str) {
        info!("[PIPELINE] {}", msg);
        if let Some(ref callback) = config.progress_callback {
            callback(msg);
        }
    }

    async fn run_transcribe(
        &self,
        input: &Path,
        config: &PipelineConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::agent::voice::transcription::TranscriptionEngine;

        self.report_progress(config, "Transcribing audio...");

        let engine = TranscriptionEngine::new()?;
        let segments = engine.transcribe(input).await?;

        self.report_progress(config, &format!("Transcribed {} segments", segments.len()));
        Ok(())
    }

    async fn run_smart_edit(
        &self,
        input: &Path,
        output: &Path,
        intent: &str,
        config: &PipelineConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        use crate::agent::smart_editor;

        self.report_progress(config, &format!("Smart editing: {}", intent));

        let progress_cb = config.progress_callback.clone();
        // Explicitly cast to the type expected by smart_edit (Send + Sync)
        let callback: Option<Box<dyn Fn(&str) + Send + Sync>> = progress_cb
            .map(|cb| Box::new(move |msg: &str| cb(msg)) as Box<dyn Fn(&str) + Send + Sync>);

        smart_editor::smart_edit(input, intent, output, config.funny_mode, callback).await?;

        Ok(output.to_path_buf())
    }

    async fn run_vectorize(
        &self,
        input: &Path,
        output: &Path,
        config: &PipelineConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        use crate::agent::vector_engine::{vectorize_video, VectorConfig};

        self.report_progress(config, "Vectorizing frames...");

        let vector_config = VectorConfig::default();
        let output_dir = output.parent().unwrap().join("vectors");

        vectorize_video(input, &output_dir, vector_config).await?;

        // Vector output is SVG directory, not video - return input for now
        // In a full implementation, we'd reassemble the video
        Ok(input.to_path_buf())
    }

    async fn run_upscale(
        &self,
        input: &Path,
        output: &Path,
        scale_factor: f64,
        config: &PipelineConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        use crate::agent::vector_engine::upscale_video;

        self.report_progress(config, &format!("Upscaling {}x...", scale_factor));

        upscale_video(input, scale_factor, output).await?;

        Ok(output.to_path_buf())
    }

    async fn run_enhance(
        &self,
        input: &Path,
        output: &Path,
        config: &PipelineConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        use crate::agent::production_tools::enhance_audio;

        self.report_progress(config, "Enhancing audio...");

        // Extract audio, enhance, and remux
        let audio_path = output.with_extension("wav");
        enhance_audio(input, &audio_path).await?;

        // Remux with enhanced audio using GPU encoder
        let encoder = self.gpu.ffmpeg_encoder();
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-y", "-nostdin"]);

        // Add hardware acceleration if available
        if let Some(hwaccel) = self.gpu.ffmpeg_hwaccel() {
            cmd.args(["-hwaccel", hwaccel]);
        }

        cmd.arg("-i")
            .arg(safe_arg_path(input))
            .arg("-i")
            .arg(safe_arg_path(&audio_path))
            .args(["-map", "0:v:0", "-map", "1:a:0"])
            .arg("-c:v")
            .arg(encoder)
            .args(["-c:a", "aac", "-b:a", "192k"])
            .arg(safe_arg_path(output));

        let status = cmd.status().await?;
        if !status.success() {
            return Err("Audio remux failed".into());
        }

        // Cleanup temp audio
        let _ = std::fs::remove_file(&audio_path);

        Ok(output.to_path_buf())
    }

    async fn run_encode(
        &self,
        input: &Path,
        output: &Path,
        config: &PipelineConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        self.report_progress(
            config,
            &format!("Encoding with {}...", self.gpu.ffmpeg_encoder()),
        );

        // let encoder = self.gpu.ffmpeg_encoder();
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-y", "-nostdin"]);

        // Add hardware acceleration for decoding if available
        if let Some(hwaccel) = self.gpu.ffmpeg_hwaccel() {
            cmd.args(["-hwaccel", hwaccel]);
        }

        cmd.arg("-i").arg(safe_arg_path(input));

        // Configure encoder based on backend
        match &self.gpu.backend {
            GpuBackend::NvencGpu { .. } => {
                cmd.args([
                    "-c:v",
                    "h264_nvenc",
                    "-preset",
                    "p4", // Quality/speed balance
                    "-rc",
                    "vbr", // Variable bitrate
                    "-cq",
                    "23", // Quality level
                    "-b:v",
                    "0", // Let CQ control bitrate
                ]);
            }
            GpuBackend::Cpu { .. } => {
                cmd.args(["-c:v", "libx264", "-preset", "medium", "-crf", "23"]);
            }
        }

        cmd.args(["-c:a", "aac", "-b:a", "192k"])
            .arg(safe_arg_path(output));

        let status = cmd.status().await?;
        if !status.success() {
            return Err("GPU encoding failed".into());
        }

        Ok(output.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_parsing() {
        let stages = PipelineStage::parse_list("vectorize,upscale,encode");
        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0], PipelineStage::Vectorize);
        assert_eq!(stages[1], PipelineStage::Upscale);
        assert_eq!(stages[2], PipelineStage::Encode);
    }

    #[test]
    fn test_all_stages() {
        let stages = PipelineStage::parse_list("all");
        assert!(stages.len() >= 3);
    }
}
