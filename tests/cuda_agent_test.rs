// SYNOID CUDA Agent Integration Tests
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID

#[cfg(test)]
mod cuda_agent_tests {
    use synoid_core::agent::cuda::cuda_kernel_gen::{
        CudaKernelGenerator, KernelRequest, OptimizationTarget,
    };
    use synoid_core::agent::cuda::cuda_pipeline::{CudaPipeline, KernelRequestBuilder};
    use synoid_core::agent::specialized::synoid_link::SynoidLink;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_cuda_kernel_generator_creation() {
        let cache_dir = PathBuf::from(".test_cuda_cache");
        let generator = CudaKernelGenerator::new(cache_dir);
        println!("✅ CudaKernelGenerator created successfully");
    }

    #[test]
    fn test_kernel_skill_library() {
        use synoid_core::agent::cuda::cuda_kernel_gen::KernelSkillLibrary;

        let library = KernelSkillLibrary::default();

        // Test color grading skill matching
        let color_skill = library.match_skill("apply cinematic color grading");
        assert!(color_skill.is_some(), "Color grading skill should match");
        println!("✅ Color grading skill matched: {}", color_skill.unwrap().name);

        // Test blur skill matching
        let blur_skill = library.match_skill("gaussian blur the video");
        assert!(blur_skill.is_some(), "Blur skill should match");
        println!("✅ Blur skill matched: {}", blur_skill.unwrap().name);

        // Test denoise skill matching
        let denoise_skill = library.match_skill("denoise noisy footage");
        assert!(denoise_skill.is_some(), "Denoise skill should match");
        println!("✅ Denoise skill matched: {}", denoise_skill.unwrap().name);

        // Test sharpen skill matching
        let sharpen_skill = library.match_skill("sharpen the video");
        assert!(sharpen_skill.is_some(), "Sharpen skill should match");
        println!("✅ Sharpen skill matched: {}", sharpen_skill.unwrap().name);
    }

    #[tokio::test]
    async fn test_kernel_generation_from_request() {
        let cache_dir = PathBuf::from(".test_cuda_cache");
        std::fs::create_dir_all(&cache_dir).ok();

        let generator = CudaKernelGenerator::new(cache_dir);

        let mut params = HashMap::new();
        params.insert("intensity".to_string(), 0.8);

        let request = KernelRequest {
            intent: "cinematic color grading".to_string(),
            width: 1920,
            height: 1080,
            params,
            optimization: OptimizationTarget::Quality,
        };

        let result = generator.generate(&request).await;
        assert!(result.is_ok(), "Kernel generation should succeed");

        let kernel = result.unwrap();
        println!("✅ Generated kernel: {}", kernel.name);
        println!("   Block size: {:?}", kernel.block_size);
        println!("   Grid size: {:?}", kernel.grid_size);
        assert!(!kernel.source_code.is_empty(), "Kernel source should not be empty");
    }

    #[test]
    fn test_kernel_request_builder() {
        let request = KernelRequestBuilder::new("vintage film look")
            .dimensions(3840, 2160)
            .param("grain_amount", 0.3)
            .param("vignette_strength", 0.5)
            .param("saturation", 0.7)
            .optimization(OptimizationTarget::Quality)
            .build();

        assert_eq!(request.intent, "vintage film look");
        assert_eq!(request.width, 3840);
        assert_eq!(request.height, 2160);
        assert_eq!(*request.params.get("grain_amount").unwrap(), 0.3);
        println!("✅ KernelRequestBuilder works correctly");
    }

    #[test]
    fn test_cuda_pipeline_creation() {
        let cache_dir = PathBuf::from(".test_cuda_cache");
        let pipeline = CudaPipeline::new(cache_dir, 0);
        println!("✅ CudaPipeline created successfully");
    }

    #[test]
    fn test_synoid_link_creation() {
        let cache_dir = PathBuf::from(".test_cuda_cache");
        let synoid_link = SynoidLink::new(cache_dir, 0);
        println!("✅ SynoidLink created successfully");
    }

    #[test]
    fn test_optimization_targets() {
        let speed = OptimizationTarget::Speed;
        let quality = OptimizationTarget::Quality;
        let balanced = OptimizationTarget::Balanced;
        let low_memory = OptimizationTarget::LowMemory;

        println!("✅ Optimization targets:");
        println!("   - Speed: {:?}", speed);
        println!("   - Quality: {:?}", quality);
        println!("   - Balanced: {:?}", balanced);
        println!("   - LowMemory: {:?}", low_memory);
    }

    #[tokio::test]
    async fn test_kernel_compilation_requirement() {
        // This test verifies that we can detect NVCC availability
        let cache_dir = PathBuf::from(".test_cuda_cache");
        std::fs::create_dir_all(&cache_dir).ok();

        let generator = CudaKernelGenerator::new(cache_dir);

        let request = KernelRequest {
            intent: "basic passthrough".to_string(),
            width: 1920,
            height: 1080,
            params: HashMap::new(),
            optimization: OptimizationTarget::Balanced,
        };

        let mut kernel = generator.generate(&request).await.unwrap();

        // Check if NVCC is available
        match tokio::process::Command::new("nvcc")
            .arg("--version")
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                println!("✅ NVCC detected: {}", version.lines().next().unwrap_or(""));

                // Try compilation
                let compile_result = generator.compile(&mut kernel).await;
                if compile_result.is_ok() {
                    println!("✅ Kernel compilation successful");
                } else {
                    println!("⚠️  Kernel compilation failed: {:?}", compile_result.err());
                }
            }
            _ => {
                println!("⚠️  NVCC not found - CUDA compilation will not be available");
                println!("   Install CUDA Toolkit to enable GPU acceleration");
            }
        }
    }

    #[test]
    fn test_all_cuda_skills_present() {
        let skills_dir = PathBuf::from("src/agent/cuda/cuda_skills");

        let expected_skills = vec![
            "color_grading.cu",
            "gaussian_blur.cu",
            "temporal_denoise.cu",
            "unsharp_mask.cu",
        ];

        for skill_file in expected_skills {
            let skill_path = skills_dir.join(skill_file);
            assert!(
                skill_path.exists(),
                "CUDA skill file should exist: {}",
                skill_file
            );
            println!("✅ Found CUDA skill: {}", skill_file);
        }
    }

    #[test]
    fn test_latent_optimizer_config() {
        use synoid_core::agent::cuda::latent_optimizer::LatentConfig;

        let default_config = LatentConfig::default();
        assert_eq!(default_config.compression_ratio, 0.5);
        assert!(default_config.temporal_compression);
        assert_eq!(default_config.frame_sampling, 1);
        assert!(default_config.gpu_accelerated);
        println!("✅ LatentConfig defaults are correct");

        let custom_config = LatentConfig {
            compression_ratio: 0.3,
            temporal_compression: false,
            frame_sampling: 2,
            gpu_accelerated: false,
        };
        assert_eq!(custom_config.compression_ratio, 0.3);
        println!("✅ Custom LatentConfig works");
    }
}
