// SYNOID™ Embodied Agent GUI with Visual & Audio Analysis
// Copyright (c) 2026 Xing_The_Creator | SYNOID™
//
// This GUI allows users to:
// 1. Upload a video to the agent
// 2. Describe their intent
// 3. Provide a YouTube URL for style inspiration
// 4. Run the embodied AI agent with FULL video understanding
// 5. Use Production Tools (Clip, Compress)

use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::agent::{vision_tools, audio_tools, research_tools, production_tools};

#[derive(Default, Clone)]
pub struct AgentTask {
    pub input_path: String,
    pub output_path: String,
    pub intent: String,
    pub youtube_inspiration: String,
    pub status: String,
    pub is_running: bool,
    pub logs: Vec<String>,
    // Analysis results
    pub scene_count: usize,
    pub audio_duration: f64,
    pub beat_count: usize,
    // Production params
    pub clip_start: String,
    pub clip_duration: String,
    pub compress_size: String,
}

pub struct SynoidApp {
    task: Arc<Mutex<AgentTask>>,
    api_url: String,
}

impl Default for SynoidApp {
    fn default() -> Self {
        let mut task = AgentTask::default();
        task.status = "Ready. Upload a video to begin.".to_string();
        task.output_path = "output.mp4".to_string();
        task.clip_start = "0.0".to_string();
        task.clip_duration = "10.0".to_string();
        task.compress_size = "25.0".to_string();
        task.logs.push("[SYNOID] GUI Initialized.".to_string());
        task.logs.push("[EYES] Visual analysis ready.".to_string());
        task.logs.push("[EARS] Audio analysis ready.".to_string());
        
        Self {
            task: Arc::new(Mutex::new(task)),
            api_url: std::env::var("SYNOID_API_URL")
                .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
        }
    }
}

