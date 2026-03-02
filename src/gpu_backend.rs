// SYNOID GPU Backend - FFmpeg NVENC Acceleration + Neuroplasticity Bridge
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Uses FFmpeg NVENC for GPU encoding - no Rust CUDA deps needed
// RTX 5080 (sm_120) not yet supported by cudarc/wgpu
//
// Neuroplasticity Integration: The Brain's adaptive speed multiplier
// tunes CUDA batch sizes, thread counts, and FFmpeg presets so the
// system gets faster as it learns.

use std::process::Command;
use tracing::{info, warn};

/// GPU Backend Selection
#[derive(Debug, Clone)]
pub enum GpuBackend {
    /// NVIDIA GPU with NVENC (detected via FFmpeg)
    NvencGpu {
        name: String,
        driver_version: String,
    },
    /// CPU fallback (rayon parallel)
    Cpu { threads: usize },
}

impl std::fmt::Display for GpuBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuBackend::NvencGpu {
                name,
                driver_version,
            } => {
                write!(f, "NVENC: {} (Driver {})", name, driver_version)
            }
            GpuBackend::Cpu { threads } => write!(f, "CPU ({} threads)", threads),
        }
    }
}

// ---------------------------------------------------------------------------
// CUDA Acceleration Config — tuned by Neuroplasticity
// ---------------------------------------------------------------------------

/// Parameters for CUDA-accelerated workloads, dynamically tuned by
/// the Neuroplasticity speed multiplier so the system becomes more
/// aggressive as it gains experience.
#[derive(Debug, Clone)]
pub struct CudaAccelConfig {
    /// How many frames / items to batch per GPU kernel launch.
    pub batch_size: usize,
    /// Number of concurrent CUDA streams (or FFmpeg threads).
    pub parallel_streams: usize,
    /// Whether to prefetch the next batch while the current one runs.
    pub prefetch_enabled: bool,
    /// FFmpeg encoding preset (faster presets for more experienced brains).
    pub ffmpeg_preset: &'static str,
    /// The neuroplasticity speed multiplier that produced this config.
    pub neuro_speed: f64,
}

