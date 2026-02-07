// SYNOID GPU Backend - Unified GPU Acceleration Layer
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Provides automatic GPU detection and fallback: CUDA → wgpu → CPU

use std::sync::Arc;
use tracing::{info, warn};

/// GPU Backend Selection
#[derive(Debug, Clone)]
pub enum GpuBackend {
    /// NVIDIA CUDA (fastest for RTX GPUs)
    Cuda { device_id: u32, name: String, vram_mb: u64 },
    /// wgpu (cross-platform: Vulkan/DX12/Metal)
    Wgpu { adapter_name: String },
    /// CPU fallback (rayon parallel)
    Cpu { threads: usize },
}

impl std::fmt::Display for GpuBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuBackend::Cuda { device_id, name, vram_mb } => {
                write!(f, "CUDA[{}]: {} ({} MB)", device_id, name, vram_mb)
            }
            GpuBackend::Wgpu { adapter_name } => write!(f, "wgpu: {}", adapter_name),
            GpuBackend::Cpu { threads } => write!(f, "CPU ({} threads)", threads),
        }
    }
}

/// GPU Context for unified processing
pub struct GpuContext {
    pub backend: GpuBackend,
    /// wgpu device (if using wgpu backend)
    pub wgpu_device: Option<Arc<wgpu::Device>>,
    pub wgpu_queue: Option<Arc<wgpu::Queue>>,
}

impl GpuContext {
    /// Detect and initialize the best available GPU backend
    pub async fn auto_detect() -> Self {
        // Try CUDA first (NVIDIA GPUs)
        if let Some(cuda_ctx) = Self::try_cuda() {
            return cuda_ctx;
        }

        // Fall back to wgpu (Vulkan/DX12/Metal)
        if let Some(wgpu_ctx) = Self::try_wgpu().await {
            return wgpu_ctx;
        }

        // Final fallback: CPU
        let threads = num_cpus::get();
        warn!("[GPU] No GPU detected. Falling back to CPU ({} threads)", threads);
        Self {
            backend: GpuBackend::Cpu { threads },
            wgpu_device: None,
            wgpu_queue: None,
        }
    }

    /// Try to initialize CUDA backend
    fn try_cuda() -> Option<Self> {
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::CudaDevice;
            match CudaDevice::new(0) {
                Ok(device) => {
                    let props = device.device_properties();
                    let name = props.name().unwrap_or("Unknown GPU".to_string());
                    let vram_mb = props.total_global_mem() / (1024 * 1024);
                    
                    info!("[GPU] ✓ CUDA initialized: {} ({} MB VRAM)", name, vram_mb);
                    return Some(Self {
                        backend: GpuBackend::Cuda { device_id: 0, name, vram_mb },
                        wgpu_device: None,
                        wgpu_queue: None,
                    });
                }
                Err(e) => {
                    warn!("[GPU] CUDA init failed: {}", e);
                }
            }
        }
        
        // CUDA not available or failed
        None
    }

    /// Try to initialize wgpu backend
    async fn try_wgpu() -> Option<Self> {
        let instance = wgpu::Instance::default();
        
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }).await?;

        let adapter_info = adapter.get_info();
        let adapter_name = adapter_info.name.clone();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("SYNOID GPU"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ).await.ok()?;

        info!("[GPU] ✓ wgpu initialized: {} ({:?})", adapter_name, adapter_info.backend);
        
        Some(Self {
            backend: GpuBackend::Wgpu { adapter_name },
            wgpu_device: Some(Arc::new(device)),
            wgpu_queue: Some(Arc::new(queue)),
        })
    }

    /// Check if we have GPU acceleration available
    pub fn has_gpu(&self) -> bool {
        !matches!(self.backend, GpuBackend::Cpu { .. })
    }

    /// Get the number of parallel workers for this backend
    pub fn parallel_workers(&self) -> usize {
        match &self.backend {
            GpuBackend::Cuda { .. } => 1,  // GPU handles parallelism internally
            GpuBackend::Wgpu { .. } => 1,  // GPU handles parallelism internally
            GpuBackend::Cpu { threads } => *threads,
        }
    }

    /// Get FFmpeg encoder for this backend
    pub fn ffmpeg_encoder(&self) -> &'static str {
        match &self.backend {
            GpuBackend::Cuda { .. } => "h264_nvenc",  // NVIDIA hardware encoder
            GpuBackend::Wgpu { adapter_name } => {
                // Check for Intel/AMD GPU encoders
                if adapter_name.to_lowercase().contains("intel") {
                    "h264_qsv"  // Intel Quick Sync
                } else if adapter_name.to_lowercase().contains("amd") {
                    "h264_amf"  // AMD AMF
                } else {
                    "libx264"   // Software fallback
                }
            }
            GpuBackend::Cpu { .. } => "libx264",  // Software encoder
        }
    }

    /// Get FFmpeg hardware acceleration flag
    pub fn ffmpeg_hwaccel(&self) -> Option<&'static str> {
        match &self.backend {
            GpuBackend::Cuda { .. } => Some("cuda"),
            GpuBackend::Wgpu { adapter_name } => {
                if adapter_name.to_lowercase().contains("intel") {
                    Some("qsv")
                } else {
                    None
                }
            }
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
    println!("Hardware Acceleration: {}", if ctx.has_gpu() { "✓ ENABLED" } else { "✗ DISABLED" });
    println!("FFmpeg Encoder: {}", ctx.ffmpeg_encoder());
    if let Some(hwaccel) = ctx.ffmpeg_hwaccel() {
        println!("FFmpeg HW Accel: {}", hwaccel);
    }
    println!("Parallel Workers: {}", ctx.parallel_workers());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpu_detection() {
        let ctx = GpuContext::auto_detect().await;
        // Should always succeed (falls back to CPU)
        println!("Detected: {}", ctx.backend);
        assert!(ctx.parallel_workers() > 0);
    }
}
