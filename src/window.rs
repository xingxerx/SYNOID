// SYNOID™ Embodied Agent GUI with Visual & Audio Analysis
// Copyright (c) 2026 Xing_The_Creator | SYNOID™
//
// "Davinci-esque" Premium Interface Design
// Deep Dark Theme | Tabbed Workflow | Professional Typography

use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::agent::{vision_tools, audio_tools, research_tools, production_tools, vector_engine};
use crate::agent::vector_engine::{VectorConfig, vectorize_video};

// --- Color Palette (Davinci-inspired) ---
const COLOR_BG_DARK: egui::Color32 = egui::Color32::from_rgb(26, 26, 26);
const COLOR_PANEL_BG: egui::Color32 = egui::Color32::from_rgb(34, 34, 34);
const COLOR_ACCENT_ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 120, 50); // Davinci Resolve Orange
const COLOR_ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(50, 150, 255);
const COLOR_TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(220, 220, 220);
const COLOR_TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(160, 160, 160);

#[derive(PartialEq, Clone, Copy)]
enum AppTab {
    Media,
    Edit,
    Color, // Placeholder for future style/grading
    Deliver,
}

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
    active_tab: AppTab,
    api_url: String,
}

impl Default for SynoidApp {
    fn default() -> Self {
        let mut task = AgentTask::default();
        task.status = "System Ready.".to_string();
        task.output_path = "output.mp4".to_string();
        task.clip_start = "0.0".to_string();
        task.clip_duration = "10.0".to_string();
        task.compress_size = "25.0".to_string();
        task.logs.push("[SYSTEM] Core initialized.".to_string());
        
        Self {
            task: Arc::new(Mutex::new(task)),
            active_tab: AppTab::Media,
            api_url: std::env::var("SYNOID_API_URL")
                .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
        }
    }
}

impl SynoidApp {
    fn configure_style(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = COLOR_BG_DARK;
        visuals.panel_fill = COLOR_PANEL_BG;
        visuals.widgets.noninteractive.bg_fill = COLOR_PANEL_BG;
        visuals.widgets.active.bg_fill = COLOR_ACCENT_ORANGE;
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.selection.bg_fill = COLOR_ACCENT_ORANGE;
        
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::new(24.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Body, egui::FontId::new(14.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Button, egui::FontId::new(14.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Monospace, egui::FontId::new(12.0, egui::FontFamily::Monospace)),
        ].into();
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(15.0, 8.0);
        ctx.set_style(style);
    }
}

impl eframe::App for SynoidApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.configure_style(ctx);
        let mut task = self.task.lock().unwrap();

