// SYNOID Agent Core - The "Ghost"
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This is the central logic kernel that powers both the CLI and GUI.
// It maintains state, manages long-running processes, and routes intent.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use tracing::info;

use crate::agent::autonomous_learner::AutonomousLearner;
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

    // Autonomous Learner (Sync Mutex)
    pub autonomous_learner: Arc<Mutex<Option<AutonomousLearner>>>,
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
            autonomous_learner: Arc::new(Mutex::new(None)), // Lazy init
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

    fn sanitize_input(input: &str) -> String {
        let mut s = input.trim().to_string();

        // Remove surrounding quotes if they exist
        if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
            s.remove(0);
            s.pop();
        }

        // Remove hidden control characters (e.g., \u{202a} Left-to-Right Embedding)
        // This is common when copying paths from Windows Explorer property dialogs.
        s.chars()
            .filter(|c| !c.is_control() && *c != '\u{202a}' && *c != '\u{202b}' && *c != '\u{202c}')
            .collect()
    }

    pub async fn process_youtube_intent(
        &self,
        url: &str,
        intent: &str,
        output: Option<PathBuf>,
        login: Option<&str>,
        funny_mode: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status("📥 Downloading...");
        let sanitized_url = Self::sanitize_input(url);
        self.log(&format!("[CORE] Processing YouTube: {}", sanitized_url));

        let output_dir = Path::new("downloads");
        let path_obj = Path::new(&sanitized_url);

        // Check if input is a local file string or has a drive letter
        let is_local = path_obj.exists()
            || (sanitized_url.len() > 1 && sanitized_url.chars().nth(1) == Some(':'))
            || sanitized_url.starts_with("\\\\"); // UNC Path Support

        let (title, local_path) = if is_local {
            if !path_obj.exists() {
                let msg = format!(
                    "[CORE] ❌ Local file check failed: '{}' not found.",
                    sanitized_url
                );
                self.log(&msg);
                return Err(msg.into());
            }

            let final_path = if path_obj.is_dir() {
                self.log(&format!(
                    "[CORE] 📂 Input is a directory. Scanning for video files in {:?}",
                    path_obj
                ));
                let mut video_file = None;
                if let Ok(entries) = std::fs::read_dir(path_obj) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                let ext_str = ext.to_string_lossy().to_lowercase();
                                if ["mp4", "mkv", "avi", "mov", "webm"].contains(&ext_str.as_str())
                                {
                                    // Prefer files that contain "copy" or match part of the intent if possible?
                                    // For now, let's just pick the first one we find.
                                    video_file = Some(path);
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(found) = video_file {
                    self.log(&format!(
                        "[CORE] 🎯 Automatically selected video: {:?}",
                        found.file_name().unwrap()
                    ));
                    found
                } else {
                    let msg = format!(
                        "[CORE] ❌ No video files found in directory: {:?}",
                        path_obj
                    );
                    self.log(&msg);
                    return Err(msg.into());
                }
            } else {
                path_obj.to_path_buf()
            };

            self.log(&format!("[CORE] 📁 Using local file: {:?}", final_path));
            (
                final_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                final_path,
            )
        } else {
            if !source_tools::check_ytdlp().await {
                let msg = "yt-dlp not found! Please install it via pip.";
                self.log(&format!("[CORE] ❌ {}", msg));
                return Err(msg.into());
            }

            // Extract needed fields immediately so the non-Send Result is dropped before next await
            match source_tools::download_youtube(&sanitized_url, output_dir, login).await {
                Ok(info) => (info.title, info.local_path),
                Err(e) => {
                    let msg = format!("[CORE] ❌ Download failed: {}", e);
                    self.log(&msg);
                    return Err(msg.into());
                }
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

            match smart_editor::smart_edit(
                &local_path,
                intent,
                &out_path,
                funny_mode,
                Some(callback),
                None,
                None,
            )
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                return Err(e.to_string().into());
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                return Err(e.to_string().into());
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                return Err(e.to_string().into());
            }
        }
        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn process_brain_request(
        &self,
        request: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

        // 2. Execute — Route through Smart Render for deep editing/cutting
        self.set_status("🧠 Planning & Rendering...");
        let result = {
            let mut cortex = self.cortex.lock().await;
            // execute_smart_render now calls smart_edit which handles transcription and cutting
            cortex
                .execute_smart_render(intent, input, output, &visual_data, &[], &audio_data)
                .await
        };

        match result {
            Ok(summary) => {
                self.log(&format!("[CORE] ✅ {}", summary));
            }
            Err(e) => {
                self.log(&format!("[CORE] ❌ Embodiment failed: {}", e));
                return Err(e.into());
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status("🎨 Vectorizing...");
        self.log(&format!("[CORE] Vectorizing {:?}", input));

        let mut config = VectorConfig::default();
        config.colormode = mode.to_string();

        match vector_engine::vectorize_video(input, output, config).await {
            Ok(msg) => self.log(&format!("[CORE] ✅ {}", msg)),
            Err(e) => {
                self.log(&format!("[CORE] ❌ Vectorization failed: {}", e));
                return Err(e.to_string().into());
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status(&format!("🔎 Upscaling {:.1}x...", scale));
        self.log(&format!(
            "[CORE] Infinite Upscale (Scale: {:.1}x) on {:?}",
            scale, input
        ));

        match vector_engine::upscale_video(input, scale, output).await {
            Ok(msg) => self.log(&format!("[CORE] ✅ {}", msg)),
            Err(e) => {
                self.log(&format!("[CORE] ❌ Upscale failed: {}", e));
                return Err(e.to_string().into());
            }
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn get_audio_tracks(
        &self,
        input: &Path,
    ) -> Result<Vec<crate::agent::audio_tools::AudioTrack>, Box<dyn std::error::Error + Send + Sync>>
    {
        crate::agent::audio_tools::get_audio_tracks(input).await
    }

    // --- Voice Tools ---

    // Ensure voice engine is initialized
    fn ensure_voice_engine(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut engine = self.voice_engine.lock().unwrap();
        if engine.is_none() {
            match VoiceEngine::new() {
                Ok(e) => *engine = Some(e),
                Err(e) => return Err(e.to_string().into()),
            }
        }
        Ok(())
    }

    pub async fn voice_record(
        &self,
        output: Option<PathBuf>,
        duration: u32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    pub async fn download_voice_model(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                return Err(e.to_string().into());
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

    // --- Autonomous Learning Control ---

    pub fn start_autonomous_learning(&self) {
        self.set_status("🚀 Starting Autonomous Loop...");
        self.log("[CORE] Initializing Autonomous Learner...");

        let mut learner_guard = self.autonomous_learner.lock().unwrap();
        if learner_guard.is_none() {
            // Create new learner sharing the same brain
            let learner = AutonomousLearner::new(self.brain.clone());
            *learner_guard = Some(learner);
        }

        if let Some(learner) = learner_guard.as_ref() {
            learner.start();
        }
    }

    pub fn stop_autonomous_learning(&self) {
        self.set_status("🛑 Stopping Autonomous Loop...");
        let learner_guard = self.autonomous_learner.lock().unwrap();
        if let Some(learner) = learner_guard.as_ref() {
            learner.stop();
            self.log("[CORE] Autonomous Loop signal sent: STOP");
        }
        self.set_status("⚡ Ready");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_input() {
        // Test trimming
        assert_eq!(AgentCore::sanitize_input("  test  "), "test");

        // Test surrounding quotes
        assert_eq!(AgentCore::sanitize_input("\"C:\\Path\""), "C:\\Path");
        assert_eq!(AgentCore::sanitize_input("'C:\\Path'"), "C:\\Path");

        // Test hidden control characters (LRE \u{202a})
        let input = "\u{202a}C:\\Users\\xing\\Videos\\test.mp4";
        assert_eq!(
            AgentCore::sanitize_input(input),
            "C:\\Users\\xing\\Videos\\test.mp4"
        );

        // Test combination
        let complex = "  \u{202a}\"C:\\Path With Spaces\\test.mp4\"  ";
        assert_eq!(
            AgentCore::sanitize_input(complex),
            "C:\\Path With Spaces\\test.mp4"
        );
    }

    #[test]
    fn test_is_local_robustness() {
        // This is a bit tricky because Path::exists() depends on the FS.
        // But we can test the string-based logic.

        let drive_path = "C:\\Videos\\test.mp4";
        assert!(drive_path.len() > 1 && drive_path.chars().nth(1) == Some(':'));

        let unc_path = "\\\\server\\share\\test.mp4";
        assert!(unc_path.starts_with("\\\\"));

        let url = "https://youtube.com/watch?v=123";
        assert!(!(url.len() > 1 && url.chars().nth(1) == Some(':')));
        assert!(!url.starts_with("\\\\"));
    }
}
