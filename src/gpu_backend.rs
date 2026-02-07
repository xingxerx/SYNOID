// SYNOID GPU Backend - Unified GPU Acceleration Layer
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Provides GPU detection via cudarc (CUDA 13.0), FFmpeg NVENC, and wgpu
// CUDA 13.0 supports RTX 50 series (sm_120)

use std::process::Command;
use std::sync::Arc;
use tracing::{info, warn};

/// GPU Backend Selection (priority: CUDA → NVENC → wgpu → CPU)
#[derive(Debug, Clone)]
pub enum GpuBackend {
    /// Native CUDA via cudarc (compute + encoding)
    Cuda { device_name: String, compute_capability: (u32, u32), memory_mb: u64 },
    /// NVIDIA GPU with NVENC (encoding only, no compute)
    NvencGpu { name: String, driver_version: String },
    /// wgpu (cross-platform: Vulkan/DX12/Metal)
    Wgpu { adapter_name: String },
    /// CPU fallback (rayon parallel)
    Cpu { threads: usize },
}

impl std::fmt::Display for GpuBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuBackend::Cuda { device_name, compute_capability, memory_mb } => {
                write!(f, "CUDA: {} (sm_{}{}, {} MB)", device_name, compute_capability.0, compute_capability.1, memory_mb)
            }
            GpuBackend::NvencGpu { name, driver_version } => {
                write!(f, "NVENC: {} (Driver {})", name, driver_version)
            }
            GpuBackend::Wgpu { adapter_name } => write!(f, "wgpu: {}", adapter_name),
            GpuBackend::Cpu { threads } => write!(f, "CPU ({} threads)", threads),
        }
    }
}

/// CUDA Context for native GPU compute (via cudarc)
#[derive(Clone)]
pub struct CudaContext {
    pub device: Arc<cudarc::driver::CudaDevice>,
}

impl CudaContext {
    /// Try to initialize CUDA with cudarc
    pub fn try_init() -> Option<(Self, GpuBackend)> {
        // Initialize CUDA driver
        cudarc::driver::result::init().ok()?;
        
        // Get device count
        let device_count = cudarc::driver::result::device::get_count().ok()?;
        if device_count == 0 {
            return None;
        }
        
        // Get first device
        let device = cudarc::driver::CudaDevice::new(0).ok()?;
        
        // Get device properties
        let device_name = cudarc::driver::result::device::get_name(0).unwrap_or_else(|_| "Unknown GPU".to_string());
        let (major, minor) = cudarc::driver::result::device::get_attribute(
            cudarc::driver::sys::CUdevice_attribute::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR,
            0,
        ).ok().and_then(|maj| {
            cudarc::driver::result::device::get_attribute(
                cudarc::driver::sys::CUdevice_attribute::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR,
                0,
            ).ok().map(|min| (maj as u32, min as u32))
        }).unwrap_or((0, 0));
        
        // Get total memory
        let total_mem = device.total_memory().unwrap_or(0) / (1024 * 1024); // Convert to MB
        
        info!("[GPU] ✓ CUDA initialized: {} (sm_{}{}, {} MB)", device_name, major, minor, total_mem);
        
        Some((
            CudaContext { device: Arc::new(device) },
            GpuBackend::Cuda { 
                device_name, 
                compute_capability: (major, minor),
                memory_mb: total_mem as u64,
            }
        ))
    }
}

/// GPU Context for unified processing
pub struct GpuContext {
    pub backend: GpuBackend,
    /// Native CUDA device (if using CUDA backend)
    pub cuda_ctx: Option<CudaContext>,
    /// wgpu device (if using wgpu backend)
    pub wgpu_device: Option<Arc<wgpu::Device>>,
    pub wgpu_queue: Option<Arc<wgpu::Queue>>,
}

