// SYNOID CUDA Kernel Generator
// Inspired by CUDA-Agent: AI-powered kernel generation for video processing
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

/// CUDA kernel generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelRequest {
    /// High-level intent (e.g., "custom color grading", "temporal blur")
    pub intent: String,
    /// Input video dimensions
    pub width: u32,
    pub height: u32,
    /// Processing parameters
    pub params: HashMap<String, f32>,
    /// Optimization target
    pub optimization: OptimizationTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationTarget {
    Speed,      // Maximum throughput
    Quality,    // Best output quality
    Balanced,   // Speed/quality balance
    LowMemory,  // Minimize GPU memory usage
}

/// Generated CUDA kernel with metadata
#[derive(Debug, Clone)]
pub struct GeneratedKernel {
    pub name: String,
    pub source_code: String,
    pub compiled_path: Option<PathBuf>,
    pub block_size: (u32, u32, u32),
    pub grid_size: (u32, u32, u32),
    pub shared_memory_bytes: usize,
}

/// CUDA Kernel Generator - AI-powered kernel synthesis
#[derive(Clone)]
pub struct CudaKernelGenerator {
    /// Cache directory for compiled kernels
    cache_dir: PathBuf,
    /// Skill library for common patterns
    skill_library: KernelSkillLibrary,
    /// LLM provider for kernel generation
    llm_provider: Option<Arc<crate::agent::llm_provider::MultiProviderLlm>>,
}

use std::sync::Arc;

impl CudaKernelGenerator {
    /// Create new kernel generator
    pub fn new(cache_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&cache_dir).ok();
        Self {
            cache_dir,
            skill_library: KernelSkillLibrary::default(),
            llm_provider: None,
        }
    }

    /// Set LLM provider for AI-powered generation
    pub fn with_llm(mut self, provider: Arc<crate::agent::llm_provider::MultiProviderLlm>) -> Self {
        self.llm_provider = Some(provider);
        self
    }

    /// Generate kernel from request
    pub async fn generate(&self, request: &KernelRequest) -> Result<GeneratedKernel> {
        info!("[CUDA-GEN] Generating kernel for: {}", request.intent);

        // 1. Check skill library for pre-defined pattern
        if let Some(skill) = self.skill_library.match_skill(&request.intent) {
            info!("[CUDA-GEN] Using skill library: {}", skill.name);
            return self.instantiate_skill(skill, request);
        }

        // 2. Use LLM for custom kernel generation
        if let Some(ref llm) = self.llm_provider {
            info!("[CUDA-GEN] Generating custom kernel via LLM");
            return self.generate_with_llm(llm.clone(), request).await;
        }

        // 3. Fallback to basic template
        warn!("[CUDA-GEN] No LLM available, using basic template");
        self.generate_basic_kernel(request)
    }

    /// Instantiate kernel from skill template
    fn instantiate_skill(&self, skill: &KernelSkill, request: &KernelRequest) -> Result<GeneratedKernel> {
        let source = skill.template
            .replace("{WIDTH}", &request.width.to_string())
            .replace("{HEIGHT}", &request.height.to_string());

        // Apply parameters
        let source = request.params.iter().fold(source, |acc, (key, value)| {
            acc.replace(&format!("{{{}}}", key.to_uppercase()), &value.to_string())
        });

        Ok(GeneratedKernel {
            name: skill.name.clone(),
            source_code: source,
            compiled_path: None,
            block_size: skill.block_size,
            grid_size: self.calculate_grid_size(request.width, request.height, skill.block_size),
            shared_memory_bytes: skill.shared_memory_bytes,
        })
    }

    /// Generate kernel using LLM
    async fn generate_with_llm(
        &self,
        llm: Arc<crate::agent::llm_provider::MultiProviderLlm>,
        request: &KernelRequest,
    ) -> Result<GeneratedKernel> {
        let prompt = self.build_generation_prompt(request);

        let response = llm.reason(&prompt).await
            .map_err(|e| anyhow::anyhow!(e))
            .context("LLM generation failed")?;

        // Extract CUDA code from response
        let source_code = self.extract_cuda_code(&response)?;

        Ok(GeneratedKernel {
            name: format!("custom_{}", sanitize_name(&request.intent)),
            source_code,
            compiled_path: None,
            block_size: (16, 16, 1),
            grid_size: self.calculate_grid_size(request.width, request.height, (16, 16, 1)),
            shared_memory_bytes: 0,
        })
    }

    /// Build LLM prompt for kernel generation
    fn build_generation_prompt(&self, request: &KernelRequest) -> String {
        format!(
            r#"You are an expert CUDA kernel developer. Generate a high-performance CUDA kernel for video processing.

Task: {}
Input Dimensions: {}x{}
Optimization Target: {:?}
Parameters: {:?}

Requirements:
1. Write a complete CUDA __global__ kernel function
2. Use efficient memory access patterns (coalesced reads/writes)
3. Optimize for the target: {:?}
4. Include proper bounds checking
5. Use shared memory if beneficial
6. Follow CUDA best practices

Output ONLY the CUDA kernel code wrapped in ```cuda code blocks. Include kernel launch configuration as comments."#,
            request.intent,
            request.width,
            request.height,
            request.optimization,
            request.params,
            request.optimization
        )
    }

    /// Extract CUDA code from LLM response
    fn extract_cuda_code(&self, response: &str) -> Result<String> {
        // Extract code between ```cuda and ```
        if let Some(start) = response.find("```cuda") {
            if let Some(end) = response[start..].find("```") {
                let code = &response[start + 7..start + end];
                return Ok(code.trim().to_string());
            }
        }

        // Fallback: look for __global__ keyword
        if response.contains("__global__") {
            return Ok(response.to_string());
        }

        anyhow::bail!("Could not extract CUDA code from LLM response")
    }

    /// Generate basic fallback kernel
    fn generate_basic_kernel(&self, request: &KernelRequest) -> Result<GeneratedKernel> {
        let source = format!(
            r#"
__global__ void basic_process(
    unsigned char* input,
    unsigned char* output,
    int width,
    int height,
    int channels
) {{
    int x = blockIdx.x * blockDim.x + threadIdx.x;
    int y = blockIdx.y * blockDim.y + threadIdx.y;

    if (x >= width || y >= height) return;

    int idx = (y * width + x) * channels;

    // Simple passthrough (modify as needed)
    for (int c = 0; c < channels; c++) {{
        output[idx + c] = input[idx + c];
    }}
}}
"#
        );

        Ok(GeneratedKernel {
            name: "basic_process".to_string(),
            source_code: source,
            compiled_path: None,
            block_size: (16, 16, 1),
            grid_size: self.calculate_grid_size(request.width, request.height, (16, 16, 1)),
            shared_memory_bytes: 0,
        })
    }

    /// Compile kernel to PTX
    pub async fn compile(&self, kernel: &mut GeneratedKernel) -> Result<()> {
        let kernel_file = self.cache_dir.join(format!("{}.cu", kernel.name));
        let ptx_file = self.cache_dir.join(format!("{}.ptx", kernel.name));

        // Write kernel source
        tokio::fs::write(&kernel_file, &kernel.source_code).await
            .context("Failed to write kernel source")?;

        // Compile with nvcc
        info!("[CUDA-GEN] Compiling kernel: {}", kernel.name);
        let output = Command::new("nvcc")
            .args([
                "--ptx",
                "-O3",
                "-arch=sm_75", // Adjust for target GPU
                kernel_file.to_str().unwrap(),
                "-o",
                ptx_file.to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute nvcc")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Kernel compilation failed: {}", stderr);
        }

        kernel.compiled_path = Some(ptx_file);
        info!("[CUDA-GEN] Kernel compiled successfully");
        Ok(())
    }

    /// Calculate grid size based on image dimensions and block size
    fn calculate_grid_size(&self, width: u32, height: u32, block_size: (u32, u32, u32)) -> (u32, u32, u32) {
        let grid_x = (width + block_size.0 - 1) / block_size.0;
        let grid_y = (height + block_size.1 - 1) / block_size.1;
        (grid_x, grid_y, 1)
    }
}

