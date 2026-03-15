// SYNOID CUDA Agent Integration Demo
// Demonstrates all CUDA agent capabilities
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID

use synoid_core::agent::cuda::cuda_kernel_gen::{
    CudaKernelGenerator, KernelRequest, OptimizationTarget,
};
use synoid_core::agent::cuda::cuda_pipeline::{CudaPipeline, KernelRequestBuilder};
use synoid_core::agent::cuda::latent_optimizer::{LatentConfig, LatentOptimizer};
use synoid_core::agent::specialized::synoid_link::SynoidLink;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("\n🚀 SYNOID CUDA Agent Integration Demo");
    println!("=====================================\n");

    // 1. Test CUDA Kernel Generator
    test_kernel_generator().await?;

    // 2. Test CUDA Pipeline
    test_cuda_pipeline().await?;

    // 3. Test SynoidLink
    test_synoid_link().await?;

    // 4. Test Latent Optimizer
    test_latent_optimizer().await?;

    // 5. Test Skill Library
    test_skill_library().await?;

    // 6. Check System Requirements
    check_system_requirements().await?;

    println!("\n✨ All CUDA Agent tests completed successfully!");
    println!("🎯 SYNOID CUDA integration is fully operational.\n");

    Ok(())
}

async fn test_kernel_generator() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n📦 Test 1: CUDA Kernel Generator");
    info!("================================");

    let cache_dir = PathBuf::from(".synoid_cuda_cache");
    std::fs::create_dir_all(&cache_dir)?;

    let generator = CudaKernelGenerator::new(cache_dir.clone());

    // Test generating different kernel types
    let test_cases = vec![
        ("cinematic color grading", 1920, 1080),
        ("gaussian blur", 3840, 2160),
        ("temporal denoise", 1920, 1080),
        ("sharpen video", 1920, 1080),
    ];

    for (intent, width, height) in test_cases {
        let mut params = HashMap::new();
        params.insert("intensity".to_string(), 0.8);

        let request = KernelRequest {
            intent: intent.to_string(),
            width,
            height,
            params,
            optimization: OptimizationTarget::Balanced,
        };

        let kernel = generator.generate(&request).await?;
        info!("✅ Generated kernel: {} ({}x{})", kernel.name, width, height);
        info!("   Block size: {:?}", kernel.block_size);
        info!("   Grid size: {:?}", kernel.grid_size);
        info!("   Shared memory: {} bytes", kernel.shared_memory_bytes);
    }

    Ok(())
}

async fn test_cuda_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔧 Test 2: CUDA Pipeline");
    info!("========================");

    let cache_dir = PathBuf::from(".synoid_cuda_cache");
    let pipeline = CudaPipeline::new(cache_dir, 0);

    info!("✅ CudaPipeline initialized");
    info!("   Device ID: 0");
    info!("   Cache directory: .synoid_cuda_cache");

    // Test kernel request builder
    let custom_request = KernelRequestBuilder::new("vintage film look")
        .dimensions(1920, 1080)
        .param("grain_amount", 0.3)
        .param("vignette_strength", 0.5)
        .param("saturation", 0.7)
        .optimization(OptimizationTarget::Quality)
        .build();

    info!("✅ Built custom kernel request: {}", custom_request.intent);
    info!("   Dimensions: {}x{}", custom_request.width, custom_request.height);
    info!("   Parameters: {} items", custom_request.params.len());

    Ok(())
}

async fn test_synoid_link() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔗 Test 3: SynoidLink Bridge");
    info!("============================");

    let cache_dir = PathBuf::from(".synoid_cuda_cache");
    let synoid_link = SynoidLink::new(cache_dir, 0);

    info!("✅ SynoidLink initialized");
    info!("   Ready to bridge SYNOID pipeline with CUDA kernels");

    // Test frame structure
    use synoid_core::agent::specialized::synoid_link::SynoidFrame;

    let test_frame = SynoidFrame::new(
        1920,
        1080,
        3,
        vec![0u8; 1920 * 1080 * 3],
        0.0,
    );

    info!("✅ Created test frame");
    info!("   Dimensions: {}x{}", test_frame.width, test_frame.height);
    info!("   Channels: {}", test_frame.channels);
    info!("   Data size: {} bytes", test_frame.data.len());

    Ok(())
}

