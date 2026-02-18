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
use crate::agent::motor_cortex::MotorCortex;
use crate::agent::production_tools;
use crate::agent::source_tools;
use crate::agent::unified_pipeline::{PipelineConfig, PipelineStage, UnifiedPipeline};
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
            pipeline: Arc::new(AsyncMutex::new(None)),
        }
    }

    /// Connect GPU context to the Brain for CUDA-accelerated processing.
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
        info!("{}", msg);
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
        chunk_minutes: u32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if chunk_minutes > 0 && chunk_minutes < 600 {
            self.log(&format!("[CORE] ℹ️ Note: Long video chunking ({} mins) requested but experimental. Proceeding with full video.", chunk_minutes));
        }

        self.set_status("📥 Downloading...");
        let sanitized_url = Self::sanitize_input(url);
        self.log(&format!("[CORE] Processing YouTube: {}", sanitized_url));

        let output_dir = Path::new("downloads");
        let path_obj = Path::new(&sanitized_url);

        let is_local = path_obj.exists()
            || (sanitized_url.len() > 1 && sanitized_url.chars().nth(1) == Some(':'))
            || sanitized_url.starts_with("\\\\");

        let (title, local_path) = if is_local {
            if !path_obj.exists() {
                let msg = format!("[CORE] ❌ Local file check failed: '{}' not found.", sanitized_url);
                self.log(&msg);
                return Err(msg.into());
            }

            let final_path = if path_obj.is_dir() {
                self.log(&format!("[CORE] 📂 Input is a directory. Scanning for video files in {:?}", path_obj));
                let mut video_file = None;
                if let Ok(entries) = std::fs::read_dir(path_obj) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                let ext_str = ext.to_string_lossy().to_lowercase();
                                if ["mp4", "mkv", "avi", "mov", "webm"].contains(&ext_str.as_str()) {
                                    video_file = Some(path);
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(found) = video_file {
                    self.log(&format!("[CORE] 🎯 Automatically selected video: {:?}", found.file_name().unwrap()));
                    found
                } else {
                    let msg = format!("[CORE] ❌ No video files found in directory: {:?}", path_obj);
                    self.log(&msg);
                    return Err(msg.into());
                }
            } else {
                path_obj.to_path_buf()
            };

            self.log(&format!("[CORE] 📁 Using local file: {:?}", final_path));
            (
                final_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                final_path,
            )
        } else {
            if !source_tools::check_ytdlp().await {
                let msg = "yt-dlp not found! Please install it via pip.";
                self.log(&format!("[CORE] ❌ {}", msg));
                return Err(msg.into());
            }

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

            match smart_editor::smart_edit(&local_path, intent, &out_path, Some(callback), None, None).await {
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

    pub async fn process_brain_request(&self, request: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        dry_run: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status("🤖 Embodying...");
        self.log(&format!("[CORE] Embodied Agent Activating for: {}", intent));

        use crate::agent::audio_tools;
        use crate::agent::vision_tools;

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

        self.set_status("🧠 Planning & Rendering...");
        let result = {
            let mut cortex = self.cortex.lock().await;
            cortex.execute_smart_render(intent, input, output, &visual_data, &audio_data, dry_run).await
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

        {
            let mut brain = self.brain.lock().await;
            brain.neuroplasticity.record_success();
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn learn_style(&self, input: &Path, name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status(&format!("🎓 Learning '{}'...", name));

        let request = format!("learn style '{}' from '{}'", name, input.display());
        let mut brain = self.brain.lock().await;
        match brain.process(&request).await {
            Ok(msg) => self.log(&format!("[CORE] ✅ {}", msg)),
            Err(e) => {
                self.log(&format!("[CORE] ❌ Learning failed: {}", e));
                return Err(e.into());
            }
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn get_audio_tracks(&self, input: &Path) -> Result<Vec<crate::agent::audio_tools::AudioTrack>, Box<dyn std::error::Error + Send + Sync>> {
        crate::agent::audio_tools::get_audio_tracks(input).await
    }

    // --- Unified Pipeline ---

    pub async fn run_unified_pipeline(
        &self,
        input: &Path,
        output: &Path,
        stages_str: &str,
        _gpu: &str,
        intent: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status("🚀 Running Pipeline...");

        let parsed_stages = PipelineStage::parse_list(stages_str);
        if parsed_stages.is_empty() {
            let msg = "No valid stages specified.";
            self.log(&format!("[CORE] ❌ {}", msg));
            return Err(msg.into());
        }

        let mut pipeline_guard = self.pipeline.lock().await;
        if pipeline_guard.is_none() {
            self.log("[CORE] Initializing GPU Pipeline...");
            *pipeline_guard = Some(UnifiedPipeline::new().await);
        }
        let pipeline = pipeline_guard.as_ref().unwrap();

        let self_clone = self.clone();
        let config = PipelineConfig {
            stages: parsed_stages,
            intent,
            target_size_mb: 0.0,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_input() {
        assert_eq!(AgentCore::sanitize_input("  test  "), "test");
        assert_eq!(AgentCore::sanitize_input("\"C:\\Path\""), "C:\\Path");
        assert_eq!(AgentCore::sanitize_input("'C:\\Path'"), "C:\\Path");

        let input = "\u{202a}C:\\Users\\xing\\Videos\\test.mp4";
        assert_eq!(AgentCore::sanitize_input(input), "C:\\Users\\xing\\Videos\\test.mp4");

        let complex = "  \u{202a}\"C:\\Path With Spaces\\test.mp4\"  ";
        assert_eq!(AgentCore::sanitize_input(complex), "C:\\Path With Spaces\\test.mp4");
    }

    #[test]
    fn test_is_local_robustness() {
        let drive_path = "C:\\Videos\\test.mp4";
        assert!(drive_path.len() > 1 && drive_path.chars().nth(1) == Some(':'));

        let unc_path = "\\\\server\\share\\test.mp4";
        assert!(unc_path.starts_with("\\\\"));

        let url = "https://youtube.com/watch?v=123";
        assert!(!(url.len() > 1 && url.chars().nth(1) == Some(':')));
        assert!(!url.starts_with("\\\\"));
    }
}
