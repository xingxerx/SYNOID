// SYNOID CUDA Pipeline Extension
// Extends UnifiedPipeline with CUDA kernel processing capabilities
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID

use super::cuda_kernel_gen::{KernelRequest, OptimizationTarget};
use crate::agent::specialized::synoid_link::SynoidLink;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

/// CUDA-enhanced pipeline configuration
pub struct CudaPipelineConfig {
    pub kernel_request: Option<KernelRequest>,
    pub synoid_link: Option<Arc<SynoidLink>>,
    pub device_id: i32,
}

impl Default for CudaPipelineConfig {
    fn default() -> Self {
        Self {
            kernel_request: None,
            synoid_link: None,
            device_id: 0,
        }
    }
}

/// CUDA Pipeline - High-performance video processing with custom kernels
pub struct CudaPipeline {
    synoid_link: Arc<SynoidLink>,
}

impl CudaPipeline {
    /// Create new CUDA pipeline
    pub fn new(cache_dir: PathBuf, device_id: i32) -> Self {
        let synoid_link = Arc::new(SynoidLink::new(cache_dir, device_id));

        Self { synoid_link }
    }

    /// Set LLM provider for AI-powered kernel generation
    pub fn with_llm(
        mut self,
        provider: Arc<crate::agent::llm_provider::MultiProviderLlm>,
    ) -> Self {
        self.synoid_link = Arc::new(
            Arc::try_unwrap(self.synoid_link)
                .unwrap_or_else(|_arc| SynoidLink::new(PathBuf::from(".synoid_cuda_cache"), 0))
                .with_llm(provider),
        );
        self
    }

    /// Process video with custom CUDA kernel
    pub async fn process_with_kernel(
        &self,
        input: &Path,
        output: &Path,
        request: &KernelRequest,
        progress_callback: Option<Arc<dyn Fn(f64) + Send + Sync>>,
    ) -> Result<()> {
        info!("[CUDA-PIPELINE] Processing video with custom kernel");
        info!("[CUDA-PIPELINE] Intent: {}", request.intent);

        self.synoid_link
            .process_video(input, output, request, progress_callback)
            .await
    }

    /// Apply cinematic color grading
    pub async fn apply_color_grading(
        &self,
        input: &Path,
        output: &Path,
        intensity: f32,
    ) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("intensity".to_string(), intensity);

        // Get video dimensions
        let (width, height) = get_video_dimensions(input).await?;

        let request = KernelRequest {
            intent: "cinematic color grading".to_string(),
            width,
            height,
            params,
            optimization: OptimizationTarget::Quality,
        };

        self.process_with_kernel(input, output, &request, None)
            .await
    }

    /// Apply Gaussian blur
    pub async fn apply_blur(&self, input: &Path, output: &Path, radius: f32) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("radius".to_string(), radius);

        let (width, height) = get_video_dimensions(input).await?;

        let request = KernelRequest {
            intent: "gaussian blur".to_string(),
            width,
            height,
            params,
            optimization: OptimizationTarget::Speed,
        };

        self.process_with_kernel(input, output, &request, None)
            .await
    }

    /// Apply temporal denoising
    pub async fn apply_denoise(
        &self,
        input: &Path,
        output: &Path,
        strength: f32,
    ) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("strength".to_string(), strength);
        params.insert("threshold".to_string(), 10.0);

        let (width, height) = get_video_dimensions(input).await?;

        let request = KernelRequest {
            intent: "temporal denoise".to_string(),
            width,
            height,
            params,
            optimization: OptimizationTarget::Quality,
        };

        self.process_with_kernel(input, output, &request, None)
            .await
    }

    /// Apply sharpening (unsharp mask)
    pub async fn apply_sharpen(&self, input: &Path, output: &Path, amount: f32) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("amount".to_string(), amount);
        params.insert("threshold".to_string(), 5.0);

        let (width, height) = get_video_dimensions(input).await?;

        let request = KernelRequest {
            intent: "sharpen unsharp mask".to_string(),
            width,
            height,
            params,
            optimization: OptimizationTarget::Balanced,
        };

        self.process_with_kernel(input, output, &request, None)
            .await
    }

    /// AI-powered custom effect generation
    pub async fn generate_custom_effect(
        &self,
        input: &Path,
        output: &Path,
        effect_description: &str,
    ) -> Result<()> {
        info!(
            "[CUDA-PIPELINE] Generating custom effect: {}",
            effect_description
        );

        let (width, height) = get_video_dimensions(input).await?;

        let request = KernelRequest {
            intent: effect_description.to_string(),
            width,
            height,
            params: HashMap::new(),
            optimization: OptimizationTarget::Balanced,
        };

        self.process_with_kernel(input, output, &request, None)
            .await
    }
}

/// Get video dimensions using ffprobe
async fn get_video_dimensions(video_path: &Path) -> Result<(u32, u32)> {
    use tokio::process::Command;

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
            video_path.to_str().unwrap(),
        ])
        .output()
        .await?;

    let dims = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = dims.trim().split(',').collect();

    let width: u32 = parts[0].parse()?;
    let height: u32 = parts[1].parse()?;

    Ok((width, height))
}

/// Builder for creating custom kernel requests
pub struct KernelRequestBuilder {
    intent: String,
    width: u32,
    height: u32,
    params: HashMap<String, f32>,
    optimization: OptimizationTarget,
}

impl KernelRequestBuilder {
    /// Create new builder
    pub fn new(intent: impl Into<String>) -> Self {
        Self {
            intent: intent.into(),
            width: 1920,
            height: 1080,
            params: HashMap::new(),
            optimization: OptimizationTarget::Balanced,
        }
    }

    /// Set video dimensions
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Add parameter
    pub fn param(mut self, key: impl Into<String>, value: f32) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    /// Set optimization target
    pub fn optimization(mut self, target: OptimizationTarget) -> Self {
        self.optimization = target;
        self
    }

    /// Build kernel request
    pub fn build(self) -> KernelRequest {
        KernelRequest {
            intent: self.intent,
            width: self.width,
            height: self.height,
            params: self.params,
            optimization: self.optimization,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_request_builder() {
        let request = KernelRequestBuilder::new("test effect")
            .dimensions(1920, 1080)
            .param("intensity", 0.8)
            .param("radius", 5.0)
            .optimization(OptimizationTarget::Quality)
            .build();

        assert_eq!(request.intent, "test effect");
        assert_eq!(request.width, 1920);
        assert_eq!(request.height, 1080);
        assert_eq!(*request.params.get("intensity").unwrap(), 0.8);
    }
}