/// Kernel Skill Library - Pre-defined patterns for common video effects
#[derive(Debug, Clone)]
pub struct KernelSkillLibrary {
    skills: Vec<KernelSkill>,
}

#[derive(Debug, Clone)]
pub struct KernelSkill {
    pub name: String,
    pub keywords: Vec<String>,
    pub template: String,
    pub block_size: (u32, u32, u32),
    pub shared_memory_bytes: usize,
}

impl Default for KernelSkillLibrary {
    fn default() -> Self {
        let mut skills = Vec::new();

        // Skill: Color Grading (LUT-based)
        skills.push(KernelSkill {
            name: "color_grading_lut".to_string(),
            keywords: vec!["color".into(), "grade".into(), "lut".into(), "cinematic".into()],
            template: include_str!("cuda_skills/color_grading.cu").to_string(),
            block_size: (16, 16, 1),
            shared_memory_bytes: 0,
        });

        // Skill: Fast Gaussian Blur
        skills.push(KernelSkill {
            name: "gaussian_blur".to_string(),
            keywords: vec!["blur".into(), "gaussian".into(), "smooth".into()],
            template: include_str!("cuda_skills/gaussian_blur.cu").to_string(),
            block_size: (16, 16, 1),
            shared_memory_bytes: 4096, // Shared memory for blur optimization
        });

        // Skill: Temporal Denoising
        skills.push(KernelSkill {
            name: "temporal_denoise".to_string(),
            keywords: vec!["denoise".into(), "noise".into(), "clean".into(), "temporal".into()],
            template: include_str!("cuda_skills/temporal_denoise.cu").to_string(),
            block_size: (16, 16, 1),
            shared_memory_bytes: 0,
        });

        // Skill: Unsharp Mask (Sharpening)
        skills.push(KernelSkill {
            name: "unsharp_mask".to_string(),
            keywords: vec!["sharpen".into(), "sharp".into(), "unsharp".into(), "enhance".into()],
            template: include_str!("cuda_skills/unsharp_mask.cu").to_string(),
            block_size: (16, 16, 1),
            shared_memory_bytes: 2048,
        });

        Self { skills }
    }
}

impl KernelSkillLibrary {
    /// Match skill based on intent keywords
    pub fn match_skill(&self, intent: &str) -> Option<&KernelSkill> {
        let intent_lower = intent.to_lowercase();

        for skill in &self.skills {
            for keyword in &skill.keywords {
                if intent_lower.contains(keyword) {
                    return Some(skill);
                }
            }
        }

        None
    }
}

/// Sanitize name for use as kernel identifier
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_matching() {
        let library = KernelSkillLibrary::default();
        assert!(library.match_skill("apply cinematic color grading").is_some());
        assert!(library.match_skill("blur the video").is_some());
        assert!(library.match_skill("denoise footage").is_some());
    }

    #[test]
    fn test_name_sanitization() {
        assert_eq!(sanitize_name("Custom Color-Grading!"), "custom_color_grading_");
    }
}