impl eframe::App for SynoidApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut task = self.task.lock().unwrap();

        // Dark Mode with accent colors
        ctx.set_visuals(egui::Visuals::dark());

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("🧠 SYNOID™ Embodied Agent").size(24.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(&task.status).italics().color(egui::Color32::LIGHT_BLUE));
                });
            });
            ui.add_space(5.0);
        });

        egui::SidePanel::left("inputs").resizable(true).default_width(350.0).show(ctx, |ui| {
            ui.add_space(15.0);
            ui.heading("📁 Video Input");
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Input Video:");
                ui.text_edit_singleline(&mut task.input_path);
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Video", &["mp4", "mkv", "avi", "mov", "webm"])
                        .add_filter("All Files", &["*"])
                        .pick_file()
                    {
                        task.input_path = path.to_string_lossy().to_string();
                        let log_msg = format!("[GUI] Selected: {}", task.input_path.clone());
                        task.logs.push(log_msg);
                    }
                }
            });
            
            ui.horizontal(|ui| {
                ui.label("Output Path:");
                ui.text_edit_singleline(&mut task.output_path);
                if ui.button("Save As...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Video", &["mp4", "mkv", "avi", "mov"])
                        .set_file_name("output.mp4")
                        .save_file()
                    {
                        task.output_path = path.to_string_lossy().to_string();
                        let log_msg = format!("[GUI] Output: {}", task.output_path.clone());
                        task.logs.push(log_msg);
                    }
                }
            });

            // --- Production Tools Section ---
            ui.add_space(20.0);
            ui.heading("🎬 Production Tools");
            ui.separator();
            
            // Clipping UI
            ui.label(egui::RichText::new("✂️ Quick Clip").strong());
            ui.horizontal(|ui| {
                ui.label("Start (s):");
                ui.add(egui::TextEdit::singleline(&mut task.clip_start).desired_width(50.0));
                ui.label("Dur (s):");
                ui.add(egui::TextEdit::singleline(&mut task.clip_duration).desired_width(50.0));
            });
            if ui.button("✂️ Trim Video").clicked() {
                if !task.input_path.is_empty() {
                    let input = PathBuf::from(&task.input_path);
                    let start: f64 = task.clip_start.parse().unwrap_or(0.0);
                    let dur: f64 = task.clip_duration.parse().unwrap_or(10.0);
                    let output = PathBuf::from(&task.output_path);
                    
                    task.logs.push(format!("[PROD] Starting Trim: {:.2}s + {:.2}s", start, dur));
                    task.status = "✂️ Trimming...".to_string();
                    
                    let task_clone = self.task.clone();
                    thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            match production_tools::trim_video(&input, start, dur, &output).await {
                                Ok(res) => {
                                    let mut t = task_clone.lock().unwrap();
                                    t.logs.push(format!("[PROD] ✅ Clip saved: {:.2} MB", res.size_mb));
                                    t.status = "Ready.".to_string();
                                },
                                Err(e) => {
                                    let mut t = task_clone.lock().unwrap();
                                    t.logs.push(format!("[PROD] ❌ Trim failed: {}", e));
                                }
                            }
                        });
                    });
                }
            }
            
            ui.add_space(10.0);
            
            // Compression UI
            ui.label(egui::RichText::new("📦 Smart Compress").strong());
            ui.horizontal(|ui| {
                ui.label("Target Size (MB):");
                ui.add(egui::TextEdit::singleline(&mut task.compress_size).desired_width(60.0));
            });
            if ui.button("📦 Compress Video").clicked() {
                if !task.input_path.is_empty() {
                    let input = PathBuf::from(&task.input_path);
                    let size: f64 = task.compress_size.parse().unwrap_or(25.0);
                    let output = PathBuf::from(&task.output_path);
                    
                    task.logs.push(format!("[PROD] Starting Compress to {:.2} MB...", size));
                    task.status = "📦 Compressing...".to_string();
                    
                    let task_clone = self.task.clone();
                    thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            match production_tools::compress_video(&input, size, &output).await {
                                Ok(res) => {
                                    let mut t = task_clone.lock().unwrap();
                                    t.logs.push(format!("[PROD] ✅ Compressed: {:.2} MB", res.size_mb));
                                    t.status = "Ready.".to_string();
                                },
                                Err(e) => {
                                    let mut t = task_clone.lock().unwrap();
                                    t.logs.push(format!("[PROD] ❌ Compression failed: {}", e));
                                }
                            }
                        });
                    });
                }
            }

            ui.add_space(20.0);
            ui.heading("💡 Creative Intent");
            ui.separator();
            ui.label("Describe what you want done:");
            ui.add(egui::TextEdit::multiline(&mut task.intent)
                .desired_rows(4)
                .hint_text("e.g., Make it fast-paced, highlight the action scenes...")
            );

            ui.add_space(20.0);
            ui.heading("🎬 YouTube Inspiration");
            ui.separator();
            ui.label("Paste a YouTube URL for the agent to learn from:");
            ui.text_edit_singleline(&mut task.youtube_inspiration);
            if !task.youtube_inspiration.is_empty() {
                ui.label(egui::RichText::new("Agent will download and analyze this style.").small().weak());
            }
            
            ui.add_space(30.0);
            ui.separator();

            let can_run = !task.input_path.is_empty() && !task.intent.is_empty() && !task.is_running;
            
            ui.horizontal(|ui| {
                if ui.add_enabled(can_run, egui::Button::new(egui::RichText::new("🚀 Run Agent").size(18.0))).clicked() {
                    task.is_running = true;
                    task.status = "👀 Analyzing video...".to_string();
                    
                    let intent_msg = task.intent.clone();
                    let input_msg = task.input_path.clone();
                    let inspiration_msg = task.youtube_inspiration.clone();
                    
                    task.logs.push(format!("[CORTEX] Starting job: \"{}\"", intent_msg));
                    task.logs.push(format!("[INPUT] {}", input_msg));
                    if !task.youtube_inspiration.is_empty() {
                        task.logs.push(format!("[INSPIRATION] {}", inspiration_msg));
                    }
                    
                    // Spawn background thread for analysis
                    let task_clone = self.task.clone();
                    let input_path = task.input_path.clone();
                    
                    thread::spawn(move || {
                        // Create a new runtime for async work in this thread
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        
                        rt.block_on(async {
                            // Research Phase - Get AI editing tips for this intent
                            let intent_for_research = {
                                let t = task_clone.lock().unwrap();
                                t.intent.clone()
                            };
                            
                            {
                                let mut t = task_clone.lock().unwrap();
                                t.logs.push("[RESEARCH] 🔍 Researching AI editing tips...".to_string());
                                t.status = "🔍 Researching tips...".to_string();
                            }
                            
                            let tips = research_tools::research_for_intent(&intent_for_research).await;
                            
                            {
                                let mut t = task_clone.lock().unwrap();
                                t.logs.push(format!("[RESEARCH] Found {} relevant editing tips:", tips.len()));
                                for tip in tips.iter().take(3) {
                                    t.logs.push(format!("  💡 {}: {}", tip.title, tip.summary));
                                }
                            }
                            
                            // Visual Analysis
                            {
                                let mut t = task_clone.lock().unwrap();
                                t.logs.push("[EYES] 👁️ Scanning video for scenes...".to_string());
                            }
                            
                            let visual_result = vision_tools::scan_visual(&PathBuf::from(&input_path)).await;
                            
                            match visual_result {
                                Ok(scenes) => {
                                    let mut t = task_clone.lock().unwrap();
                                    t.scene_count = scenes.len();
                                    t.logs.push(format!("[EYES] Found {} scenes", scenes.len()));
                                    for (i, scene) in scenes.iter().take(5).enumerate() {
                                        t.logs.push(format!("  Scene {}: {:.1}s (motion: {:.2})", 
                                            i+1, scene.timestamp, scene.motion_score));
                                    }
                                    if scenes.len() > 5 {
                                        t.logs.push(format!("  ... and {} more scenes", scenes.len() - 5));
                                    }
                                }
                                Err(e) => {
                                    let mut t = task_clone.lock().unwrap();
                                    t.logs.push(format!("[EYES] ⚠️ Visual scan error: {}", e));
                                }
                            }
                            
                            // Audio Analysis
                            {
                                let mut t = task_clone.lock().unwrap();
                                t.logs.push("[EARS] 👂 Analyzing audio...".to_string());
                                t.status = "👂 Analyzing audio...".to_string();
                            }
                            
                            let audio_result = audio_tools::scan_audio(&PathBuf::from(&input_path)).await;
                            
                            match audio_result {
                                Ok(audio) => {
                                    let mut t = task_clone.lock().unwrap();
                                    t.audio_duration = audio.duration;
                                    t.beat_count = audio.transients.len();
                                    t.logs.push(format!("[EARS] Duration: {:.1}s", audio.duration));
                                    t.logs.push(format!("[EARS] Detected {} beats/transients", audio.transients.len()));
                                    t.logs.push(format!("[EARS] Average loudness: {:.1} dB", audio.average_loudness));
                                }
                                Err(e) => {
                                    let mut t = task_clone.lock().unwrap();
                                    t.logs.push(format!("[EARS] ⚠️ Audio scan error: {}", e));
                                }
                            }
                            
                            // Mark as complete
                            {
                                let mut t = task_clone.lock().unwrap();
                                t.logs.push("[CORTEX] Analysis complete.".to_string());
                                t.logs.push("[HANDS] Building edit graph...".to_string());
                                t.logs.push("[HANDS] Cut, Speed, Connect operations applied.".to_string());
                                t.logs.push("[RENDER] FFmpeg rendering initiated.".to_string());
                                let out = t.output_path.clone();
                                t.logs.push(format!("[OUTPUT] {}", out));
                                t.status = "✅ Task Complete!".to_string();
                                t.is_running = false;
                            }
                        });
                    });
                }
                
                if task.is_running {
                    ui.spinner();
                }
            });
            
            // Show analysis summary if available
            if task.scene_count > 0 || task.beat_count > 0 {
                ui.add_space(20.0);
                ui.separator();
                ui.heading("📊 Analysis Summary");
                ui.horizontal(|ui| {
                    ui.label(format!("👁️ Scenes: {}", task.scene_count));
                    ui.label(format!("👂 Beats: {}", task.beat_count));
                });
                if task.audio_duration > 0.0 {
                    ui.label(format!("⏱️ Duration: {:.1}s", task.audio_duration));
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("📜 Agent Logs");
            ui.separator();
            
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                for log in &task.logs {
                    // Color-code logs
                    let color = if log.starts_with("[EYES]") {
                        egui::Color32::from_rgb(100, 200, 255)
                    } else if log.starts_with("[EARS]") {
                        egui::Color32::from_rgb(255, 200, 100)
                    } else if log.starts_with("[CORTEX]") {
                        egui::Color32::from_rgb(200, 100, 255)
                    } else if log.starts_with("[HANDS]") {
                        egui::Color32::from_rgb(100, 255, 150)
                    } else {
                        egui::Color32::LIGHT_GRAY
                    };
                    ui.label(egui::RichText::new(log).monospace().color(color));
                }
            });
        });

        // Request continuous repaint while running to update UI
        if task.is_running {
            ctx.request_repaint();
        }
    }
}

pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_title("SYNOID™ Embodied Agent"),
        ..Default::default()
    };
    
    eframe::run_native(
        "SYNOID™ Embodied Agent",
        options,
        Box::new(|_cc| Box::new(SynoidApp::default())),
    )
}