        // 1. Bottom Tab Bar (Davinci Style)
        egui::TopBottomPanel::bottom("bottom_nav").min_height(50.0).show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal_centered(|ui| {
                ui.style_mut().spacing.item_spacing = egui::vec2(20.0, 0.0);


                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(20.0);
                    // Status on left
                    ui.label(egui::RichText::new(&task.status).size(12.0).color(COLOR_ACCENT_BLUE));
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(20.0);
                });
            });

            // Actually centering the tabs needs a specific layout container or just balanced spacers.
            // Let's put them in a central horizontal strip.
            let available_width = ctx.available_rect().width();
            let tab_width = 400.0;
            let offset = (available_width - tab_width) / 2.0;

            egui::Area::new("tab_area".into()) // Fixed central area for tabs
                .fixed_pos(egui::pos2(offset, ctx.available_rect().height() - 40.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let mut tab_btn = |ui: &mut egui::Ui, tab: AppTab, label: &str, icon: &str| {
                            let selected = self.active_tab == tab;
                            let color = if selected { COLOR_ACCENT_ORANGE } else { COLOR_TEXT_SECONDARY };
                            let label_text = egui::RichText::new(format!("{} {}", icon, label)).size(14.0).strong().color(color);
                            if ui.add(egui::Button::new(label_text).frame(false)).clicked() {
                                self.active_tab = tab;
                            }
                        };
                        tab_btn(ui, AppTab::Media, "MEDIA", "📁");
                        ui.label(egui::RichText::new("|").color(egui::Color32::DARK_GRAY));
                        tab_btn(ui, AppTab::Edit, "EDIT", "✂️");
                        ui.label(egui::RichText::new("|").color(egui::Color32::DARK_GRAY));
                        tab_btn(ui, AppTab::Color, "COLOR", "🎨");
                        ui.label(egui::RichText::new("|").color(egui::Color32::DARK_GRAY));
                        tab_btn(ui, AppTab::Deliver, "DELIVER", "🚀");
                    });
                });
        });

        // 2. Main Content Area
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("SYNOID").color(COLOR_ACCENT_ORANGE).size(28.0).strong());
                ui.heading(egui::RichText::new("STUDIO").color(COLOR_TEXT_PRIMARY).size(28.0).weak());
            });
            ui.separator();
            ui.add_space(10.0);

            match self.active_tab {
                AppTab::Media => {
                    ui.columns(2, |columns| {
                        columns[0].heading("Import Media");
                        columns[0].add_space(10.0);
                        columns[0].group(|ui| {
                            ui.set_width(ui.available_width());
                            ui.label("Source File:");
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut task.input_path);
                                if ui.button("📂 Browse").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Video", &["mp4", "mkv", "avi", "mov"])
                                        .pick_file() {
                                        task.input_path = path.to_string_lossy().to_string();
                                        let msg = format!("[MEDIA] Imported: {}", task.input_path);
                                        task.logs.push(msg); // Fix borrow here
                                    }
                                }
                            });
                            ui.add_space(10.0);
                            ui.label("YouTube / URL Import:");
                            ui.text_edit_singleline(&mut task.youtube_inspiration); // Can check if user wants this field for YT processing
                        });

                        columns[1].heading("Media Properties");
                        columns[1].add_space(10.0);
                        columns[1].group(|ui| {
                            ui.set_width(ui.available_width());
                            if !task.input_path.is_empty() {
                                ui.label(egui::RichText::new(format!("File: {}", task.input_path)).strong());
                                ui.add_space(5.0);
                                if task.scene_count > 0 {
                                    ui.label(format!("Scenes Detected: {}", task.scene_count));
                                    ui.label(format!("Audio Duration: {:.1}s", task.audio_duration));
                                } else {
                                    ui.label("No analysis data availble.");
                                    if ui.button("Analyze Now").clicked() {
                                        task.is_running = true; 
                                        // Simplified trigger for analysis
                                        task.status = "Analyzing...".to_string();
                                    }
                                }
                            } else {
                                ui.label(egui::RichText::new("No media selected").italics().weak());
                            }

                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);

                            // Production Tools in Media Tab
                            ui.label(egui::RichText::new("Production Tools").strong());
                            ui.add_space(5.0);

                            // Compress Video
                            ui.horizontal(|ui| {
                                ui.label("Compress Target (MB):");
                                ui.add(egui::TextEdit::singleline(&mut task.compress_size).desired_width(50.0));
                                if ui.button("📦 Compress Video").clicked() {
                                    if !task.input_path.is_empty() {
                                        let input = PathBuf::from(&task.input_path);
                                        let size: f64 = task.compress_size.parse().unwrap_or(25.0);
                                        let output = PathBuf::from(&task.output_path); // Use output_path for compressed output
                                        
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
                            });

                            ui.add_space(10.0);
                            
                            // Vectorize UI
                            ui.label(egui::RichText::new("🎨 Vectorize (SVG)").strong());
                            if ui.button("✨ Convert to SVG").clicked() {
                                if !task.input_path.is_empty() {
                                    let input = PathBuf::from(&task.input_path);
                                    let output_dir = PathBuf::from(&task.output_path).with_extension(""); 
                                    
                                    task.logs.push(format!("[VECTOR] Starting Raster-to-Vector Engine..."));
                                    task.status = "🎨 Vectorizing...".to_string();
                                    
                                    let task_clone = self.task.clone();
                                    
                                    thread::spawn(move || {
                                        let rt = tokio::runtime::Runtime::new().unwrap();
                                        rt.block_on(async {
                                            let config = VectorConfig::default();
                                            match vectorize_video(&input, &output_dir, config).await {
                                                Ok(msg) => {
                                                    let mut t = task_clone.lock().unwrap();
                                                    t.logs.push(format!("[VECTOR] ✅ {}", msg));
                                                    t.status = "Ready.".to_string();
                                                },
                                                Err(e) => {
                                                    let mut t = task_clone.lock().unwrap();
                                                    t.logs.push(format!("[VECTOR] ❌ Error: {}", e));
                                                }
                                            }
                                        });
                                    });
                                }
                            }

                            ui.add_space(10.0);
                            
                            // Upscale UI
                            ui.label(egui::RichText::new("🔎 Infinite Upscale").strong());
                            ui.horizontal(|ui| {
                                ui.label("Scale (x):");
                                ui.add(egui::TextEdit::singleline(&mut task.compress_size).desired_width(40.0)); // Reusing field for now or add new one
                                if ui.button("🚀 Render High-Res").clicked() {
                                    if !task.input_path.is_empty() {
                                        let input = PathBuf::from(&task.input_path);
                                        let output = PathBuf::from(&task.output_path);
                                        let scale: f64 = task.compress_size.parse().unwrap_or(2.0); // Default to 2.0 if parse fails
                                        
                                        task.logs.push(format!("[UPSCALE] Starting {:.1}x Zoom...", scale));
                                        task.status = "🔎 Upscaling...".to_string();
                                        
                                        let task_clone = self.task.clone();
                                        use crate::agent::vector_engine::upscale_video;
                                        
                                        thread::spawn(move || {
                                            let rt = tokio::runtime::Runtime::new().unwrap();
                                            rt.block_on(async {
                                                match upscale_video(&input, scale, &output).await {
                                                    Ok(msg) => {
                                                        let mut t = task_clone.lock().unwrap();
                                                        t.logs.push(format!("[UPSCALE] ✅ {}", msg));
                                                        t.status = "Ready.".to_string();
                                                    },
                                                    Err(e) => {
                                                        let mut t = task_clone.lock().unwrap();
                                                        t.logs.push(format!("[UPSCALE] ❌ Error: {}", e));
                                                    }
                                                }
                                            });
                                        });
                                    }
                                }
                            });
                        });
                    });
                },
                AppTab::Edit => {
                    ui.columns(2, |columns| {
                        columns[0].heading("Creative Intent");
                        columns[0].group(|ui| {
                            ui.set_width(ui.available_width());
                            ui.label("Describe your vision for the AI editor:");
                            ui.add(egui::TextEdit::multiline(&mut task.intent)
                                .desired_rows(10)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY));
                        });

                        columns[1].heading("Timeline / Tools");
                        columns[1].group(|ui| {
                             ui.set_width(ui.available_width());
                             ui.label(egui::RichText::new("Quick Tools").strong());
                             ui.separator();
                             
                             ui.horizontal(|ui| {
                                 ui.label("Trim Start:");
                                 ui.add(egui::TextEdit::singleline(&mut task.clip_start).desired_width(50.0));
                                 ui.label("Duration:");
                                 ui.add(egui::TextEdit::singleline(&mut task.clip_duration).desired_width(50.0));
                             });
                             if ui.button("Apply Trim").clicked() {
                                 // Trigger trim logic (simplified for UI demo)
                                 task.status = "Trimming...".to_string();
                             }

                             ui.add_space(10.0);
                             ui.horizontal(|ui| {
                                 ui.label("Compress Target (MB):");
                                 ui.add(egui::TextEdit::singleline(&mut task.compress_size).desired_width(50.0));
                             });
                             if ui.button("Compress").clicked() {
                                 task.status = "Compressing...".to_string();
                             }
                        });
                    });
                },
                AppTab::Color => {
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("AI Color Grading Module").size(20.0).weak());
                        ui.label("Coming in v2.0 - Neural Color Matching");
                    });
                },
                AppTab::Deliver => {
                    ui.heading("Render Settings");
                    ui.add_space(10.0);
                    
                    ui.group(|ui| {
                        ui.label("Output Path:");
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut task.output_path);
                            if ui.button("Browse...").clicked() {
                                if let Some(path) = rfd::FileDialog::new().save_file() {
                                    task.output_path = path.to_string_lossy().to_string();
                                }
                            }
                        });
                        
                        ui.add_space(20.0);
                        let can_run = !task.is_running && !task.input_path.is_empty();
                        let btn = egui::Button::new(egui::RichText::new("🚀 RENDER & DELIVER").size(20.0).color(egui::Color32::WHITE))
                            .fill(COLOR_ACCENT_ORANGE);
                        
                        if ui.add_enabled(can_run, btn).clicked() {
                            task.is_running = true;
                            task.status = "Rendering...".to_string();
                            task.logs.push("[RENDER] Job started...".to_string());
                            // Trigger full pipeline logic here
                        }
                    });
                    
                    ui.add_space(20.0);
                    ui.heading("Job Logs");
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for log in &task.logs {
                             ui.label(egui::RichText::new(log).monospace().size(12.0));
                        }
                    });
                }
            }
        });
        
        if task.is_running {
            ctx.request_repaint();
        }
    }
}

pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("SYNOID Studio")
            .with_decorations(true),        ..Default::default()
    };
    
    eframe::run_native(
        "SYNOID Studio",
        options,
        Box::new(|_cc| Ok(Box::new(SynoidApp::default()))),
    )
}
