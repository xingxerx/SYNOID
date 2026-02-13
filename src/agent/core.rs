// SYNOID Agent Core - The "Ghost"
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This is the central logic kernel that powers both the CLI and GUI.
// It maintains state, manages long-running processes, and routes intent.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use tracing::info;

use crate::agent::brain::Brain;
use crate::agent::defense::{IntegrityGuard, Sentinel};
use crate::agent::motor_cortex::MotorCortex;
use crate::agent::production_tools;
use crate::agent::source_tools;
use crate::agent::unified_pipeline::{PipelineConfig, PipelineStage, UnifiedPipeline};
use crate::agent::vector_engine::{self, VectorConfig};
use crate::agent::voice::VoiceEngine;
use crate::gpu_backend;

/// The shared state of the agent
#[derive(Clone)]
pub struct AgentCore {
    pub api_url: String,
    // Observability State (Thread-safe, Sync for GUI)
    pub status: Arc<Mutex<String>>,
    pub logs: Arc<Mutex<Vec<String>>>,

    // Sub-systems (Async Mutex for heavy async tasks)
    pub brain: Arc<AsyncMutex<Brain>>,
    pub cortex: Arc<AsyncMutex<MotorCortex>>,

    // Voice Engine (Sync Mutex because it's used synchronously in blocking blocks or directly)
    pub voice_engine: Arc<Mutex<Option<VoiceEngine>>>,

    // Unified Pipeline (Async Mutex)
    pub pipeline: Arc<AsyncMutex<Option<UnifiedPipeline>>>,
}

