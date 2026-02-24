// SYNOID Agent Core - The "Ghost"
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This is the central logic kernel that powers both the CLI and GUI.
// It maintains state, manages long-running processes, and routes intent.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex as AsyncMutex;
use tracing::info;

use crate::agent::brain::Brain;
use crate::agent::defense::{IntegrityGuard, Sentinel};
use crate::agent::motor_cortex::MotorCortex;
use crate::agent::production_tools;
use crate::agent::source_tools;
use crate::agent::unified_pipeline::{PipelineConfig, PipelineStage, UnifiedPipeline};


use crate::agent::autonomous_learner::AutonomousLearner;
use crate::agent::editor_queue::{VideoEditorQueue, EditJob, JobStatus};
use crate::gpu_backend;

/// The shared state of the agent
#[derive(Clone)]
pub struct AgentCore {
    pub api_url: String,
    // Observability State (Thread-safe, Sync for GUI)
    pub status: Arc<Mutex<String>>,
    pub logs: Arc<Mutex<Vec<String>>>,
    pub sentinel_active: Arc<AtomicBool>,

    // Sub-systems (Async Mutex for heavy async tasks)
    pub brain: Arc<AsyncMutex<Brain>>,
    pub cortex: Arc<AsyncMutex<MotorCortex>>,



    // Unified Pipeline (Async Mutex)
    pub pipeline: Arc<AsyncMutex<Option<UnifiedPipeline>>>,

    // Autonomous Learner (Sync Mutex)
    pub autonomous_learner: Arc<Mutex<Option<AutonomousLearner>>>,

    // Video Editor Queue (Sync for easy access)
    pub editor_queue: Arc<VideoEditorQueue>,
    
    // Video Editing Agent (The high-level orchestrator)
    pub video_editing_agent: Arc<Mutex<Option<crate::agent::video_editing_agent::VideoEditingAgent>>>,
}