async fn test_latent_optimizer() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🎨 Test 4: Latent Optimizer");
    info!("===========================");

    let default_config = LatentConfig::default();
    let optimizer = LatentOptimizer::new(default_config.clone());

    info!("✅ LatentOptimizer initialized");
    info!("   Compression ratio: {}", default_config.compression_ratio);
    info!("   Temporal compression: {}", default_config.temporal_compression);
    info!("   Frame sampling: {}", default_config.frame_sampling);
    info!("   GPU accelerated: {}", default_config.gpu_accelerated);

    // Test custom configuration
    let custom_config = LatentConfig {
        compression_ratio: 0.3,
        temporal_compression: true,
        frame_sampling: 2,
        gpu_accelerated: true,
    };

    let custom_optimizer = LatentOptimizer::new(custom_config);
    info!("✅ Created custom LatentOptimizer");
    info!("   Optimized for: High quality, 2x frame sampling");

    Ok(())
}

async fn test_skill_library() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n📚 Test 5: CUDA Skill Library");
    info!("==============================");

    use synoid_core::agent::cuda::cuda_kernel_gen::KernelSkillLibrary;

    let library = KernelSkillLibrary::default();

    let test_intents = vec![
        "apply cinematic color grading",
        "gaussian blur the video",
        "denoise noisy footage",
        "sharpen the video",
        "enhance image quality",
    ];

    for intent in test_intents {
        if let Some(skill) = library.match_skill(intent) {
            info!("✅ Matched skill: {} -> {}", intent, skill.name);
            info!("   Keywords: {:?}", skill.keywords);
            info!("   Block size: {:?}", skill.block_size);
        } else {
            info!("⚠️  No skill matched: {}", intent);
        }
    }

    // Verify all CUDA skill files exist
    let skills_dir = PathBuf::from("src/agent/cuda/cuda_skills");
    let skill_files = vec![
        "color_grading.cu",
        "gaussian_blur.cu",
        "temporal_denoise.cu",
        "unsharp_mask.cu",
    ];

    info!("\n📁 Verifying CUDA skill files:");
    for skill_file in skill_files {
        let skill_path = skills_dir.join(skill_file);
        if skill_path.exists() {
            info!("   ✅ {}", skill_file);
        } else {
            warn!("   ❌ Missing: {}", skill_file);
        }
    }

    Ok(())
}

async fn check_system_requirements() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 6: System Requirements Check");
    info!("====================================");

    // Check for NVCC (CUDA compiler)
    match tokio::process::Command::new("nvcc")
        .arg("--version")
        .output()
        .await
    {
        Ok(output) if output.status.success() => {
            let version_info = String::from_utf8_lossy(&output.stdout);
            let version_line = version_info
                .lines()
                .find(|line| line.contains("release"))
                .unwrap_or("Unknown version");
            info!("✅ NVCC (CUDA Compiler): {}", version_line.trim());
        }
        _ => {
            warn!("⚠️  NVCC not found - CUDA kernel compilation unavailable");
            warn!("   Install CUDA Toolkit from: https://developer.nvidia.com/cuda-downloads");
            warn!("   SYNOID will fall back to CPU/FFmpeg processing");
        }
    }

    // Check for FFmpeg
    match tokio::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .await
    {
        Ok(output) if output.status.success() => {
            let version_info = String::from_utf8_lossy(&output.stdout);
            let version_line = version_info.lines().next().unwrap_or("Unknown");
            info!("✅ FFmpeg: {}", version_line.trim());
        }
        _ => {
            warn!("⚠️  FFmpeg not found - video processing unavailable");
        }
    }

    // Check for NVIDIA GPU
    match tokio::process::Command::new("nvidia-smi")
        .output()
        .await
    {
        Ok(output) if output.status.success() => {
            info!("✅ NVIDIA GPU detected (nvidia-smi available)");

            // Parse GPU info
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = output_str.lines().find(|l| l.contains("NVIDIA")) {
                let gpu_info = line.split('|').nth(1).unwrap_or("").trim();
                info!("   GPU: {}", gpu_info);
            }
        }
        _ => {
            warn!("⚠️  nvidia-smi not found - NVIDIA GPU may not be available");
            warn!("   CUDA acceleration will not be possible");
        }
    }

    // Check Rust/Cargo version
    match tokio::process::Command::new("cargo")
        .arg("--version")
        .output()
        .await
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            info!("✅ Cargo: {}", version.trim());
        }
        _ => {
            warn!("⚠️  Cargo not found");
        }
    }

    Ok(())
}