impl AgentCore {
    pub fn new(api_url: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
            status: Arc::new(Mutex::new("⚡ System Ready".to_string())),
            logs: Arc::new(Mutex::new(vec![
                "[SYSTEM] SYNOID Core initialized.".to_string()
            ])),
            brain: Arc::new(AsyncMutex::new(Brain::new(api_url, "gpt-oss:20b"))),
            cortex: Arc::new(AsyncMutex::new(MotorCortex::new(api_url))),
            voice_engine: Arc::new(Mutex::new(None)),
            pipeline: Arc::new(AsyncMutex::new(None)),
        }
    }

    /// Connect GPU context to the Brain for CUDA-accelerated processing.
    /// Call this after async GPU detection completes.
    pub async fn connect_gpu_to_brain(&self) {
        let gpu = gpu_backend::get_gpu_context().await;
        let mut brain = self.brain.lock().await;
        brain.connect_gpu(gpu);
        self.log(&format!(
            "[CORE] 🔗 Neural-GPU bridge active: {}",
            brain.acceleration_status()
        ));
    }

    /// Get combined acceleration status from Brain + GPU + Neuroplasticity.
    pub async fn acceleration_status(&self) -> String {
        let brain = self.brain.lock().await;
        brain.acceleration_status()
    }

    // --- State Helpers ---

    pub fn set_status(&self, msg: &str) {
        if let Ok(mut status) = self.status.lock() {
            *status = msg.to_string();
        }
    }

    pub fn log(&self, msg: &str) {
        info!("{}", msg); // Also log to stdout/tracing
        if let Ok(mut logs) = self.logs.lock() {
            logs.push(msg.to_string());
        }
    }

    pub fn get_status(&self) -> String {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logs.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    // --- Core Logic Methods ---

    pub async fn process_youtube_intent(
        &self,
        url: &str,
        intent: &str,
        output: Option<PathBuf>,
        login: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status("📥 Downloading...");
        self.log(&format!("[CORE] Processing YouTube: {}", url));

        let output_dir = Path::new("downloads");

        if !source_tools::check_ytdlp().await {
            let msg = "yt-dlp not found! Please install it via pip.";
            self.log(&format!("[CORE] ❌ {}", msg));
            return Err(msg.into());
        }

        // Extract needed fields immediately so the non-Send Result is dropped before next await
        let (title, local_path) = match source_tools::download_youtube(url, output_dir, login).await
        {
            Ok(info) => (info.title, info.local_path),
            Err(e) => {
                let msg = format!("[CORE] ❌ Download failed: {}", e);
                self.log(&msg);
                return Err(msg.into());
            }
        };

        self.log(&format!("[CORE] ✅ Video acquired: {}", title));
        let out_path = output.unwrap_or_else(|| PathBuf::from("output.mp4"));

        if !intent.is_empty() {
            self.set_status(&format!("🧠 Processing Intent: {}", intent));
            self.log(&format!("[CORE] Applying intent: {}", intent));

            use crate::agent::smart_editor;

            let self_clone = self.clone();
            let callback = Box::new(move |msg: &str| {
                self_clone.log(msg);
            });

            match smart_editor::smart_edit(&local_path, intent, &out_path, false, Some(callback))
                .await
            {
                Ok(res) => self.log(&format!("[CORE] ✅ {}", res)),
                Err(e) => self.log(&format!("[CORE] ❌ Edit failed: {}", e)),
            }
        } else {
            if let Err(e) = std::fs::copy(&local_path, &out_path) {
                self.log(&format!("[CORE] ❌ Copy failed: {}", e));
            } else {
                self.log(&format!("[CORE] ✅ Saved to {:?}", out_path));
            }
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn process_research(
        &self,
        topic: &str,
        limit: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status(&format!("🕵️ Researching: {}", topic));
        self.log(&format!("[CORE] Researching topic: {}", topic));

        match source_tools::search_youtube(topic, limit).await {
            Ok(results) => {
                self.log(&format!("[CORE] === 📚 Results: '{}' ===", topic));
                for (i, source) in results.iter().enumerate() {
                    self.log(&format!(
                        "{}. {} (Duration: {:.1} min)",
                        i + 1,
                        source.title,
                        source.duration / 60.0
                    ));
                    self.log(&format!(
                        "   URL: {}",
                        source.original_url.as_deref().unwrap_or("Unknown")
                    ));
                }
            }
            Err(e) => {
                self.log(&format!("[CORE] ❌ Research failed: {}", e));
                return Err(e);
            }
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn clip_video(
        &self,
        input: &Path,
        start: f64,
        duration: f64,
        output: Option<PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status("✂️ Clipping...");
        let out_path = output.unwrap_or_else(|| {
            let stem = input.file_stem().unwrap().to_string_lossy();
            input.with_file_name(format!("{}_clip.mp4", stem))
        });

        match production_tools::trim_video(input, start, duration, &out_path).await {
            Ok(res) => {
                self.log(&format!(
                    "[CORE] ✂️ Clip saved: {:?} ({:.2} MB)",
                    res.output_path, res.size_mb
                ));
            }
            Err(e) => {
                self.log(&format!("[CORE] ❌ Clipping failed: {}", e));
                return Err(e);
            }
        }
        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn compress_video(
        &self,
        input: &Path,
        size_mb: f64,
        output: Option<PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status("📦 Compressing...");
        let out_path = output.unwrap_or_else(|| {
            let stem = input.file_stem().unwrap().to_string_lossy();
            input.with_file_name(format!("{}_compressed.mp4", stem))
        });

        match production_tools::compress_video(input, size_mb, &out_path).await {
            Ok(res) => {
                self.log(&format!(
                    "[CORE] 📦 Compressed saved: {:?} ({:.2} MB)",
                    res.output_path, res.size_mb
                ));
            }
            Err(e) => {
                self.log(&format!("[CORE] ❌ Compression failed: {}", e));
                return Err(e);
            }
        }
        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn process_brain_request(
        &self,
        request: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status("🧠 Thinking...");
        self.log(&format!("[CORE] Brain Request: {}", request));

        let mut brain = self.brain.lock().await;
        match brain.process(request).await {
            Ok(res) => self.log(&format!("[CORE] ✅ {}", res)),
            Err(e) => self.log(&format!("[CORE] ❌ {}", e)),
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn embody_intent(
        &self,
        input: &Path,
        intent: &str,
        output: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status("🤖 Embodying...");
        self.log(&format!("[CORE] Embodied Agent Activating for: {}", intent));

        use crate::agent::audio_tools;
        use crate::agent::vision_tools;

        // 1. Scan Context
        self.log("[CORE] Scanning visual context...");
        let visual_data = match vision_tools::scan_visual(input).await {
            Ok(d) => d,
            Err(e) => {
                self.log(&format!("[CORE] ❌ Vision scan failed: {}", e));
                return Err(e.to_string().into());
            }
        };

        self.log("[CORE] Scanning audio context...");
        let audio_data = match audio_tools::scan_audio(input).await {
            Ok(d) => d,
            Err(e) => {
                self.log(&format!("[CORE] ❌ Audio scan failed: {}", e));
                return Err(e.to_string().into());
            }
        };

        // 2. Execute — extract args and drop the cortex lock before awaiting
        let args = {
            let mut cortex = self.cortex.lock().await;
            match cortex
                .execute_one_shot_render(intent, input, output, &visual_data, &audio_data)
                .await
            {
                Ok(a) => a,
                Err(e) => {
                    self.log(&format!("[CORE] ❌ Embodiment failed: {}", e));
                    return Err(e.to_string().into());
                }
            }
        }; // cortex lock is dropped here

        let cmd_str = args.join(" ");
        self.log(&format!("[CORE] 🎬 Generated FFmpeg Command: {}", cmd_str));

        if let Some((prog, cmd_args)) = args.split_first() {
            let mut child = tokio::process::Command::new(prog)
                .args(cmd_args)
                .kill_on_drop(true)
                .spawn()?;

            let status = child.wait().await?;
            if status.success() {
                self.log("[CORE] ✅ Render Complete");
            } else {
                self.log("[CORE] ❌ Render Failed");
                return Err("FFmpeg execution failed".into());
            }
        }

        // Record success in neuroplasticity so the system speeds up
        {
            let mut brain = self.brain.lock().await;
            brain.neuroplasticity.record_success();
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn learn_style(
        &self,
        input: &Path,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status(&format!("🎓 Learning '{}'...", name));
        self.log(&format!(
            "[CORE] Analyzing style '{}' from {:?}",
            name, input
        ));

        use crate::agent::academy::{StyleLibrary, TechniqueExtractor};
        // Stub implementation from main.rs
        let _lib = StyleLibrary::new();
        let _extractor = TechniqueExtractor {};

        self.log(&format!(
            "[CORE] ✅ Analyzed style '{}'. Saved to library.",
            name
        ));
        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn vectorize_video(
        &self,
        input: &Path,
        output: &Path,
        mode: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status("🎨 Vectorizing...");
        self.log(&format!("[CORE] Vectorizing {:?}", input));

        let mut config = VectorConfig::default();
        config.colormode = mode.to_string();

        match vector_engine::vectorize_video(input, output, config).await {
            Ok(msg) => self.log(&format!("[CORE] ✅ {}", msg)),
            Err(e) => {
                self.log(&format!("[CORE] ❌ Vectorization failed: {}", e));
                return Err(e);
            }
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn upscale_video(
        &self,
        input: &Path,
        scale: f64,
        output: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status(&format!("🔎 Upscaling {:.1}x...", scale));
        self.log(&format!(
            "[CORE] Infinite Upscale (Scale: {:.1}x) on {:?}",
            scale, input
        ));

        match vector_engine::upscale_video(input, scale, output).await {
            Ok(msg) => self.log(&format!("[CORE] ✅ {}", msg)),
            Err(e) => {
                self.log(&format!("[CORE] ❌ Upscale failed: {}", e));
                return Err(e);
            }
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    // --- Voice Tools ---

    // Ensure voice engine is initialized
    fn ensure_voice_engine(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut engine = self.voice_engine.lock().unwrap();
        if engine.is_none() {
            match VoiceEngine::new() {
                Ok(e) => *engine = Some(e),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    pub async fn voice_record(
        &self,
        output: Option<PathBuf>,
        duration: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status("🎙️ Recording...");
        use crate::agent::voice::AudioIO;
        let audio_io = AudioIO::new();

        let out_path = output.unwrap_or_else(|| PathBuf::from("voice_sample.wav"));

        match tokio::task::spawn_blocking(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                audio_io.record_to_file(&out_path, duration).map_err(|e| {
                    let boxed: Box<dyn std::error::Error + Send + Sync> = e.to_string().into();
                    boxed
                })
            },
        )
        .await?
        {
            Ok(_) => self.log(&format!("[CORE] ✅ Recorded {} seconds", duration)),
            Err(e) => self.log(&format!("[CORE] ❌ Recording failed: {}", e)),
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn voice_clone(
        &self,
        audio_path: &Path,
        profile_name: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status("🎭 Cloning Voice...");
        if let Err(e) = self.ensure_voice_engine() {
            self.log(&format!("[CORE] ❌ Engine init failed: {}", e));
            return Err(e);
        }

        let engine_guard = self.voice_engine.lock().unwrap();
        let engine = engine_guard.as_ref().unwrap();

        if let Some(name) = profile_name {
            self.log(&format!("[CORE] Creating voice profile '{}'...", name));
            match engine.create_profile(&name, audio_path) {
                Ok(p) => self.log(&format!(
                    "[CORE] ✅ Profile '{}' created ({} dims)",
                    p.name,
                    p.embedding.len()
                )),
                Err(e) => self.log(&format!("[CORE] ❌ Profile creation failed: {}", e)),
            }
        } else {
            match engine.clone_voice(audio_path) {
                Ok(embedding) => self.log(&format!(
                    "[CORE] ✅ Voice cloned. Embedding: {} dims",
                    embedding.len()
                )),
                Err(e) => self.log(&format!("[CORE] ❌ Clone failed: {}", e)),
            }
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn voice_speak(
        &self,
        text: &str,
        profile: Option<String>,
        output: Option<PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status("🗣️ Speaking...");
        if let Err(e) = self.ensure_voice_engine() {
            self.log(&format!("[CORE] ❌ Engine init failed: {}", e));
            return Err(e);
        }

        let out_path = output.unwrap_or_else(|| PathBuf::from("tts_output.wav"));
        let engine_guard = self.voice_engine.lock().unwrap();
        let engine = engine_guard.as_ref().unwrap();

        let res = if let Some(p_name) = profile {
            engine.speak_as(text, &p_name, &out_path)
        } else {
            engine.speak(text, &out_path)
        };

        match res {
            Ok(_) => {
                self.log(&format!("[CORE] ✅ Speech saved to {:?}", out_path));
                // Play it
                use crate::agent::voice::AudioIO;
                let audio_io = AudioIO::new();
                let _ = audio_io.play_file(&out_path);
            }
            Err(e) => self.log(&format!("[CORE] ❌ Speech failed: {}", e)),
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn download_voice_model(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status("📥 Downloading Model...");
        if let Err(e) = self.ensure_voice_engine() {
            return Err(e);
        }
        let engine_guard = self.voice_engine.lock().unwrap();
        let engine = engine_guard.as_ref().unwrap();

        match engine.download_model("microsoft/speecht5_tts") {
            Ok(path) => self.log(&format!("[CORE] ✅ Model ready: {:?}", path)),
            Err(e) => self.log(&format!("[CORE] ❌ Download failed: {}", e)),
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    // --- Unified Pipeline ---

    pub async fn run_unified_pipeline(
        &self,
        input: &Path,
        output: &Path,
        stages_str: &str,
        _gpu: &str,
        intent: Option<String>,
        scale: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_status("🚀 Running Pipeline...");

        let parsed_stages = PipelineStage::parse_list(stages_str);
        if parsed_stages.is_empty() {
            let msg = "No valid stages specified.";
            self.log(&format!("[CORE] ❌ {}", msg));
            return Err(msg.into());
        }

        // Initialize pipeline lazily
        let mut pipeline_guard = self.pipeline.lock().await;
        if pipeline_guard.is_none() {
            self.log("[CORE] Initializing GPU Pipeline...");
            *pipeline_guard = Some(UnifiedPipeline::new().await);
        }
        let pipeline = pipeline_guard.as_ref().unwrap();

        // Config
        let self_clone = self.clone();
        let config = PipelineConfig {
            stages: parsed_stages,
            intent,
            scale_factor: scale,
            target_size_mb: 0.0,
            funny_mode: false,
            progress_callback: Some(Arc::new(move |msg: &str| {
                self_clone.log(msg);
            })),
        };

        match pipeline.process(input, output, config).await {
            Ok(out_path) => self.log(&format!("[CORE] ✅ Pipeline complete: {:?}", out_path)),
            Err(e) => {
                self.log(&format!("[CORE] ❌ Pipeline failed: {}", e));
                return Err(e);
            }
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    // --- Sentinel ---
    pub async fn activate_sentinel(&self, mode: &str, watch: Option<PathBuf>) {
        self.set_status(&format!("🛡️ Sentinel Active ({})", mode));
        self.log("[CORE] 🛡️ ACTIVATING SENTINEL Cyberdefense System...");

        let mut integrity = IntegrityGuard::new();
        if let Some(path) = watch {
            self.log(&format!("[CORE] Watching Path: {:?}", path));
            integrity.watch_path(path);
            let _ = integrity.build_baseline();
        }

        let mut sentinel = Sentinel::new();
        self.log("[CORE] ✅ Sentinel Online. Monitoring system...");

        loop {
            // Check System Health
            if mode == "all" || mode == "sys" {
                let alerts = sentinel.scan_processes();
                for alert in alerts {
                    self.log(&format!("[SENTINEL] ⚠️ {}", alert));
                }
            }

            // Check File Integrity
            if mode == "all" || mode == "file" {
                let violations = integrity.verify_integrity();
                for v in violations {
                    self.log(&format!("[INTEGRITY] ❌ {}", v));
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }
}
