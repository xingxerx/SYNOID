// SYNOID GPU Backend - FFmpeg NVENC Acceleration
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Uses FFmpeg NVENC for GPU encoding - no Rust CUDA deps needed
// RTX 5080 (sm_120) not yet supported by cudarc/wgpu

use std::process::Command;
use tracing::{info, warn};

/// GPU Backend Selection
#[derive(Debug, Clone)]
pub enum GpuBackend {
    /// NVIDIA GPU with NVENC (detected via FFmpeg)
    NvencGpu { name: String, driver_version: String },
    /// CPU fallback (rayon parallel)
    Cpu { threads: usize },
}

impl std::fmt::Display for GpuBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuBackend::NvencGpu { name, driver_version } => {
                write!(f, "NVENC: {} (Driver {})", name, driver_version)
            }
            GpuBackend::Cpu { threads } => write!(f, "CPU ({} threads)", threads),
        }
    }
}

/// GPU Context for unified processing
pub struct GpuContext {
    pub backend: GpuBackend,
}

impl GpuContext {
    /// Detect and initialize the best available GPU backend
    pub async fn auto_detect() -> Self {
        // Try NVIDIA NVENC first (via nvidia-smi)
        if let Some(nvenc_ctx) = Self::try_nvenc() {
            return nvenc_ctx;
        }

        // Final fallback: CPU
        let threads = num_cpus::get();
        warn!("[GPU] No NVIDIA GPU detected. Using CPU ({} threads)", threads);
        Self {
            backend: GpuBackend::Cpu { threads },
        }
    }

    /// Try to detect NVIDIA GPU via nvidia-smi
    fn try_nvenc() -> Option<Self> {
        let output = Command::new("nvidia-smi")
            .args(["--query-gpu=name,driver_version", "--format=csv,noheader"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split(',').collect();
        
        if parts.len() >= 2 {
            let name = parts[0].trim().to_string();
            let driver_version = parts[1].trim().to_string();
            
            info!("[GPU] ✓ NVIDIA GPU detected: {} (Driver {})", name, driver_version);
            info!("[GPU] FFmpeg NVENC hardware encoding enabled");
            
            return Some(Self {
                backend: GpuBackend::NvencGpu { name, driver_version },
            });
        }

        None
    }

    /// Check if we have GPU acceleration available
    pub fn has_gpu(&self) -> bool {
        matches!(self.backend, GpuBackend::NvencGpu { .. })
    }

    /// Get the number of parallel workers for this backend
    pub fn parallel_workers(&self) -> usize {
        match &self.backend {
            GpuBackend::NvencGpu { .. } => 1,  // GPU handles parallelism internally
            GpuBackend::Cpu { threads } => *threads,
        }
    }

    /// Get FFmpeg encoder for this backend
    pub fn ffmpeg_encoder(&self) -> &'static str {
        match &self.backend {
            GpuBackend::NvencGpu { .. } => "h264_nvenc",
            GpuBackend::Cpu { .. } => "libx264",
        }
    }

    /// Get FFmpeg hardware acceleration flag for decoding
    pub fn ffmpeg_hwaccel(&self) -> Option<&'static str> {
        match &self.backend {
            GpuBackend::NvencGpu { .. } => Some("cuda"),
            GpuBackend::Cpu { .. } => None,
        }
    }
}

/// Global GPU context accessor
static GPU_CONTEXT: std::sync::OnceLock<GpuContext> = std::sync::OnceLock::new();

/// Get or initialize the global GPU context
pub async fn get_gpu_context() -> &'static GpuContext {
    if let Some(ctx) = GPU_CONTEXT.get() {
        return ctx;
    }
    
    let ctx = GpuContext::auto_detect().await;
    GPU_CONTEXT.get_or_init(|| ctx)
}

/// Print GPU status (for CLI `gpu` command)
pub async fn print_gpu_status() {
    let ctx = get_gpu_context().await;
    
    println!("=== SYNOID GPU Status ===");
    println!("Backend: {}", ctx.backend);
    println!("Hardware Encoding: {}", if ctx.has_gpu() { "✓ NVENC" } else { "✗ CPU" });
    println!("FFmpeg Encoder: {}", ctx.ffmpeg_encoder());
    if let Some(hwaccel) = ctx.ffmpeg_hwaccel() {
        println!("FFmpeg HW Accel: {}", hwaccel);
    }
    println!("Parallel Workers: {}", ctx.parallel_workers());
    
    if ctx.has_gpu() {
        println!("\n[Note] RTX 50 series CUDA compute (sm_120) not yet supported");
        println!("       by Rust ML libs. Using FFmpeg NVENC for encoding.");
        println!("       Whisper transcription uses CPU mode for reliability.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpu_detection() {
        let ctx = GpuContext::auto_detect().await;
        println!("Detected: {}", ctx.backend);
        assert!(ctx.parallel_workers() > 0);
    }
}
