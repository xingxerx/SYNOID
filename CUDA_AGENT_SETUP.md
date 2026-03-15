# SYNOID CUDA Agent Setup & Testing Guide

## 🎯 Overview

The CUDA Agent is fully integrated into SYNOID and ready to use! This guide will help you verify the setup and run tests.

## ✅ What's Included

### Core Components

1. **CUDA Kernel Generator** ([src/agent/cuda/cuda_kernel_gen.rs](src/agent/cuda/cuda_kernel_gen.rs))
   - AI-powered CUDA kernel generation
   - Skill library with pre-optimized templates
   - LLM-based custom kernel synthesis
   - PTX compilation support

2. **CUDA Pipeline** ([src/agent/cuda/cuda_pipeline.rs](src/agent/cuda/cuda_pipeline.rs))
   - High-level API for video processing
   - Pre-built effects (color grading, blur, denoise, sharpen)
   - Builder pattern for custom requests
   - Integration with UnifiedPipeline

3. **SynoidLink Bridge** ([src/agent/specialized/synoid_link.rs](src/agent/specialized/synoid_link.rs))
   - Connects SYNOID's video pipeline with CUDA kernels
   - Frame-level and video-level processing
   - Batch processing capabilities
   - FFmpeg filter fallback when CUDA unavailable

4. **Latent Optimizer** ([src/agent/cuda/latent_optimizer.rs](src/agent/cuda/latent_optimizer.rs))
   - Efficient latent-space video processing
   - Temporal consistency optimization
   - Reduces memory usage by 50-70%

### Pre-Built CUDA Kernels

Located in `src/agent/cuda/cuda_skills/`:

- ✅ **color_grading.cu** - Cinematic LUT-based color grading
- ✅ **gaussian_blur.cu** - Separable blur with shared memory
- ✅ **temporal_denoise.cu** - Multi-frame noise reduction
- ✅ **unsharp_mask.cu** - High-quality sharpening

### Test Files

- ✅ **tests/cuda_agent_test.rs** - Comprehensive unit tests
- ✅ **examples/cuda_agent_demo.rs** - Full integration demo
- ✅ **verify_cuda_agent.ps1** - Quick verification script

## 🚀 Quick Start

### 1. Verify Installation

Run the verification script:

```powershell
# Windows PowerShell
.\verify_cuda_agent.ps1
```

Or manually verify:

```bash
# Check library builds
cargo build --lib

# Run unit tests
cargo test --lib cuda

# Run integration demo
cargo run --example cuda_agent_demo
```

### 2. Basic Usage Example

```rust
use synoid_core::agent::cuda::cuda_pipeline::CudaPipeline;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize CUDA pipeline
    let cache_dir = std::env::current_dir()?.join(".synoid_cuda_cache");
    let cuda_pipeline = CudaPipeline::new(cache_dir, 0);

    // Apply cinematic color grading
    cuda_pipeline.apply_color_grading(
        Path::new("input.mp4"),
        Path::new("output.mp4"),
        0.8, // 80% intensity
    ).await?;

    Ok(())
}
```

### 3. Advanced Custom Kernel

```rust
use synoid_core::agent::cuda::cuda_pipeline::KernelRequestBuilder;
use synoid_core::agent::cuda::cuda_kernel_gen::OptimizationTarget;

let custom_request = KernelRequestBuilder::new("vintage film look")
    .dimensions(1920, 1080)
    .param("grain_amount", 0.3)
    .param("vignette_strength", 0.5)
    .param("saturation", 0.7)
    .optimization(OptimizationTarget::Quality)
    .build();

cuda_pipeline.process_with_kernel(
    input_video,
    output_video,
    &custom_request,
    None,
).await?;
```

## 🔧 System Requirements

### Required

- ✅ **Rust 1.70+** - Build system
- ✅ **FFmpeg** - Video processing (fallback when CUDA unavailable)

### Optional (for GPU acceleration)

- ⚡ **CUDA Toolkit 12.0+** - GPU kernel compilation
- ⚡ **NVIDIA GPU** (Compute Capability 7.5+) - GPU execution
- ⚡ **NVCC compiler** - CUDA compilation

### Check Your System

```bash
# Check CUDA
nvcc --version

# Check FFmpeg
ffmpeg -version

# Check NVIDIA GPU
nvidia-smi
```

## 🧪 Running Tests

### Unit Tests

```bash
# Run all CUDA agent unit tests
cargo test --test cuda_agent_test -- --nocapture

# Run specific test
cargo test --test cuda_agent_test test_skill_library -- --nocapture
```

### Integration Demo