impl GpuContext {
    /// Detect and initialize the best available GPU backend
    /// Priority: CUDA (compute+encode) → NVENC (encode) → wgpu → CPU
    pub async fn auto_detect() -> Self {
        // Try native CUDA first (full GPU compute + encoding)
        if let Some((cuda_ctx, backend)) = CudaContext::try_init() {
            return Self {
                backend,
                cuda_ctx: Some(cuda_ctx),
                wgpu_device: None,
                wgpu_queue: None,
            };
        }
        
        // Fall back to NVIDIA NVENC (encoding only, via nvidia-smi)
        if let Some(nvenc_ctx) = Self::try_nvenc() {
            return nvenc_ctx;
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
            cuda_ctx: None,
            wgpu_device: None,
            wgpu_queue: None,
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
            info!("[GPU] FFmpeg NVENC hardware encoding available");
            
            return Some(Self {
                backend: GpuBackend::NvencGpu { name, driver_version },
                cuda_ctx: None,
                wgpu_device: None,
                wgpu_queue: None,
            });
        }

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
            cuda_ctx: None,
            wgpu_device: Some(Arc::new(device)),
            wgpu_queue: Some(Arc::new(queue)),
        })
    }

    /// Check if we have GPU acceleration available
    pub fn has_gpu(&self) -> bool {
        !matches!(self.backend, GpuBackend::Cpu { .. })
    }

    /// Check if NVENC is available (includes CUDA backend)
    pub fn has_nvenc(&self) -> bool {
        matches!(self.backend, GpuBackend::Cuda { .. } | GpuBackend::NvencGpu { .. })
    }

    /// Check if native CUDA compute is available
    pub fn has_cuda(&self) -> bool {
        matches!(self.backend, GpuBackend::Cuda { .. })
    }

    /// Get the number of parallel workers for this backend
    pub fn parallel_workers(&self) -> usize {
        match &self.backend {
            GpuBackend::Cuda { .. } => 1,  // GPU handles parallelism internally
            GpuBackend::NvencGpu { .. } => 1,  // GPU handles parallelism internally
            GpuBackend::Wgpu { .. } => 1,  // GPU handles parallelism internally
            GpuBackend::Cpu { threads } => *threads,
        }
    }

    /// Get FFmpeg encoder for this backend
    pub fn ffmpeg_encoder(&self) -> &'static str {
        match &self.backend {
            GpuBackend::Cuda { .. } => "h264_nvenc",  // NVIDIA hardware encoder
            GpuBackend::NvencGpu { .. } => "h264_nvenc",  // NVIDIA hardware encoder
            GpuBackend::Wgpu { adapter_name } => {
                // Check for Intel/AMD GPU encoders
                let name_lower = adapter_name.to_lowercase();
                if name_lower.contains("intel") {
                    "h264_qsv"  // Intel Quick Sync
                } else if name_lower.contains("amd") || name_lower.contains("radeon") {
                    "h264_amf"  // AMD AMF
                } else {
                    "libx264"   // Software fallback
                }
            }
            GpuBackend::Cpu { .. } => "libx264",  // Software encoder
        }
    }

    /// Get FFmpeg hardware acceleration flag for decoding
    pub fn ffmpeg_hwaccel(&self) -> Option<&'static str> {
        match &self.backend {
            GpuBackend::Cuda { .. } => Some("cuda"),
            GpuBackend::NvencGpu { .. } => Some("cuda"),
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

    /// Get NVENC preset for quality/speed balance
    pub fn nvenc_preset(&self) -> &'static str {
        "p4"  // Balanced preset (p1=fastest, p7=best quality)
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
    println!("NVENC Available: {}", if ctx.has_nvenc() { "✓ YES" } else { "✗ NO" });
    println!("FFmpeg Encoder: {}", ctx.ffmpeg_encoder());
    if let Some(hwaccel) = ctx.ffmpeg_hwaccel() {
        println!("FFmpeg HW Accel: {}", hwaccel);
    }
    println!("Parallel Workers: {}", ctx.parallel_workers());
    
    // Additional info for NVIDIA
    if ctx.has_nvenc() {
        println!("\n[Note] RTX 50 series CUDA compute (sm_120) not yet supported");
        println!("       by Rust ML libs. Using FFmpeg NVENC for GPU encoding.");
        println!("       Whisper transcription uses CPU mode for reliability.");
    }
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
