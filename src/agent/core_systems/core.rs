// SYNOID Agent Core - The "Ghost"
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// This is the central logic kernel that powers both the CLI and GUI.
// It maintains state, manages long-running processes, and routes intent.

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use tracing::info;

use crate::agent::engines::process_utils::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt as WinCommandExt;

use crate::agent::core_systems::brain::Brain;
use crate::agent::security::defense::{IntegrityGuard, Sentinel};
use crate::agent::specialized::global_discovery::GlobalDiscovery;
use crate::agent::engines::motor_cortex::MotorCortex;
use crate::agent::tools::production_tools;
use crate::agent::tools::source_tools;
use crate::agent::engines::unified_pipeline::{PipelineConfig, PipelineStage, UnifiedPipeline};

use crate::agent::core_systems::autonomous_learner::AutonomousLearner;
use crate::agent::engines::editor_queue::{EditJob, JobStatus, VideoEditorQueue};
use crate::gpu_backend;

const AUTONOMOUS_PID_FILE: &str = "autonomous_worker.pid";
const AUTONOMOUS_LOG_FILE: &str = "autonomous_worker.log";



fn autonomous_runtime_dir(instance_id: &str) -> PathBuf {
    PathBuf::from(format!("cortex_cache{}", instance_id))
}

fn autonomous_pid_path(instance_id: &str) -> PathBuf {
    autonomous_runtime_dir(instance_id).join(AUTONOMOUS_PID_FILE)
}

fn autonomous_log_path(instance_id: &str) -> PathBuf {
    autonomous_runtime_dir(instance_id).join(AUTONOMOUS_LOG_FILE)
}

fn ensure_autonomous_runtime_dir(instance_id: &str) -> std::io::Result<PathBuf> {
    let dir = autonomous_runtime_dir(instance_id);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn parse_pid(contents: &str) -> Option<u32> {
    contents.trim().parse().ok()
}

fn read_autonomous_pid(instance_id: &str) -> Option<u32> {
    parse_pid(&fs::read_to_string(autonomous_pid_path(instance_id)).ok()?)
}

fn write_autonomous_pid(instance_id: &str, pid: u32) -> std::io::Result<()> {
    ensure_autonomous_runtime_dir(instance_id)?;
    fs::write(autonomous_pid_path(instance_id), pid.to_string())
}

fn clear_autonomous_pid(instance_id: &str) {
    let pid_path = autonomous_pid_path(instance_id);
    if let Err(err) = fs::remove_file(pid_path) {
        if err.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!("[CORE] Failed to clear autonomous worker PID file: {}", err);
        }
    }
}

fn is_pid_running(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }

    #[cfg(windows)]
    {
        let filter = format!("PID eq {}", pid);
        return Command::new("tasklist")
            .stealth()
            .args(["/FI", &filter, "/FO", "CSV", "/NH"])
            .output()
            .map(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout).contains(&format!("\"{}\"", pid))
            })
            .unwrap_or(false);
    }

    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

fn active_autonomous_pid(instance_id: &str) -> Option<u32> {
    let pid = read_autonomous_pid(instance_id)?;
    if is_pid_running(pid) {
        Some(pid)
    } else {
        clear_autonomous_pid(instance_id);
        None
    }
}

fn spawn_autonomous_worker(api_url: &str, instance_id: &str) -> Result<(u32, PathBuf), String> {
    ensure_autonomous_runtime_dir(instance_id).map_err(|e| e.to_string())?;

    let log_path = autonomous_log_path(instance_id);
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| e.to_string())?;
    let stderr = stdout.try_clone().map_err(|e| e.to_string())?;
    let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;

    let mut command = Command::new(exe_path);
    command.stealth();
    command
        .arg("autonomous")
        .env("SYNOID_API_URL", api_url)
        .env("SYNOID_INSTANCE_ID", instance_id)
        .current_dir(current_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    #[cfg(windows)]
    command.creation_flags(0x08000000 | 0x00000008 | 0x00000200); // CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP

    let child = command.spawn().map_err(|e| e.to_string())?;
    let pid = child.id();
    write_autonomous_pid(instance_id, pid).map_err(|e| e.to_string())?;

    Ok((pid, log_path))
}