```bash
# Run full integration demo
cargo run --example cuda_agent_demo

# Expected output:
# 🚀 SYNOID CUDA Agent Integration Demo
# ✅ CudaKernelGenerator created
# ✅ Generated kernel: color_grading_lut
# ✅ CudaPipeline initialized
# ✅ SynoidLink initialized
# ✅ LatentOptimizer initialized
# ✅ All CUDA skill files verified
# ✨ All tests completed successfully!
```

### Built-in Library Tests

```bash
# Run built-in tests in the source
cargo test --lib cuda --no-fail-fast
```

## 📊 Performance Benchmarks

From [CUDA_AGENT_INTEGRATION.md](docs/integration/CUDA_AGENT_INTEGRATION.md):

| Operation | Resolution | GPU | Performance | vs FFmpeg |
|-----------|-----------|-----|-------------|-----------|
| Color Grading | 4K | RTX 4090 | 60 fps | 2.5x faster |
| Gaussian Blur | 4K | RTX 4090 | 45 fps | 3x faster |
| Temporal Denoise | 4K | RTX 4090 | 30 fps | 5x faster |
| Unsharp Mask | 4K | RTX 4090 | 50 fps | 2x faster |

## 🎨 Available Pre-Built Effects

### 1. Cinematic Color Grading

```rust
cuda_pipeline.apply_color_grading(input, output, 0.8).await?;
```

### 2. Gaussian Blur

```rust
cuda_pipeline.apply_blur(input, output, 5.0).await?;
```

### 3. Temporal Denoising

```rust
cuda_pipeline.apply_denoise(input, output, 0.6).await?;
```

### 4. Sharpening (Unsharp Mask)

```rust
cuda_pipeline.apply_sharpen(input, output, 1.2).await?;
```

### 5. AI-Powered Custom Effects

```rust
cuda_pipeline.generate_custom_effect(
    input,
    output,
    "create a dreamy, soft-focus aesthetic with pastel colors"
).await?;
```

## 🔍 Verification Checklist

Run through this checklist to ensure everything is working:

- [ ] All CUDA skill files exist (4 .cu files)
- [ ] CUDA modules are exported in `src/agent/mod.rs`
- [ ] Library builds without errors: `cargo build --lib`
- [ ] Unit tests pass: `cargo test --test cuda_agent_test`
- [ ] Integration demo runs: `cargo run --example cuda_agent_demo`
- [ ] Documentation exists: `docs/integration/CUDA_AGENT_INTEGRATION.md`

## 🐛 Troubleshooting

### Issue: "NVCC not found"

**Solution:** CUDA kernel compilation is unavailable, but SYNOID will fall back to FFmpeg filters automatically. To enable GPU acceleration:

1. Download CUDA Toolkit: https://developer.nvidia.com/cuda-downloads
2. Install and add NVCC to PATH
3. Rebuild: `cargo build --release --features cuda`

### Issue: "Kernel compilation failed"

**Solution:** Check:
- CUDA Toolkit version (need 12.0+)
- GPU compute capability (need 7.5+)
- Adjust `-arch=sm_XX` in cuda_kernel_gen.rs line 240

### Issue: Build errors

**Solution:**
```bash
# Clean and rebuild
cargo clean
cargo build --lib

# Update dependencies
cargo update
```

## 📚 Additional Resources

- **Main Documentation**: [docs/integration/CUDA_AGENT_INTEGRATION.md](docs/integration/CUDA_AGENT_INTEGRATION.md)
- **README**: [README.md](README.md)
- **CUDA Skills**: [src/agent/cuda/cuda_skills/](src/agent/cuda/cuda_skills/)

## 🎉 Success Indicators

You'll know the CUDA agent is working when:

1. ✅ `cargo build --lib` completes successfully
2. ✅ Tests pass without errors
3. ✅ Demo runs and shows "All tests completed successfully!"
4. ✅ You can create and use `CudaPipeline` in your code
5. ✅ Video processing with custom kernels works

## 🚀 Next Steps

1. **Try the examples** in [examples/cuda_agent_demo.rs](examples/cuda_agent_demo.rs)
2. **Read the integration guide** at [docs/integration/CUDA_AGENT_INTEGRATION.md](docs/integration/CUDA_AGENT_INTEGRATION.md)
3. **Create custom kernels** using the KernelRequestBuilder
4. **Integrate with Motor Cortex** for agentic video workflows

## 💡 Tips

- Start with pre-built effects before creating custom kernels
- Use `OptimizationTarget::Balanced` for general use
- Enable GPU acceleration only if you have NVIDIA GPU
- Latent optimizer reduces memory for long videos
- Check `.synoid_cuda_cache/` for compiled kernels

---

**Built with 🦀 Rust + ⚡ CUDA**

*Copyright (c) 2026 xingxerx_The_Creator | SYNOID*