impl AgentCore {
    pub fn new(api_url: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
            status: Arc::new(Mutex::new("⚡ System Ready".to_string())),
            logs: Arc::new(Mutex::new(vec![
                "[SYSTEM] SYNOID Core initialized.".to_string()
            ])),
            sentinel_active: Arc::new(AtomicBool::new(false)),
            brain: Arc::new(AsyncMutex::new(Brain::new(api_url, "llama3:latest"))),
            cortex: Arc::new(AsyncMutex::new(MotorCortex::new(api_url))),

            pipeline: Arc::new(AsyncMutex::new(None)),
            autonomous_learner: Arc::new(Mutex::new(None)), // Lazy init
            editor_queue: Arc::new(VideoEditorQueue::new(Arc::new(AsyncMutex::new(Brain::new(api_url, "llama3:latest"))))),
            video_editing_agent: Arc::new(Mutex::new(None)), // Lazy init
        }
    }

    pub fn ensure_video_editing_agent(&self) {
        let mut vea = self.video_editing_agent.lock().unwrap();
        if vea.is_none() {
            self.log("[CORE] 🤖 Initializing Video Editing Agent...");
            *vea = Some(crate::agent::video_editing_agent::VideoEditingAgent::new(self.brain.clone()));
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

    pub async fn initialize_hive_mind(&self) -> Result<(), String> {
        let mut brain = self.brain.lock().await;
        brain.initialize_hive_mind().await
    }

    pub async fn get_hive_status(&self) -> String {
        let brain = self.brain.lock().await;
        format!(
            "🧠 Reasoning: {}\n⚡ Fast: {}\n📚 Models Loaded: {}", 
            brain.hive_mind.get_reasoning_model(),
            brain.hive_mind.get_fast_model(),
            brain.hive_mind.models.len()
        )
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
            // Cap at 500 logs to prevent memory exhaustion
            if logs.len() > 500 {
                logs.remove(0);
            }
        }
    }

    pub fn get_status(&self) -> String {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn get_logs(&self) -> Vec<String> {
        let logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        // Only return the last 200 logs to keep the GUI responsive
        if logs.len() > 200 {
            logs[logs.len() - 200..].to_vec()
        } else {
            logs.clone()
        }
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
        chunk_minutes: u32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if chunk_minutes > 0 && chunk_minutes < 600 {
             // Just logging for now as chunking logic is complex and requires ffmpeg splitting
             self.log(&format!("[CORE] ℹ️ Note: Long video chunking ({} mins) requested but experimental. Proceeding with full video.", chunk_minutes));
        }

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
                    self.log(&format!("[CORE] 🎯 Automatically selected video: {:?}", found.file_name().unwrap_or_default()));
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
                final_path
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
        let out_path = output.unwrap_or_else(|| PathBuf::from("Video/output.mp4"));
        // Ensure the output directory exists
        if let Some(parent) = out_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.log(&format!("[CORE] ⚠️ Could not create output directory {:?}: {}", parent, e));
            }
        }

        if !intent.is_empty() {
            self.set_status(&format!("🧠 Processing Intent: {}", intent));
            self.log(&format!("[CORE] Applying intent: {}", intent));

            // 1. Query Brain for Learned Pattern
            let mut pattern = None;
            {
                let brain = self.brain.lock().await;
                // Only try to recall if intent looks like a style or has substance
                if intent.len() > 3 {
                    let recalled = brain.learning_kernel.recall_pattern(intent);
                    // Check if recalled pattern is just default (success_rating 3) or learned (rating > 3 or specific tag)
                    if recalled.intent_tag != "general" || recalled.success_rating > 3 {
                        self.log(&format!("[CORE] 🧠 Brain Recalled: Style '{}' (Avg Scene: {:.1}s)",
                             recalled.intent_tag, recalled.avg_scene_duration));
                        pattern = Some(recalled);
                    }
                }
            }

            use uuid::Uuid;
            use std::time::Instant;

            let job = EditJob {
                id: Uuid::new_v4(),
                input: local_path.clone(),
                intent: intent.to_string(),
                output: out_path.clone(),
                funny_mode,
                status: JobStatus::Queued,
                created_at: Instant::now(),
                pre_scanned_scenes: None,
                pre_scanned_transcript: None,
                // NEW: Pass learned pattern to the job/editor
                learned_pattern: pattern,
            };

            let job_id = self.editor_queue.add_job(job).await;
            self.log(&format!("[CORE] 📥 Video edit queued. Job ID: {}", job_id));
            self.set_status("📥 Edit Queued");
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

    pub async fn process_research(&self, topic: &str, limit: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            let stem = input.file_stem().unwrap_or_default().to_string_lossy();
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
            let stem = input.file_stem().unwrap_or_default().to_string_lossy();
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
        _dry_run: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status("🤖 Embodying...");
        self.log(&format!("[CORE] Embodied Agent Activating for: {}", intent));

        // Note: We used to scan visual and audio context here, but that blocks the GUI thread
        // for several minutes on large files. The Smart Editor handles its own scanning inside
        // the asynchronous video editor queue job!

        // 2. Execute — Queue through VideoEditorQueue
        self.set_status("📥 Edit Queued");
        use crate::agent::editor_queue::{EditJob, JobStatus};
        use uuid::Uuid;
        use std::time::Instant;

        // Query Brain for Learned Pattern
        let mut pattern = None;
        {
            let brain = self.brain.lock().await;
            if intent.len() > 3 {
                let recalled = brain.learning_kernel.recall_pattern(intent);
                if recalled.intent_tag != "general" || recalled.success_rating > 3 {
                    self.log(&format!("[CORE] 🧠 Brain Recalled: Style '{}' (Avg Scene: {:.1}s)",
                         recalled.intent_tag, recalled.avg_scene_duration));
                    pattern = Some(recalled);
                }
            }
        }

        let job = EditJob {
            id: Uuid::new_v4(),
            input: input.to_path_buf(),
            intent: intent.to_string(),
            output: output.to_path_buf(),
            funny_mode: false, // Embody intent doesn't have funny_mode param yet
            status: JobStatus::Queued,
            created_at: Instant::now(),
            pre_scanned_scenes: None,
            pre_scanned_transcript: None,
            learned_pattern: pattern,
        };

        let job_id = self.editor_queue.add_job(job).await;
        self.log(&format!("[CORE] 📥 Embodied intent queued. Job ID: {}", job_id));

        // Record success (of queuing at least)
        {
            let mut brain = self.brain.lock().await;
            brain.neuroplasticity.record_success();
        }

        self.set_status("⚡ Ready");
        Ok(())
    }

    pub async fn learn_style(&self, input: &Path, name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status(&format!("🎓 Learning '{}'...", name));

        // Use Brain's learning logic directly
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





    pub async fn get_video_frame(&self, path: &Path, time_secs: f64) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let safe_input = production_tools::safe_arg_path(path);
        
        // Extract 1 frame at the given timestamp as a JPG
        let output = tokio::process::Command::new("ffmpeg")
            .arg("-ss")
            .arg(time_secs.to_string())
            .arg("-i")
            .arg(&safe_input)
            .args(["-frames:v", "1", "-f", "image2pipe", "-vcodec", "mjpeg", "-"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(format!("FFmpeg frame extraction failed: {:?}", String::from_utf8_lossy(&output.stderr)).into());
        }

        Ok(output.stdout)
    }



    pub async fn get_suggestions(&self, input: &Path) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        self.set_status("💡 Generating Suggestions...");
        self.log(&format!("[CORE] Analyzing {:?} for creative suggestions...", input));

        let mut brain = self.brain.lock().await;
        // Bypassing fully deep visual scan for speed, just use local path info
        let prompt = format!(
            "Given this video file name: {:?}. Generate 3 short, punchy creative editing suggestions for it.",
            input.file_name().unwrap_or_default()
        );

        match brain.process(&prompt).await {
            Ok(res) => {
                // Split multi-line response or just return it as a list
                let suggestions = res.lines()
                    .map(|l| l.trim_matches(|c: char| c.is_numeric() || c == '.' || c == ' ' ).to_string())
                    .filter(|s| !s.is_empty())
                    .take(3)
                    .collect();
                Ok(suggestions)
            }
            Err(e) => Err(e.into())
        }
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
        scale: f64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status("🚀 Running Pipeline...");

        // Query Brain for Learned Pattern if intent is present
        let mut pattern = None;
        if let Some(ref intent_str) = intent {
            let brain = self.brain.lock().await;
            if intent_str.len() > 3 {
                let recalled = brain.learning_kernel.recall_pattern(intent_str);
                if recalled.intent_tag != "general" || recalled.success_rating > 3 {
                    self.log(&format!("[CORE] 🧠 Brain Recalled: Style '{}' (Avg Scene: {:.1}s)",
                         recalled.intent_tag, recalled.avg_scene_duration));
                    pattern = Some(recalled);
                }
            }
        }

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
        let pipeline = pipeline_guard.as_ref().expect("[CORE] Pipeline should be initialized at this point");

        // Config
        let self_clone = self.clone();
        let config = PipelineConfig {
            stages: parsed_stages,
            intent,
            scale_factor: scale,
            target_size_mb: 0.0,
            progress_callback: Some(Arc::new(move |msg: &str| {
                self_clone.log(msg);
            })),
            learned_pattern: pattern,
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
    pub fn stop_sentinel(&self) {
        self.sentinel_active.store(false, Ordering::Relaxed);
        self.log("[CORE] 🛡️ Sentinel Deactivation Signal Sent.");
    }

    pub async fn activate_sentinel(&self, mode: &str, watch: Option<PathBuf>) {
        self.set_status(&format!("🛡️ Sentinel Active ({})", mode));
        self.log("[CORE] 🛡️ ACTIVATING SENTINEL Cyberdefense System...");
        self.sentinel_active.store(true, Ordering::Relaxed);

        let mut integrity = IntegrityGuard::new();
        if let Some(path) = watch {
            self.log(&format!("[CORE] Watching Path: {:?}", path));
            integrity.watch_path(path);
            let _ = integrity.build_baseline();
        }

        let mut sentinel = Sentinel::new();
        self.log("[CORE] ✅ Sentinel Online. Monitoring system...");

        while self.sentinel_active.load(Ordering::Relaxed) {
            // Check System Health
            if mode == "all" || mode == "sys" {
                let alerts = sentinel.scan_processes();
                for alert in alerts {
                    self.log(&format!("[SENTINEL] ⚠️ {}", alert));
                }
            }

            // Check File Integrity
            if mode == "all" || mode == "file" {
                let violations = integrity.verify_integrity().await;
                for v in violations {
                    self.log(&format!("[INTEGRITY] ❌ {}", v));
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
        self.log("[CORE] 🛡️ Sentinel Deactivated.");
        self.set_status("⚡ Ready");
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
        assert_eq!(AgentCore::sanitize_input(input), "C:\\Users\\xing\\Videos\\test.mp4");

        // Test combination
        let complex = "  \u{202a}\"C:\\Path With Spaces\\test.mp4\"  ";
        assert_eq!(AgentCore::sanitize_input(complex), "C:\\Path With Spaces\\test.mp4");
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