fn stop_autonomous_worker(instance_id: &str) -> Result<Option<u32>, String> {
    let Some(pid) = read_autonomous_pid(instance_id) else {
        return Ok(None);
    };

    if !is_pid_running(pid) {
        clear_autonomous_pid(instance_id);
        return Ok(None);
    }

    #[cfg(windows)]
    let status = Command::new("taskkill")
        .stealth()
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status()
        .map_err(|e| e.to_string())?;

    #[cfg(not(windows))]
    let status = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() || !is_pid_running(pid) {
        clear_autonomous_pid(instance_id);
        Ok(Some(pid))
    } else {
        Err(format!("failed to stop autonomous worker PID {}", pid))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Human Control Index (Feature 7)
// HCI = Director's Decision Power / AI Autonomy
// Tracks how many editing decisions came from the human vs. the AI so users
// can see their "Authorship Score" in the Command Center.
// ─────────────────────────────────────────────────────────────────────────────

/// Atomic counters for HCI tracking.
#[derive(Debug, Default)]
pub struct HciTracker {
    /// Incremented every time the user manually directs an edit.
    pub director_decisions: AtomicU64,
    /// Incremented every time the AI autonomously applies an operation.
    pub ai_decisions: AtomicU64,
}

impl HciTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that the *human director* made a decision.
    pub fn record_director(&self) {
        self.director_decisions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record that the *AI* made an autonomous decision.
    pub fn record_ai(&self) {
        self.ai_decisions.fetch_add(1, Ordering::Relaxed);
    }

    /// Compute HCI = director_decisions / (ai_decisions + 1).
    ///
    /// A value > 1.0 means humans are driving more decisions than the AI.
    /// A value of 1.0 means equal partnership.
    /// A value < 1.0 means the AI is doing most of the work.
    pub fn score(&self) -> f64 {
        let d = self.director_decisions.load(Ordering::Relaxed) as f64;
        let a = self.ai_decisions.load(Ordering::Relaxed) as f64;
        d / (a + 1.0)
    }

    /// Human-readable authorship percentage (director share of total decisions).
    pub fn authorship_percent(&self) -> f64 {
        let d = self.director_decisions.load(Ordering::Relaxed) as f64;
        let a = self.ai_decisions.load(Ordering::Relaxed) as f64;
        let total = d + a;
        if total == 0.0 {
            100.0
        } else {
            (d / total * 100.0).round()
        }
    }

    /// Formatted status line for the GUI.
    pub fn display(&self) -> String {
        format!(
            "HCI {:.2}  |  Authorship {}%  |  Human {} / AI {}",
            self.score(),
            self.authorship_percent() as u64,
            self.director_decisions.load(Ordering::Relaxed),
            self.ai_decisions.load(Ordering::Relaxed),
        )
    }
}

/// The shared state of the agent
#[derive(Clone)]
pub struct AgentCore {
    pub api_url: String,
    // Observability State (Thread-safe, Sync for GUI)
    pub status: Arc<Mutex<String>>,
    pub logs: Arc<Mutex<Vec<String>>>,
    pub sentinel_active: Arc<AtomicBool>,
    pub instance_id: String,

    // Sub-systems (Async Mutex for heavy async tasks)
    pub brain: Arc<AsyncMutex<Brain>>,
    pub cortex: Arc<AsyncMutex<MotorCortex>>,
    pub discovery: Arc<GlobalDiscovery>,

    // Unified Pipeline (Async Mutex)
    pub pipeline: Arc<AsyncMutex<Option<UnifiedPipeline>>>,

    // Autonomous Learner (Sync Mutex)
    pub autonomous_learner: Arc<Mutex<Option<AutonomousLearner>>>,

    // Video Editor Queue (Sync for easy access)
    pub editor_queue: Arc<VideoEditorQueue>,

    // Video Editing Agent (The high-level orchestrator)
    pub video_editing_agent:
        Arc<Mutex<Option<crate::agent::video_editing_agent::VideoEditingAgent>>>,

    // Remotion Engine Animator
    pub animator: Arc<crate::agent::animator::Animator>,

    // Human Control Index tracker
    pub hci: Arc<HciTracker>,

    // AutoImprove loop state
    pub improve_running: Arc<AtomicBool>,
    pub improve_shutdown: Arc<Mutex<Option<tokio::sync::watch::Sender<bool>>>>,
}

impl AgentCore {
    pub fn new(api_url: &str, instance_id: &str) -> Self {
        let animator = Arc::new(crate::agent::animator::Animator::new(Path::new(".")));
        // Build brain first so both the core and the queue share the same instance.
        let brain = Arc::new(AsyncMutex::new(Brain::new(
            api_url,
            "llama3:latest",
            Some(animator.clone()),
        )));
        // Build logs buffer first so the queue can push progress into the GUI log.
        let logs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![
            "[SYSTEM] SYNOID Core initialized.".to_string()
        ]));
        let logs_for_queue = logs.clone();
        let log_fn: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(move |msg: &str| {
            if let Ok(mut l) = logs_for_queue.lock() {
                l.push(msg.to_string());
                if l.len() > 500 {
                    l.remove(0);
                }
            }
        });
        let editor_queue = Arc::new(VideoEditorQueue::new_with_log(
            brain.clone(),
            instance_id,
            animator.clone(),
            log_fn,
        ));

        let mut cortex_inst = MotorCortex::new(api_url);
        cortex_inst.animator = Some(animator.clone());
        let cortex = Arc::new(AsyncMutex::new(cortex_inst));

        Self {
            api_url: api_url.to_string(),
            instance_id: instance_id.to_string(),
            status: Arc::new(Mutex::new("⚡ System Ready".to_string())),
            logs,
            sentinel_active: Arc::new(AtomicBool::new(false)),
            brain,
            cortex,
            discovery: Arc::new(GlobalDiscovery::new()),
            pipeline: Arc::new(AsyncMutex::new(None)),
            autonomous_learner: Arc::new(Mutex::new(None)), // Lazy init
            editor_queue,
            video_editing_agent: Arc::new(Mutex::new(None)), // Lazy init
            animator,
            hci: Arc::new(HciTracker::new()),
            improve_running: Arc::new(AtomicBool::new(false)),
            improve_shutdown: Arc::new(Mutex::new(None)),
        }
    }

    pub fn ensure_video_editing_agent(&self) {
        let mut vea = self.video_editing_agent.lock().unwrap();
        if vea.is_none() {
            self.log("[CORE] 🤖 Initializing Video Editing Agent...");
            *vea = Some(crate::agent::video_editing_agent::VideoEditingAgent::new(
                self.brain.clone(),
                &self.instance_id,
                self.animator.clone(),
            ));
        }
    }

    /// Learn editing style from videos already present in the Download directory.
    ///
    /// Analyses up to 10 MP4s, stores patterns in the LearningKernel, awards
    /// quality-weighted XP to Neuroplasticity, and writes a tuned EditingStrategy
    /// to cortex_cache so every subsequent edit inherits the learned style.
    pub async fn learn_from_downloads(&self) {
        use crate::agent::video_style_learner;

        self.log("[CORE] 🎓 Learning editing style from downloaded reference videos...");
        self.set_status("🎓 Learning from videos...");

        let (result, report) = {
            let mut brain = self.brain.lock().await;
            let r = video_style_learner::learn_from_downloads(&mut brain).await;
            let rep = brain.neuroplasticity.acceleration_report();
            (r, rep)
        };

        if result.profiles.is_empty() {
            self.log("[CORE] ⚠️ No reference videos found in Download dir — skipping style sync.");
        } else if result.has_new {
            // New videos were learned — update the EditingStrategy on disk
            video_style_learner::synthesise_and_save_strategy(&result.profiles);
            self.log(&format!(
                "[CORE] ✅ Learned {} new video style(s) | {}",
                result.profiles.len(),
                report
            ));
        } else {
            // Everything was already cached — patterns loaded into kernel, no disk write needed
            self.log(&format!(
                "[CORE] ⚡ All {} video style(s) already memorized — instant restore | {}",
                result.profiles.len(),
                report
            ));
        }

        self.set_status("⚡ Ready");
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
            "🧠 Reasoning: {}\n⚡ Fast: {}\n👁️ Vision: {}\n📚 Models Loaded: {}",
            brain.hive_mind.get_reasoning_model(),
            brain.hive_mind.get_fast_model(),
            brain.hive_mind.get_vision_model(),
            brain.hive_mind.models.len()
        )
    }

    /// Get combined acceleration status from Brain + GPU + Neuroplasticity.
    pub async fn acceleration_status(&self) -> String {
        let brain = self.brain.lock().await;
        brain.acceleration_status()
    }

    // --- Global Discovery Methods ---

    pub async fn run_system_scan(&self) {
        self.log("[CORE] 🔎 Initiating system-wide media scan...");
        self.set_status("🔎 Scanning System...");
        let count = self.discovery.scan().await;
        self.log(&format!(
            "[CORE] ✅ System scan complete. {} files indexed.",
            count
        ));
        self.set_status("⚡ Ready");
    }

    pub async fn discover_files(
        &self,
        query: &str,
    ) -> Vec<crate::agent::global_discovery::DiscoveredFile> {
        self.log(&format!("[CORE] 🔎 Searching system for: '{}'", query));
        self.discovery.find(query).await
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
        let s = input.trim().to_string();

        // Remove hidden control characters (e.g., \u{202a} Left-to-Right Embedding)
        // This is common when copying paths from Windows Explorer property dialogs.
        let filtered: String = s
            .chars()
            .filter(|c| !c.is_control() && *c != '\u{202a}' && *c != '\u{202b}' && *c != '\u{202c}')
            .collect();

        let mut result = filtered;

        // Remove surrounding quotes if they exist
        if (result.starts_with('"') && result.ends_with('"'))
            || (result.starts_with('\'') && result.ends_with('\''))
        {
            result.remove(0);
            result.pop();
        }
        result
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

        // Human issued this command explicitly
        self.record_director_decision();
        self.set_status("📥 Downloading...");
        let sanitized_url = Self::sanitize_input(url);
        self.log(&format!("[CORE] Processing YouTube: {}", sanitized_url));

        let output_dir_buf = crate::agent::video_style_learner::get_download_dir();
        let output_dir = output_dir_buf.as_path();
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
                        found.file_name().unwrap_or_default()
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
        let out_path = output.unwrap_or_else(|| PathBuf::from("Video/output.mp4"));
        // Ensure the output directory exists
        if let Some(parent) = out_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.log(&format!(
                    "[CORE] ⚠️ Could not create output directory {:?}: {}",
                    parent, e
                ));
            }
        }

        if !intent.is_empty() {
            self.set_status(&format!("🧠 Processing Intent: {}", intent));
            self.log(&format!("[CORE] Applying intent: {}", intent));

            let lower_intent = intent.to_lowercase();
            if lower_intent.contains("viral clip") {
                self.log("[CORE] 🎓 Special Intent Detected: 'Viral Clip Generation'. Scanning Academy...");
                let academy_dir = Path::new("D:\\SYNOID\\Academy");
                if academy_dir.exists() && academy_dir.is_dir() {
                    let mut found_benchmark = false;
                    if let Ok(entries) = std::fs::read_dir(academy_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(ext) = path.extension() {
                                    let ext_str = ext.to_string_lossy().to_lowercase();
                                    if ["mp4", "mkv", "avi", "mov", "webm"]
                                        .contains(&ext_str.as_str())
                                    {
                                        self.log(&format!("[CORE] 🧠 Prioritizing learning from Academy benchmark: {:?}", path.file_name().unwrap_or_default()));
                                        // Await the learning to feed the brain before we recall the pattern below!
                                        let _ =
                                            self.learn_style(&path, "Viral Clip Generation").await;
                                        found_benchmark = true;
                                    }
                                }
                            }
                        }
                    }
                    if !found_benchmark {
                        self.log(
                            "[CORE] ⚠️ Academy directory is empty. Skipping benchmark ingestion.",
                        );
                    }
                } else {
                    self.log(
                        "[CORE] ⚠️ Academy directory not found. Skipping benchmark ingestion.",
                    );
                }
            }

            // 1. Query Brain for Learned Pattern
            let mut pattern = None;
            {
                let brain = self.brain.lock().await;
                // Only try to recall if intent looks like a style or has substance
                if intent.len() > 3 {
                    let recalled = brain.learning_kernel.lock().await.recall_pattern(intent);
                    // Check if recalled pattern is just default (success_rating 3) or learned (rating > 3 or specific tag)
                    if recalled.intent_tag != "general" || recalled.success_rating > 3 {
                        self.log(&format!(
                            "[CORE] 🧠 Brain Recalled: Style '{}' (Avg Scene: {:.1}s)",
                            recalled.intent_tag, recalled.avg_scene_duration
                        ));
                        pattern = Some(recalled);
                    }
                }
            }

            use std::time::Instant;
            use uuid::Uuid;

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

    /// Run the full AutoResearch pipeline (inspired by AutoResearchClaw).
    /// Queries arXiv, Semantic Scholar, and OpenAlex, then synthesises gaps and hypotheses.
    pub async fn process_auto_research(
        &self,
        topic: &str,
        limit: usize,
        save_json: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::agent::specialized::auto_research::{AutoResearchPipeline, print_research_result};

        self.set_status(&format!("🔬 AutoResearch: {}", topic));
        self.log(&format!("[CORE] Starting AutoResearch pipeline for: {}", topic));

        let pipeline = AutoResearchPipeline::new();
        let result = pipeline.run(topic, limit).await;

        print_research_result(&result);

        if save_json {
            let filename = format!(
                "autoresearch_{}.json",
                topic.to_lowercase().replace(' ', "_").chars().take(40).collect::<String>()
            );
            match serde_json::to_string_pretty(&result) {
                Ok(json) => {
                    if let Err(e) = std::fs::write(&filename, &json) {
                        self.log(&format!("[CORE] ⚠️ Could not save JSON: {}", e));
                    } else {
                        self.log(&format!("[CORE] 💾 Results saved to {}", filename));
                        println!("💾 Saved to {}", filename);
                    }
                }
                Err(e) => self.log(&format!("[CORE] ⚠️ JSON serialisation failed: {}", e)),
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

    pub async fn process_brain_request(
        &self,
        request: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_status("🧠 Thinking...");
        self.log(&format!("[CORE] Brain Request: {}", request));

        let mut brain = self.brain.lock().await;
        match brain.process(request).await {
            Ok(res) => {
                if res.starts_with("DISCOVERY_MODE:") {
                    let query = res.replace("DISCOVERY_MODE:", "");
                    self.run_system_scan().await;
                    let matches = self.discover_files(&query).await;
                    if matches.is_empty() {
                        self.log(&format!("[CORE] 🔍 No files found for '{}'", query));
                    } else {
                        self.log(&format!(
                            "[CORE] 🔍 Found {} matches for '{}'",
                            matches.len(),
                            query
                        ));
                        for m in matches.iter().take(5) {
                            self.log(&format!("   - {:?}", m.path));
                        }
                    }
                } else {
                    self.log(&format!("[CORE] ✅ {}", res));
                }
            }
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
        // Human chose the intent; AI will handle execution — record both sides.
        self.record_director_decision();
        self.set_status("🤖 Embodying...");
        self.log(&format!("[CORE] Embodied Agent Activating for: {}", intent));

        // Note: We used to scan visual and audio context here, but that blocks the GUI thread
        // for several minutes on large files. The Smart Editor handles its own scanning inside
        // the asynchronous video editor queue job!

        // 2. Execute — Queue through VideoEditorQueue
        self.set_status("📥 Edit Queued");
        use crate::agent::engines::editor_queue::{EditJob, JobStatus};
        use std::time::Instant;
        use uuid::Uuid;

        // Query Brain for Learned Pattern
        let mut pattern = None;
        {
            let brain = self.brain.lock().await;
            if intent.len() > 3 {
                let recalled = brain.learning_kernel.lock().await.recall_pattern(intent);
                if recalled.intent_tag != "general" || recalled.success_rating > 3 {
                    self.log(&format!(
                        "[CORE] 🧠 Brain Recalled: Style '{}' (Avg Scene: {:.1}s)",
                        recalled.intent_tag, recalled.avg_scene_duration
                    ));
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
        self.log(&format!(
            "[CORE] 📥 Embodied intent queued. Job ID: {}",
            job_id
        ));

        // The AI will autonomously process the queued job
        self.record_ai_decision();

        // Record success (of queuing at least)
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

    pub async fn record_user_rating(&self, job_id: uuid::Uuid, stars: u8) {
        let quality = match stars {
            1 => 0.1,
            2 => 0.3,
            3 => 0.6,
            4 => 0.8,
            5 => 1.0,
            _ => 0.5,
        };

        self.log(&format!(
            "[CORE] ⭐ User rated Job {} as {} stars (Quality: {:.1})",
            job_id, stars, quality
        ));

        // Find the job to get its intent and input path
        let jobs = self.editor_queue.list_jobs_detailed().await;
        if let Some(job) = jobs.iter().find(|j| j.id == job_id) {
            // Re-trigger learner with the official user-vetted quality
            let learner = crate::agent::autonomous_learner::AutonomousLearner::new(
                self.brain.clone(),
                &self.instance_id,
            );
            // We use a simplified learn_from_edit or a new one?
            // Let's use the one we updated, but since we don't have duration here easily
            // we use values from the job if available.
            if let crate::agent::editor_queue::JobStatus::Completed {
                duration_secs,
                kept_ratio,
                ..
            } = job.status
            {
                learner
                    .learn_from_edit(&job.intent, &job.input, duration_secs, kept_ratio)
                    .await;

                // Override the outcome_xp in the specific pattern
                let mut brain = self.brain.lock().await;
                let mut pattern = brain.learning_kernel.lock().await.recall_pattern(&job.intent);
                pattern.success_rating = stars as u32;
                pattern.outcome_xp = quality;
                brain.learning_kernel.lock().await.memorize(&job.intent, pattern);

                // Additional XP reward for user satisfaction
                if stars >= 4 {
                    brain.neuroplasticity.record_success_with_quality(quality);
                }
            }
        }
    }

    pub async fn list_jobs(&self) -> Vec<crate::agent::editor_queue::EditJob> {
        self.editor_queue.list_jobs_detailed().await
    }

    pub async fn get_video_frame(
        &self,
        path: &Path,
        time_secs: f64,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let safe_input = production_tools::safe_arg_path(path);

        // Extract 1 frame at the given timestamp as a JPG
        let output = tokio::process::Command::new("ffmpeg")
            .stealth()
            .arg("-ss")
            .arg(time_secs.to_string())
            .arg("-i")
            .arg(&safe_input)
            .args([
                "-frames:v",
                "1",
                "-f",
                "image2pipe",
                "-vcodec",
                "mjpeg",
                "-",
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(format!(
                "FFmpeg frame extraction failed: {:?}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        Ok(output.stdout)
    }

    pub async fn get_suggestions(
        &self,
        input: &Path,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        self.set_status("💡 Generating Suggestions...");
        self.log(&format!(
            "[CORE] Analyzing {:?} for creative suggestions...",
            input
        ));

        let mut brain = self.brain.lock().await;
        // Bypassing fully deep visual scan for speed, just use local path info
        let prompt = format!(
            "Given this video file name: {:?}. Generate 3 short, punchy creative editing suggestions for it.",
            input.file_name().unwrap_or_default()
        );

        match brain.process(&prompt).await {
            Ok(res) => {
                // Split multi-line response or just return it as a list
                let suggestions = res
                    .lines()
                    .map(|l| {
                        l.trim_matches(|c: char| c.is_numeric() || c == '.' || c == ' ')
                            .to_string()
                    })
                    .filter(|s| !s.is_empty())
                    .take(3)
                    .collect();
                Ok(suggestions)
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn get_audio_tracks(
        &self,
        input: &Path,
    ) -> Result<Vec<crate::agent::audio_tools::AudioTrack>, Box<dyn std::error::Error + Send + Sync>>
    {
        crate::agent::audio_tools::get_audio_tracks(input).await
    }

    // ── Human Control Index ──────────────────────────────────────────────────

    /// Call this whenever the human director explicitly issues a command
    /// (e.g. pressing a button in the GUI, typing an intent in the Brain
    /// panel, or invoking a CLI subcommand).
    pub fn record_director_decision(&self) {
        self.hci.record_director();
    }

    /// Call this whenever the AI autonomously applies an operation without
    /// explicit user instruction (e.g. auto-cut in SmartEditor, silent
    /// background jobs, autonomous learner decisions).
    pub fn record_ai_decision(&self) {
        self.hci.record_ai();
    }

    /// Return the current HCI display string for the GUI status bar.
    pub fn hci_display(&self) -> String {
        self.hci.display()
    }

    /// Return the raw HCI score (Director Power / AI Autonomy).
    pub fn hci_score(&self) -> f64 {
        self.hci.score()
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
                let recalled = brain.learning_kernel.lock().await.recall_pattern(intent_str);
                if recalled.intent_tag != "general" || recalled.success_rating > 3 {
                    self.log(&format!(
                        "[CORE] 🧠 Brain Recalled: Style '{}' (Avg Scene: {:.1}s)",
                        recalled.intent_tag, recalled.avg_scene_duration
                    ));
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
        let pipeline = pipeline_guard
            .as_ref()
            .expect("[CORE] Pipeline should be initialized at this point");

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
            animator: Some(self.animator.clone()),
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

    // --- AutoImprove ---

    /// Start the self-recursing strategy improvement loop in a background task.
    pub fn start_auto_improve(
        &self,
        benchmark: PathBuf,
        candidates: usize,
        iterations: Option<u64>,
    ) {
        if self.improve_running.load(Ordering::Relaxed) {
            self.log("[IMPROVE] Loop already running.");
            return;
        }

        use crate::agent::specialized::auto_improve::AutoImprove;

        let (tx, rx) = tokio::sync::watch::channel(false);
        if let Ok(mut slot) = self.improve_shutdown.lock() {
            *slot = Some(tx);
        }

        self.improve_running.store(true, Ordering::Relaxed);
        self.log(&format!(
            "[IMPROVE] 🚀 Starting AutoImprove loop | benchmark: {:?} | {} candidates/iter",
            benchmark, candidates
        ));

        let running_flag = self.improve_running.clone();
        let log_fn = {
            let logs = self.logs.clone();
            move |msg: &str| {
                if let Ok(mut l) = logs.lock() {
                    l.push(msg.to_string());
                    if l.len() > 500 {
                        l.remove(0);
                    }
                }
            }
        };

        tokio::spawn(async move {
            let mut improver = AutoImprove::new(benchmark);
            improver.candidates_per_iter = candidates;
            improver.max_iterations = iterations;

            match improver.run(rx).await {
                Ok(()) => log_fn("[IMPROVE] ✅ Loop finished."),
                Err(e) => log_fn(&format!("[IMPROVE] ❌ Error: {}", e)),
            }
            running_flag.store(false, Ordering::Relaxed);
        });
    }

    /// Signal the running AutoImprove loop to stop.
    pub fn stop_auto_improve(&self) {
        if let Ok(mut slot) = self.improve_shutdown.lock() {
            if let Some(tx) = slot.take() {
                let _ = tx.send(true);
                self.log("[IMPROVE] 🛑 Stop signal sent.");
            }
        }
        // Flag will be cleared by the background task when it exits.
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
        self.log("[CORE] Starting autonomous learner with background priority...");

        if let Some(pid) = active_autonomous_pid(&self.instance_id) {
            self.log(&format!(
                "[CORE] Autonomous worker already running in background (PID {}).",
                pid
            ));
            self.set_status("🎓 Autonomous loop running in background");
            return;
        }

        match spawn_autonomous_worker(&self.api_url, &self.instance_id) {
            Ok((pid, log_path)) => {
                self.log(&format!(
                    "[CORE] Autonomous worker launched in background (PID {}).",
                    pid
                ));
                self.log(&format!(
                    "[CORE] Background download log: {}",
                    log_path.display()
                ));
                self.set_status("🎓 Autonomous loop running in background");
            }
            Err(err) => {
                self.log(&format!(
                    "[CORE] Background worker failed to start: {}",
                    err
                ));
                self.log("[CORE] Falling back to in-process learner.");

                let mut learner_guard = self.autonomous_learner.lock().unwrap();
                if learner_guard.is_none() {
                    let learner = AutonomousLearner::new(self.brain.clone(), &self.instance_id);
                    *learner_guard = Some(learner);
                }

                if let Some(learner) = learner_guard.as_ref() {
                    learner.start();
                }
            }
        }
    }

    pub fn stop_autonomous_learning(&self) {
        self.set_status("🛑 Stopping Autonomous Loop...");

        let mut stopped_any = false;

        match stop_autonomous_worker(&self.instance_id) {
            Ok(Some(pid)) => {
                self.log(&format!(
                    "[CORE] Autonomous background worker stopped (PID {}).",
                    pid
                ));
                stopped_any = true;
            }
            Ok(None) => {}
            Err(err) => self.log(&format!(
                "[CORE] Failed to stop autonomous background worker: {}",
                err
            )),
        }

        let learner_guard = self.autonomous_learner.lock().unwrap();
        if let Some(learner) = learner_guard.as_ref() {
            if learner.is_active() {
                learner.stop();
                self.log("[CORE] In-process autonomous loop signal sent: STOP");
                stopped_any = true;
            }
        }

        if !stopped_any {
            self.log("[CORE] No autonomous learner was running.");
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
        let input = "\u{202a}C:\\Users\\xingxerx\\Videos\\test.mp4";
        assert_eq!(
            AgentCore::sanitize_input(input),
            "C:\\Users\\xingxerx\\Videos\\test.mp4"
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

    #[test]
    fn autonomous_runtime_files_are_instance_scoped() {
        assert_eq!(
            autonomous_pid_path("_3012"),
            PathBuf::from("cortex_cache_3012").join(AUTONOMOUS_PID_FILE)
        );
        assert_eq!(
            autonomous_log_path("default"),
            PathBuf::from("cortex_cachedefault").join(AUTONOMOUS_LOG_FILE)
        );
    }

    #[test]
    fn parse_pid_accepts_trimmed_numbers_only() {
        assert_eq!(parse_pid(" 1234\n"), Some(1234));
        assert_eq!(parse_pid("not-a-pid"), None);
        assert_eq!(parse_pid(""), None);
    }
}
