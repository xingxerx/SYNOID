// SYNOID Unified Pipeline - GPU-Accelerated Processing Orchestrator
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::production_tools::safe_arg_path;
use crate::gpu_backend::{get_gpu_context, GpuBackend, GpuContext};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tracing::{info, warn};

/// Pipeline stages that can be executed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    SmartEdit, // Intent-based smart editing
    Enhance,   // Audio enhancement
    Encode,    // Final video encoding
}

impl PipelineStage {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "smart_edit" | "smartedit" | "edit" => Some(Self::SmartEdit),
            "enhance" | "audio" => Some(Self::Enhance),
            "encode" | "render" => Some(Self::Encode),
            _ => None,
        }
    }

    pub fn parse_list(s: &str) -> Vec<Self> {
        if s.to_lowercase() == "all" {
            return vec![Self::SmartEdit, Self::Enhance, Self::Encode];
        }

        s.split(',')
            .filter_map(|part| Self::from_str(part.trim()))
            .collect()
    }
}

/// Configuration for pipeline execution
pub struct PipelineConfig {
    pub stages: Vec<PipelineStage>,
    pub intent: Option<String>,
    pub target_size_mb: f64,
    pub progress_callback: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            stages: vec![PipelineStage::Encode],
            intent: None,
            target_size_mb: 0.0,
            progress_callback: None,
        }
    }
}

/// Unified processing pipeline
pub struct UnifiedPipeline {
    gpu: &'static GpuContext,
}

impl UnifiedPipeline {
    pub async fn new() -> Self {
        let gpu = get_gpu_context().await;
        info!("[PIPELINE] Initialized with backend: {}", gpu.backend);
        Self { gpu }
    }

    pub async fn process(
        &self,
        input: &Path,
        output: &Path,
        config: PipelineConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let mut current_input = input.to_path_buf();
        let work_dir = input
            .parent()
            .unwrap_or(Path::new("."))
            .join(".synoid_work");
        std::fs::create_dir_all(&work_dir)?;

        self.report_progress(&config, &format!("Starting pipeline with {} stages", config.stages.len()));
        self.report_progress(&config, &format!("GPU Backend: {}", self.gpu.backend));

        for (i, stage) in config.stages.iter().enumerate() {
            let stage_output = work_dir.join(format!("stage_{:02}_{:?}.mp4", i, stage));

            self.report_progress(
                &config,
                &format!("Stage {}/{}: {:?}", i + 1, config.stages.len(), stage),
            );

            match stage {
                PipelineStage::SmartEdit => {
                    if let Some(ref intent) = config.intent {
                        current_input = self
                            .run_smart_edit(&current_input, &stage_output, intent, &config)
                            .await?;
                    } else {
                        warn!("[PIPELINE] SmartEdit skipped: no intent provided");
                    }
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
            }
        }

        std::fs::copy(&current_input, output)?;

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

    async fn run_smart_edit(
        &self,
        input: &Path,
        output: &Path,
        intent: &str,
        config: &PipelineConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        use crate::agent::smart_editor;

        self.report_progress(config, &format!("Smart editing: {}", intent));

        let progress_cb = config.progress_callback.clone();
        let callback: Option<Box<dyn Fn(&str) + Send + Sync>> = progress_cb
            .map(|cb| Box::new(move |msg: &str| cb(msg)) as Box<dyn Fn(&str) + Send + Sync>);

        smart_editor::smart_edit(input, intent, output, callback, None, None).await?;

        Ok(output.to_path_buf())
    }

    async fn run_enhance(
        &self,
        input: &Path,
        output: &Path,
        config: &PipelineConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        use crate::agent::production_tools::enhance_audio;

        self.report_progress(config, "Enhancing audio...");

        let audio_path = output.with_extension("wav");
        enhance_audio(input, &audio_path).await?;

        let encoder = self.gpu.ffmpeg_encoder();
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-y", "-nostdin"]);

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

        let _ = std::fs::remove_file(&audio_path);

        Ok(output.to_path_buf())
    }

    async fn run_encode(
        &self,
        input: &Path,
        output: &Path,
        config: &PipelineConfig,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        self.report_progress(
            config,
            &format!("Encoding with {}...", self.gpu.ffmpeg_encoder()),
        );

        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-y", "-nostdin"]);

        if let Some(hwaccel) = self.gpu.ffmpeg_hwaccel() {
            cmd.args(["-hwaccel", hwaccel]);
        }

        cmd.arg("-i").arg(safe_arg_path(input));

        match &self.gpu.backend {
            GpuBackend::NvencGpu { .. } => {
                cmd.args(["-c:v", "h264_nvenc", "-preset", "p4", "-rc", "vbr", "-cq", "23", "-b:v", "0"]);
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
        let stages = PipelineStage::parse_list("smart_edit,enhance,encode");
        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0], PipelineStage::SmartEdit);
        assert_eq!(stages[1], PipelineStage::Enhance);
        assert_eq!(stages[2], PipelineStage::Encode);
    }

    #[test]
    fn test_all_stages() {
        let stages = PipelineStage::parse_list("all");
        assert_eq!(stages.len(), 3);
    }
}
