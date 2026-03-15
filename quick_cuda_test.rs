// Quick CUDA Agent Test - Standalone verification
// Run with: cargo run --bin quick_cuda_test

use synoid_core::agent::cuda::cuda_kernel_gen::{
    CudaKernelGenerator, KernelRequest, KernelSkillLibrary, OptimizationTarget,
};
use synoid_core::agent::cuda::cuda_pipeline::{CudaPipeline, KernelRequestBuilder};
use synoid_core::agent::cuda::latent_optimizer::LatentConfig;
use synoid_core::agent::specialized::synoid_link::SynoidLink;
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    println!("\n🚀 SYNOID CUDA Agent Quick Test");
    println!("================================\n");

    let mut all_passed = true;

    // Test 1: Kernel Generator
    print!("📦 CudaKernelGenerator... ");
    let cache_dir = PathBuf::from(".synoid_cuda_cache");
    std::fs::create_dir_all(&cache_dir).ok();
    let generator = CudaKernelGenerator::new(cache_dir.clone());
    println!("✅");

    // Test 2: Skill Library
    print!("📚 KernelSkillLibrary... ");
    let library = KernelSkillLibrary::default();
    let color_skill = library.match_skill("cinematic color grading");
    if color_skill.is_some() {
        println!("✅");
    } else {
        println!("❌");
        all_passed = false;
    }

    // Test 3: Kernel Generation
    print!("⚙️  Kernel Generation... ");
    let mut params = HashMap::new();
    params.insert("intensity".to_string(), 0.8);

    let request = KernelRequest {
        intent: "cinematic color grading".to_string(),
        width: 1920,
        height: 1080,
        params,
        optimization: OptimizationTarget::Quality,
    };

    match generator.generate(&request).await {
        Ok(kernel) => {
            if !kernel.source_code.is_empty() {
                println!("✅ ({})", kernel.name);
            } else {
                println!("❌ (empty source)");
                all_passed = false;
            }
        }
        Err(e) => {
            println!("❌ ({})", e);
            all_passed = false;
        }
    }

    // Test 4: CUDA Pipeline
    print!("🔧 CudaPipeline... ");
    let _pipeline = CudaPipeline::new(cache_dir.clone(), 0);
    println!("✅");

    // Test 5: SynoidLink
    print!("🔗 SynoidLink... ");
    let _synoid_link = SynoidLink::new(cache_dir.clone(), 0);
    println!("✅");

    // Test 6: Kernel Request Builder
    print!("🎨 KernelRequestBuilder... ");
    let custom_request = KernelRequestBuilder::new("test effect")
        .dimensions(1920, 1080)
        .param("intensity", 0.8)
        .optimization(OptimizationTarget::Balanced)
        .build();

    if custom_request.intent == "test effect" && custom_request.width == 1920 {
        println!("✅");
    } else {
        println!("❌");
        all_passed = false;
    }

    // Test 7: Latent Config
    print!("🎨 LatentConfig... ");
    let config = LatentConfig::default();
    if config.compression_ratio == 0.5 && config.temporal_compression {
        println!("✅");
    } else {
        println!("❌");
        all_passed = false;
    }

    // Test 8: Check CUDA Skills
    print!("📁 CUDA Skill Files... ");
    let skills_dir = PathBuf::from("src/agent/cuda/cuda_skills");
    let skills = vec![
        "color_grading.cu",
        "gaussian_blur.cu",
        "temporal_denoise.cu",
        "unsharp_mask.cu",
    ];

    let mut skills_ok = true;
    for skill in &skills {
        if !skills_dir.join(skill).exists() {
            skills_ok = false;
            break;
        }
    }

    if skills_ok {
        println!("✅ (4 files)");
    } else {
        println!("❌ (missing files)");
        all_passed = false;
    }

    // Summary
    println!("\n{}", "=".repeat(40));
    if all_passed {
        println!("✨ All checks PASSED!");
        println!("\n🎯 SYNOID CUDA Agent is ready to use!");
        println!("\nNext steps:");
        println!("  • cargo run --example cuda_agent_demo");
        println!("  • cargo test --test cuda_agent_test");
        println!("  • See: CUDA_AGENT_SETUP.md");
    } else {
        println!("❌ Some checks FAILED");
        println!("\nSee CUDA_AGENT_SETUP.md for troubleshooting");
        std::process::exit(1);
    }
    println!();
}