impl Default for CudaAccelConfig {
    fn default() -> Self {
        Self {
            batch_size: 4,
            parallel_streams: 1,
            prefetch_enabled: false,
            ffmpeg_preset: "medium",
            neuro_speed: 1.0,
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
        warn!(
            "[GPU] No NVIDIA GPU detected. Using CPU ({} threads)",
            threads
        );
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

            info!(
                "[GPU] ✓ NVIDIA GPU detected: {} (Driver {})",
                name, driver_version
            );
            info!("[GPU] FFmpeg NVENC hardware encoding enabled");

            return Some(Self {
                backend: GpuBackend::NvencGpu {
                    name,
                    driver_version,
                },
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
            GpuBackend::NvencGpu { .. } => 1, // GPU handles parallelism internally
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

    // -------------------------------------------------------------------
    // Neuroplasticity-aware CUDA methods
    // -------------------------------------------------------------------

    /// Compute optimal CUDA acceleration parameters tuned by the
    /// Neuroplasticity speed multiplier.
    ///
    /// Higher speed → larger batches, more streams, faster FFmpeg preset.
    pub fn cuda_accel_config(&self, neuro_speed: f64) -> CudaAccelConfig {
        let base_batch = if self.has_gpu() { 8 } else { 4 };
        let batch_size = (base_batch as f64 * neuro_speed).min(128.0) as usize;

        let parallel_streams = if self.has_gpu() {
            (neuro_speed as usize).clamp(1, 8)
        } else {
            self.parallel_workers()
        };

        let prefetch_enabled = neuro_speed >= 2.0 && self.has_gpu();

        let ffmpeg_preset = match neuro_speed as u32 {
            0..=1 => "medium",
            2..=3 => "fast",
            4..=7 => "veryfast",
            8..=15 => "ultrafast",
            _ => "ultrafast",
        };

        CudaAccelConfig {
            batch_size,
            parallel_streams,
            prefetch_enabled,
            ffmpeg_preset,
            neuro_speed,
        }
    }

    /// Return extra FFmpeg CLI flags tuned by neuroplasticity.
    /// Faster brains get faster presets and more threads.
    pub fn neuroplastic_ffmpeg_flags(&self, neuro_speed: f64) -> Vec<String> {
        let cfg = self.cuda_accel_config(neuro_speed);
        let mut flags = Vec::new();

        flags.push("-preset".to_string());
        flags.push(cfg.ffmpeg_preset.to_string());

        flags.push("-threads".to_string());
        flags.push(cfg.parallel_streams.to_string());

        if cfg.prefetch_enabled {
            // Enable lookahead for NVENC when brain is fast enough
            if self.has_gpu() {
                flags.push("-rc-lookahead".to_string());
                flags.push("16".to_string());
            }
        }

        flags
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

/// Print GPU + Neuroplasticity combined status (for CLI `gpu` command)
pub async fn print_gpu_status() {
    let ctx = get_gpu_context().await;

    // Also load neuroplasticity state for combined readout
    let neuro = crate::agent::neuroplasticity::Neuroplasticity::new();
    let accel = ctx.cuda_accel_config(neuro.current_speed());

    println!("=== SYNOID GPU + Neural Acceleration Status ===");
    println!();
    println!("── GPU Backend ──");
    println!("  Backend       : {}", ctx.backend);
    println!(
        "  HW Encoding   : {}",
        if ctx.has_gpu() {
            "✓ NVENC"
        } else {
            "✗ CPU"
        }
    );
    println!("  FFmpeg Encoder : {}", ctx.ffmpeg_encoder());
    if let Some(hwaccel) = ctx.ffmpeg_hwaccel() {
        println!("  FFmpeg HW Accel: {}", hwaccel);
    }
    println!("  Workers        : {}", ctx.parallel_workers());

    println!();
    println!("── Neuroplasticity ──");
    println!("  Experience    : {} XP", neuro.experience_points);
    println!("  Speed         : {:.1}×", neuro.current_speed());
    println!("  Adaptation    : {}", neuro.adaptation_level());
    println!("  Adaptations   : {} doublings", neuro.adaptations);

    println!();
    println!("── CUDA Accel Config (Neural-Tuned) ──");
    println!("  Batch Size    : {}", accel.batch_size);
    println!("  Streams       : {}", accel.parallel_streams);
    println!(
        "  Prefetch      : {}",
        if accel.prefetch_enabled { "ON" } else { "OFF" }
    );
    println!("  FFmpeg Preset : {}", accel.ffmpeg_preset);

    if ctx.has_gpu() {
        println!();
        println!("[Note] RTX 50 series CUDA compute (sm_120) not yet supported");
        println!("       by Rust ML libs. Using FFmpeg NVENC for encoding.");
        println!("       Whisper transcription uses GPU mode for ultimate performance.");
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

    #[test]
    fn test_cuda_accel_config_baseline() {
        let ctx = GpuContext {
            backend: GpuBackend::Cpu { threads: 8 },
        };
        let cfg = ctx.cuda_accel_config(1.0);
        assert_eq!(cfg.batch_size, 4);
        assert!(!cfg.prefetch_enabled);
        assert_eq!(cfg.ffmpeg_preset, "medium");
    }

    #[test]
    fn test_cuda_accel_config_scales_with_speed() {
        let ctx = GpuContext {
            backend: GpuBackend::NvencGpu {
                name: "RTX 5080".to_string(),
                driver_version: "570.0".to_string(),
            },
        };

        // 4× speed brain
        let cfg = ctx.cuda_accel_config(4.0);
        assert_eq!(cfg.batch_size, 32); // 8 * 4
        assert!(cfg.prefetch_enabled);
        assert_eq!(cfg.ffmpeg_preset, "veryfast");

        // 16× speed brain (capped)
        let cfg = ctx.cuda_accel_config(16.0);
        assert_eq!(cfg.batch_size, 128); // 8 * 16
        assert_eq!(cfg.ffmpeg_preset, "ultrafast");
    }
}
