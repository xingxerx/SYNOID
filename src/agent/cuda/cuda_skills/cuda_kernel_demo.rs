// SYNOID CUDA Kernel Demo
// Demonstrates the CUDA-Agent integration capabilities

use crate::agent::cuda::cuda_pipeline::{CudaPipeline, KernelRequestBuilder};
use crate::agent::cuda::cuda_kernel_gen::OptimizationTarget;
use std::path::Path;
use tracing::info;

#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 SYNOID CUDA Kernel Demo");

    // Initialize CUDA pipeline
    let cache_dir = std::env::current_dir()?.join(".synoid_cuda_cache");
    let cuda_pipeline = CudaPipeline::new(cache_dir, 0);

    let input_video = Path::new("input.mp4");
    let output_dir = Path::new("output");
    std::fs::create_dir_all(output_dir)?;

    // Example 1: Cinematic Color Grading
    info!("\n📸 Example 1: Cinematic Color Grading");
    cuda_pipeline
        .apply_color_grading(
            input_video,
            &output_dir.join("cinematic.mp4"),
            0.8, // 80% intensity
        )
        .await?;
    info!("✅ Cinematic grading complete");

    // Example 2: Gaussian Blur
    info!("\n🌫️  Example 2: Gaussian Blur");
    cuda_pipeline
        .apply_blur(
            input_video,
            &output_dir.join("blurred.mp4"),
            5.0, // 5-pixel radius
        )
        .await?;
    info!("✅ Blur complete");

    // Example 3: Temporal Denoising
    info!("\n🧹 Example 3: Temporal Denoising");
    cuda_pipeline
        .apply_denoise(
            input_video,
            &output_dir.join("denoised.mp4"),
            0.6, // 60% strength
        )
        .await?;
    info!("✅ Denoising complete");

    // Example 4: Sharpening
    info!("\n✨ Example 4: Sharpening (Unsharp Mask)");
    cuda_pipeline
        .apply_sharpen(
            input_video,
            &output_dir.join("sharpened.mp4"),
            1.2, // 120% sharpening
        )
        .await?;
    info!("✅ Sharpening complete");

    // Example 5: Custom Effect with Builder Pattern
    info!("\n🎨 Example 5: Custom Kernel Request");
    let custom_request = KernelRequestBuilder::new("vintage film look")
        .dimensions(1920, 1080)
        .param("grain_amount", 0.3)
        .param("vignette_strength", 0.5)
        .param("saturation", 0.7)
        .optimization(OptimizationTarget::Quality)
        .build();

    cuda_pipeline
        .process_with_kernel(
            input_video,
            &output_dir.join("vintage.mp4"),
            &custom_request,
            None,
        )
        .await?;
    info!("✅ Custom effect complete");

    // Example 6: AI-Powered Custom Effect (requires LLM)
    info!("\n🤖 Example 6: AI-Generated Custom Effect");
    cuda_pipeline
        .generate_custom_effect(
            input_video,
            &output_dir.join("custom_ai.mp4"),
            "create a dreamy, soft-focus aesthetic with pastel colors",
        )
        .await?;
    info!("✅ AI-generated effect complete");

    info!("\n🎉 All demos complete! Check the output/ directory.");
    Ok(())
}
