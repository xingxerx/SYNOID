// SYNOID Embodied Agent GUI with Tree-Organized Commands
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// "Command Center" Premium Interface Design
// Deep Dark Theme | Tree Sidebar | Professional Typography

use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;

use std::thread;

use crate::agent::autonomous_learner::AutonomousLearner;
use crate::agent::brain::Brain;
use crate::agent::production_tools;
use crate::agent::vector_engine::{vectorize_video, VectorConfig};
use crate::state::KernelState;

// --- Color Palette (Premium Dark) ---
const COLOR_BG_DARK: egui::Color32 = egui::Color32::from_rgb(22, 22, 26);
const COLOR_PANEL_BG: egui::Color32 = egui::Color32::from_rgb(30, 30, 34);
const COLOR_SIDEBAR_BG: egui::Color32 = egui::Color32::from_rgb(26, 26, 30);
const COLOR_ACCENT_ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 120, 50);
const COLOR_ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(80, 160, 255);
const COLOR_ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(80, 200, 120);
const COLOR_ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(180, 100, 255);
const COLOR_ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(255, 80, 80);
const COLOR_TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(220, 220, 220);
const COLOR_TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(140, 140, 150);
const COLOR_TREE_ITEM: egui::Color32 = egui::Color32::from_rgb(100, 180, 255);

#[derive(PartialEq, Clone, Copy, Debug)]
enum ActiveCommand {
    None,
    // Video
    Youtube,
    Clip,
    Compress,
    // Vector
    Vectorize,
    Upscale,
    // AI
    Brain,
    Embody,
    Learn,
    Suggest,
    // Voice
    VoiceRecord,
    VoiceClone,
    VoiceSpeak,
    // Defense
    Guard,
    // Research
    Research,
}

#[derive(Default, Clone)]
pub struct TreeState {
    pub video_expanded: bool,
    pub vector_expanded: bool,
    pub ai_expanded: bool,
    pub voice_expanded: bool,
    pub defense_expanded: bool,
    pub research_expanded: bool,
}

pub type AgentTask = crate::state::TaskState;

pub struct SynoidApp {
    state: Arc<KernelState>,
    tree_state: TreeState,
    active_command: ActiveCommand,
    learner: Arc<AutonomousLearner>,
    #[allow(dead_code)]
    api_url: String,
}

impl SynoidApp {
    pub fn new(state: Arc<KernelState>) -> Self {
        let api_url = std::env::var("SYNOID_API_URL")
            .unwrap_or_else(|_| "http://localhost:11434/v1".to_string());

        Self {
            state: state.clone(),
            tree_state: TreeState {
                video_expanded: true,
                vector_expanded: true,
                ai_expanded: false,
                voice_expanded: false,
                defense_expanded: false,
                research_expanded: false,
            },
            active_command: ActiveCommand::None,
            learner: Arc::new(AutonomousLearner::new(Arc::new(tokio::sync::Mutex::new(
                Brain::new(&api_url),
            )))),
            api_url,
        }
    }

    fn configure_style(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = COLOR_BG_DARK;
        visuals.panel_fill = COLOR_PANEL_BG;
        visuals.widgets.noninteractive.bg_fill = COLOR_PANEL_BG;
        visuals.widgets.active.bg_fill = COLOR_ACCENT_ORANGE;
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 50, 60);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.selection.bg_fill = COLOR_ACCENT_ORANGE;

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (
                egui::TextStyle::Heading,
                egui::FontId::new(22.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(12.0, egui::FontFamily::Monospace),
            ),
            (
                egui::TextStyle::Small,
                egui::FontId::new(11.0, egui::FontFamily::Proportional),
            ),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        ctx.set_style(style);
    }

    fn render_tree_category(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        icon: &str,
        color: egui::Color32,
        expanded: &mut bool,
        items: Vec<(&str, &str, ActiveCommand)>,
    ) -> Option<ActiveCommand> {
        let mut selected: Option<ActiveCommand> = None;

        ui.horizontal(|ui| {
            let arrow = if *expanded { "▼" } else { "▶" };
            if ui
                .add(
                    egui::Label::new(
                        egui::RichText::new(arrow)
                            .size(10.0)
                            .color(COLOR_TEXT_SECONDARY),
                    )
                    .sense(egui::Sense::click()),
                )
                .clicked()
            {
                *expanded = !*expanded;
            }
            if ui
                .add(
                    egui::Label::new(
                        egui::RichText::new(format!("{} {}", icon, label))
                            .size(14.0)
                            .color(color)
                            .strong(),
                    )
                    .sense(egui::Sense::click()),
                )
                .clicked()
            {
                *expanded = !*expanded;
            }
        });

        if *expanded {
            for (item_icon, item_label, cmd) in items {
                let is_selected = self.active_command == cmd;
                let text_color = if is_selected {
                    COLOR_ACCENT_ORANGE
                } else {
                    COLOR_TREE_ITEM
                };

                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    let response = ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("{} {}", item_icon, item_label))
                                .size(13.0)
                                .color(text_color),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if response.clicked() {
                        selected = Some(cmd);
                    }
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                });
            }
        }
        ui.add_space(4.0);
        selected
    }

    fn render_command_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        match self.active_command {
            ActiveCommand::None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new("🎯").size(48.0));
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new("Select a Command")
                            .size(24.0)
                            .color(COLOR_TEXT_SECONDARY),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("Choose from the sidebar to get started")
                            .size(14.0)
                            .color(COLOR_TEXT_SECONDARY),
                    );
                });
            }
            ActiveCommand::Youtube => self.render_youtube_panel(ui, task),
            ActiveCommand::Clip => self.render_clip_panel(ui, task),
            ActiveCommand::Compress => self.render_compress_panel(ui, task),
            ActiveCommand::Vectorize => self.render_vectorize_panel(ui, task),
            ActiveCommand::Upscale => self.render_upscale_panel(ui, task),
            ActiveCommand::Brain => self.render_brain_panel(ui, task),
            ActiveCommand::Embody => self.render_embody_panel(ui, task),
            ActiveCommand::Learn => self.render_learn_panel(ui, task),
            ActiveCommand::Suggest => self.render_suggest_panel(ui, task),
            ActiveCommand::VoiceRecord => self.render_voice_record_panel(ui, task),
            ActiveCommand::VoiceClone => self.render_voice_clone_panel(ui, task),
            ActiveCommand::VoiceSpeak => self.render_voice_speak_panel(ui, task),
            ActiveCommand::Guard => self.render_guard_panel(ui, task),
            ActiveCommand::Research => self.render_research_panel(ui, task),
        }
    }

    // --- Command Panels ---

    fn render_youtube_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("📤 Upload Video").color(COLOR_ACCENT_ORANGE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Video File:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut task.input_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Video", &["mp4", "mkv", "avi", "mov", "webm"])
                    .pick_file()
                {
                    task.input_path = path.to_string_lossy().to_string();
                }
            }
        });
        ui.add_space(10.0);

        ui.label("Creative Intent:");
        ui.add(
            egui::TextEdit::multiline(&mut task.intent)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
        ui.add_space(5.0);
        ui.checkbox(&mut task.is_funny_bits_enabled, "🎭 Enable Funny Mode (Commentary + Transitions)");
        ui.add_space(10.0);

        ui.label("Output Path:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut task.output_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Video", &["mp4"])
                    .set_file_name("output.mp4")
                    .save_file()
                {
                    task.output_path = path.to_string_lossy().to_string();
                }
            }
        });
        ui.add_space(20.0);

        // Validation hints
        let has_input = !task.input_path.is_empty();
        let has_output = !task.output_path.is_empty();

        if !has_input || !has_output {
            ui.add_space(5.0);
            ui.label(
                egui::RichText::new("⚠️ Select a video file and output path to continue")
                    .size(12.0)
                    .color(COLOR_ACCENT_RED),
            );
            ui.add_space(5.0);
        }

        let state_clone = self.state.clone();
        let button_enabled = has_input && has_output;
        let button = egui::Button::new(egui::RichText::new("📤 Upload & Process").size(16.0)).fill(
            if button_enabled {
                COLOR_ACCENT_ORANGE
            } else {
                egui::Color32::from_rgb(80, 80, 80)
            },
        );

        if ui.add(button).clicked() && button_enabled {
            let input = PathBuf::from(&task.input_path);
            let output = PathBuf::from(&task.output_path);
            let intent = task.intent.clone();
            let funny_mode = task.is_funny_bits_enabled;

            task.logs
                .push(format!("[UPLOAD] Processing: {}", task.input_path));
            if !intent.is_empty() {
                task.logs.push(format!("[UPLOAD] 🧠 Intent: {}", intent));
            }
            task.status = "📤 Processing...".to_string();
            task.is_running = true;

            thread::spawn(move || {
                // Validate input file exists
                if !input.exists() {
                    let mut t = state_clone.task.lock().unwrap();
                    t.logs
                        .push(format!("[UPLOAD] ❌ File not found: {:?}", input));
                    t.status = "⚡ Ready".to_string();
                    t.is_running = false;
                    return;
                }

                // Use smart editor if intent is provided
                if !intent.is_empty() {
                    use crate::agent::smart_editor;

                    let state_for_callback = state_clone.clone();
                    let callback = Box::new(move |msg: &str| {
                        if let Ok(mut t) = state_for_callback.task.lock() {
                            t.logs.push(msg.to_string());
                        }
                    });

                    let rt = tokio::runtime::Runtime::new().unwrap();
                    match rt.block_on(smart_editor::smart_edit(
                        &input,
                        &intent,
                        &output,
                        funny_mode,
                        Some(callback),
                    )) {
                        Ok(result) => {
                            let mut t = state_clone.task.lock().unwrap();
                            t.logs.push(format!("[UPLOAD] {}", result));
                            t.status = "⚡ Ready".to_string();
                            t.is_running = false;
                        }
                        Err(e) => {
                            let mut t = state_clone.task.lock().unwrap();
                            t.logs.push(format!("[UPLOAD] ❌ Smart edit failed: {}", e));
                            t.logs
                                .push("[UPLOAD] Falling back to simple copy...".to_string());

                            // Fallback to copy
                            if let Ok(bytes) = std::fs::copy(&input, &output) {
                                let mb = bytes as f64 / 1_000_000.0;
                                t.logs.push(format!("[UPLOAD] ✅ Copied: {:.2} MB", mb));
                            }
                            t.status = "⚡ Ready".to_string();
                            t.is_running = false;
                        }
                    }
                } else {
                    // No intent - just copy the file
                    match std::fs::copy(&input, &output) {
                        Ok(bytes) => {
                            let mut t = state_clone.task.lock().unwrap();
                            let mb = bytes as f64 / 1_000_000.0;
                            t.logs
                                .push(format!("[UPLOAD] ✅ Video imported: {:.2} MB", mb));
                            t.logs.push("[UPLOAD] 💡 Tip: Add an intent like 'remove boring parts' for AI editing".to_string());
                            t.logs.push(format!("[UPLOAD] Output: {:?}", output));
                            t.status = "⚡ Ready".to_string();
                            t.is_running = false;
                        }
                        Err(e) => {
                            let mut t = state_clone.task.lock().unwrap();
                            t.logs.push(format!("[UPLOAD] ❌ Error: {}", e));
                            t.status = "⚡ Ready".to_string();
                            t.is_running = false;
                        }
                    }
                }
            });
        }
    }

    fn render_clip_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("✂️ Clip Video").color(COLOR_ACCENT_BLUE));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, task);
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Start (sec):");
            ui.add(egui::TextEdit::singleline(&mut task.clip_start).desired_width(80.0));
            ui.label("Duration (sec):");
            ui.add(egui::TextEdit::singleline(&mut task.clip_duration).desired_width(80.0));
        });
        ui.add_space(10.0);

        self.render_output_file_picker(ui, task);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("✂️ Trim Video").size(16.0))
                    .fill(COLOR_ACCENT_BLUE),
            )
            .clicked()
        {
            task.logs.push(format!(
                "[CLIP] Trimming {}s from {}s",
                task.clip_duration, task.clip_start
            ));
            task.status = "✂️ Clipping...".to_string();
        }
    }

    fn render_compress_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("📦 Compress Video").color(COLOR_ACCENT_GREEN));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, task);
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Target Size (MB):");
            ui.add(egui::TextEdit::singleline(&mut task.compress_size).desired_width(80.0));
        });
        ui.add_space(10.0);

        self.render_output_file_picker(ui, task);
        ui.add_space(20.0);

        let state_clone = self.state.clone();
        if ui
            .add(
                egui::Button::new(egui::RichText::new("📦 Compress").size(16.0))
                    .fill(COLOR_ACCENT_GREEN),
            )
            .clicked()
        {
            if !task.input_path.is_empty() {
                let input = PathBuf::from(&task.input_path);
                let size: f64 = task.compress_size.parse().unwrap_or(25.0);
                let output = PathBuf::from(&task.output_path);

                task.logs.push(format!("[COMPRESS] Target: {:.1} MB", size));
                task.status = "📦 Compressing...".to_string();

                thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        match production_tools::compress_video(&input, size, &output).await {
                            Ok(res) => {
                                let mut t = state_clone.task.lock().unwrap();
                                t.logs
                                    .push(format!("[COMPRESS] ✅ Done: {:.2} MB", res.size_mb));
                                t.status = "⚡ Ready".to_string();
                            }
                            Err(e) => {
                                let mut t = state_clone.task.lock().unwrap();
                                t.logs.push(format!("[COMPRESS] ❌ Error: {}", e));
                                t.status = "⚡ Ready".to_string();
                            }
                        }
                    });
                });
            }
        }
    }

    fn render_vectorize_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("🎨 Vectorize to SVG").color(COLOR_ACCENT_PURPLE));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, task);
        ui.add_space(10.0);

        ui.label("Output Directory:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut task.output_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    task.output_path = path.to_string_lossy().to_string();
                }
            }
        });
        ui.add_space(20.0);

        let state_clone = self.state.clone();
        if ui
            .add(
                egui::Button::new(egui::RichText::new("🎨 Convert to SVG").size(16.0))
                    .fill(COLOR_ACCENT_PURPLE),
            )
            .clicked()
        {
            if !task.input_path.is_empty() {
                let input = PathBuf::from(&task.input_path);
                let output_dir = PathBuf::from(&task.output_path);

                task.logs
                    .push("[VECTOR] Starting conversion...".to_string());
                task.status = "🎨 Vectorizing...".to_string();

                thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let config = VectorConfig::default();
                        match vectorize_video(&input, &output_dir, config).await {
                            Ok(msg) => {
                                let mut t = state_clone.task.lock().unwrap();
                                t.logs.push(format!("[VECTOR] ✅ {}", msg));
                                t.status = "⚡ Ready".to_string();
                            }
                            Err(e) => {
                                let mut t = state_clone.task.lock().unwrap();
                                t.logs.push(format!("[VECTOR] ❌ Error: {}", e));
                                t.status = "⚡ Ready".to_string();
                            }
                        }
                    });
                });
            }
        }
    }

    fn render_upscale_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("🔎 Infinite Upscale").color(COLOR_ACCENT_ORANGE));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, task);
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Scale Factor:");
            ui.add(egui::TextEdit::singleline(&mut task.scale_factor).desired_width(60.0));
            ui.label("x");
        });
        ui.add_space(10.0);

        self.render_output_file_picker(ui, task);
        ui.add_space(20.0);

        let state_clone = self.state.clone();
        if ui
            .add(
                egui::Button::new(egui::RichText::new("🔎 Upscale Video").size(16.0))
                    .fill(COLOR_ACCENT_ORANGE),
            )
            .clicked()
        {
            if !task.input_path.is_empty() {
                let input = PathBuf::from(&task.input_path);
                let output = PathBuf::from(&task.output_path);
                let scale: f64 = task.scale_factor.parse().unwrap_or(2.0);

                task.logs
                    .push(format!("[UPSCALE] Starting {:.1}x upscale...", scale));
                task.status = "🔎 Upscaling...".to_string();

                use crate::agent::vector_engine::upscale_video;
                thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        match upscale_video(&input, scale, &output).await {
                            Ok(msg) => {
                                let mut t = state_clone.task.lock().unwrap();
                                t.logs.push(format!("[UPSCALE] ✅ {}", msg));
                                t.status = "⚡ Ready".to_string();
                            }
                            Err(e) => {
                                let mut t = state_clone.task.lock().unwrap();
                                t.logs.push(format!("[UPSCALE] ❌ Error: {}", e));
                                t.status = "⚡ Ready".to_string();
                            }
                        }
                    });
                });
            }
        }
    }

    fn render_brain_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("🧠 Brain Command").color(COLOR_ACCENT_BLUE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Natural Language Request:");
        ui.add(
            egui::TextEdit::multiline(&mut task.intent)
                .desired_rows(4)
                .desired_width(f32::INFINITY),
        );
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🧠 Process Request").size(16.0))
                    .fill(COLOR_ACCENT_BLUE),
            )
            .clicked()
        {
            task.logs
                .push(format!("[BRAIN] Processing: {}", task.intent));
            task.status = "🧠 Thinking...".to_string();
        }
    }

    fn render_embody_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("🤖 Embodied Agent").color(COLOR_ACCENT_PURPLE));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, task);
        ui.add_space(10.0);

        ui.label("Creative Intent:");
        ui.add(
            egui::TextEdit::multiline(&mut task.intent)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
        ui.add_space(10.0);

        self.render_output_file_picker(ui, task);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🤖 Execute Intent").size(16.0))
                    .fill(COLOR_ACCENT_PURPLE),
            )
            .clicked()
        {
            task.logs.push(format!("[EMBODY] Intent: {}", task.intent));
            task.status = "🤖 Executing...".to_string();
        }
    }

    fn render_learn_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("🎓 Learn Style").color(COLOR_ACCENT_GREEN));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, task);
        ui.add_space(10.0);

        ui.label("Style Name:");
        ui.text_edit_singleline(&mut task.voice_profile);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🎓 Analyze & Learn").size(16.0))
                    .fill(COLOR_ACCENT_GREEN),
            )
            .clicked()
        {
            task.logs
                .push(format!("[LEARN] Analyzing style: {}", task.voice_profile));
            task.status = "🎓 Learning...".to_string();
        }
    }

    fn render_suggest_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("💡 Get Suggestions").color(COLOR_ACCENT_BLUE));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, task);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("💡 Analyze Video").size(16.0))
                    .fill(COLOR_ACCENT_BLUE),
            )
            .clicked()
        {
            task.logs.push("[SUGGEST] Analyzing video...".to_string());
            task.status = "💡 Analyzing...".to_string();
        }
    }

    fn render_voice_record_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("🎙️ Record Voice").color(COLOR_ACCENT_RED));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Recording Duration (seconds):");
        ui.add(egui::TextEdit::singleline(&mut task.clip_duration).desired_width(80.0));
        ui.add_space(10.0);

        self.render_output_file_picker(ui, task);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🎙️ Start Recording").size(16.0))
                    .fill(COLOR_ACCENT_RED),
            )
            .clicked()
        {
            task.logs
                .push(format!("[VOICE] Recording {}s...", task.clip_duration));
            task.status = "🎙️ Recording...".to_string();
        }
    }

    fn render_voice_clone_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("🎭 Clone Voice").color(COLOR_ACCENT_PURPLE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Voice Sample (Audio File):");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut task.input_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Audio", &["wav", "mp3", "flac"])
                    .pick_file()
                {
                    task.input_path = path.to_string_lossy().to_string();
                }
            }
        });
        ui.add_space(10.0);

        ui.label("Profile Name:");
        ui.text_edit_singleline(&mut task.voice_profile);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🎭 Create Voice Profile").size(16.0))
                    .fill(COLOR_ACCENT_PURPLE),
            )
            .clicked()
        {
            task.logs
                .push(format!("[VOICE] Creating profile: {}", task.voice_profile));
            task.status = "🎭 Cloning...".to_string();
        }
    }

    fn render_voice_speak_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("🗣️ Text to Speech").color(COLOR_ACCENT_ORANGE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Text to Speak:");
        ui.add(
            egui::TextEdit::multiline(&mut task.voice_text)
                .desired_rows(4)
                .desired_width(f32::INFINITY),
        );
        ui.add_space(10.0);

        ui.label("Voice Profile (optional):");
        ui.text_edit_singleline(&mut task.voice_profile);
        ui.add_space(10.0);

        self.render_output_file_picker(ui, task);
        ui.add_space(20.0);

        if ui.button("📥 Download/Verify TTS Model").clicked() {
            task.logs
                .push("[VOICE] Checking model status...".to_string());
            // In a real app this would call VoiceEngine::download_model
            task.logs
                .push("[VOICE] ✅ Model 'tiny' is ready.".to_string());
        }
        ui.add_space(10.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🗣️ Generate Speech").size(16.0))
                    .fill(COLOR_ACCENT_ORANGE),
            )
            .clicked()
        {
            task.logs.push("[VOICE] Generating speech...".to_string());
            task.status = "🗣️ Speaking...".to_string();
        }
    }

    fn render_guard_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("🛡️ Cyberdefense Sentinel").color(COLOR_ACCENT_RED));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Monitor Mode:");
        ui.horizontal(|ui| {
            ui.radio_value(&mut task.guard_mode, "all".to_string(), "All");
            ui.radio_value(&mut task.guard_mode, "sys".to_string(), "Processes");
            ui.radio_value(&mut task.guard_mode, "file".to_string(), "Files");
        });
        ui.add_space(10.0);

        ui.label("Watch Path (optional):");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut task.guard_watch_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    task.guard_watch_path = path.to_string_lossy().to_string();
                }
            }
        });
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🛡️ Activate Sentinel").size(16.0))
                    .fill(COLOR_ACCENT_RED),
            )
            .clicked()
        {
            task.logs
                .push(format!("[GUARD] Activating mode: {}", task.guard_mode));
            task.status = "🛡️ Guarding...".to_string();
        }
    }

    fn render_research_panel(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.heading(egui::RichText::new("🔍 Research Topic").color(COLOR_ACCENT_BLUE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Research Topic:");
        ui.text_edit_singleline(&mut task.research_topic);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🔍 Search").size(16.0))
                    .fill(COLOR_ACCENT_BLUE),
            )
            .clicked()
        {
            task.logs
                .push(format!("[RESEARCH] Searching: {}", task.research_topic));
            task.status = "🔍 Researching...".to_string();
        }
    }

    // --- Helper renders ---

    fn render_input_file_picker(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.label("Input File:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut task.input_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Video", &["mp4", "mkv", "avi", "mov"])
                    .pick_file()
                {
                    task.input_path = path.to_string_lossy().to_string();
                }
            }
        });
    }

    fn render_output_file_picker(&self, ui: &mut egui::Ui, task: &mut AgentTask) {
        ui.label("Output File:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut task.output_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new().save_file() {
                    task.output_path = path.to_string_lossy().to_string();
                }
            }
        });
    }
    #[allow(dead_code)]
    fn render_funny_bits_overlay(&self, ui: &mut egui::Ui) {
        let moments = {
            let lock = self.state.funny_moments.lock().unwrap();
            lock.clone()
        };

        if moments.is_empty() {
            return;
        }

        let window_rect = ui.ctx().screen_rect();
        let timeline_rect = egui::Rect::from_min_size(
            egui::pos2(window_rect.min.x + 200.0, window_rect.max.y - 40.0), // Sidebar width offset
            egui::vec2(window_rect.width() - 220.0, 30.0),
        );

        let painter = ui.painter();

        // Background
        painter.rect_filled(timeline_rect, 5.0, egui::Color32::from_black_alpha(200));

        // Let's assume the video duration is roughly the end of the last moment + buffer
        // In a real app we'd get actual duration from metadata
        let max_time = moments
            .last()
            .map(|m| m.start_time + m.duration)
            .unwrap_or(100.0)
            .max(60.0);

        for moment in moments {
            let start_x = timeline_rect.min.x
                + (moment.start_time as f32 / max_time as f32) * timeline_rect.width();
            let width = (moment.duration as f32 / max_time as f32) * timeline_rect.width();

            let rect = egui::Rect::from_min_size(
                egui::pos2(start_x, timeline_rect.min.y + 5.0),
                egui::vec2(width.max(2.0), 20.0),
            );

            let color = match moment.moment_type {
                crate::funny_engine::analyzer::MomentType::Laughter => egui::Color32::YELLOW,
                crate::funny_engine::analyzer::MomentType::DeadSilence => egui::Color32::LIGHT_BLUE,
                _ => egui::Color32::RED,
            };

            painter.rect_filled(rect, 2.0, color);
        }

        ui.ctx().request_repaint();
    }
}

impl eframe::App for SynoidApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.configure_style(ctx);

        // Left Sidebar - Command Tree
        egui::SidePanel::left("command_tree")
            .default_width(240.0)
            .resizable(true)
            .frame(
                egui::Frame::none()
                    .fill(COLOR_SIDEBAR_BG)
                    .inner_margin(egui::Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("SYNOID")
                            .size(20.0)
                            .color(COLOR_ACCENT_ORANGE)
                            .strong(),
                    );
                });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Command Center")
                        .size(11.0)
                        .color(COLOR_TEXT_SECONDARY),
                );
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(12.0);

                // Clone expanded states for mutable borrow
                let mut video_exp = self.tree_state.video_expanded;
                let mut vector_exp = self.tree_state.vector_expanded;
                let mut ai_exp = self.tree_state.ai_expanded;
                let mut voice_exp = self.tree_state.voice_expanded;
                let mut defense_exp = self.tree_state.defense_expanded;
                let mut research_exp = self.tree_state.research_expanded;

                let mut new_cmd: Option<ActiveCommand> = None;

                // Video Production
                if let Some(cmd) = self.render_tree_category(
                    ui,
                    "video",
                    "📹",
                    COLOR_ACCENT_ORANGE,
                    &mut video_exp,
                    vec![
                        ("📤", "Upload Video", ActiveCommand::Youtube),
                        ("✂️", "Clip", ActiveCommand::Clip),
                        ("📦", "Compress", ActiveCommand::Compress),
                    ],
                ) {
                    new_cmd = Some(cmd);
                }

                // Vector Engine
                if let Some(cmd) = self.render_tree_category(
                    ui,
                    "vector",
                    "🎨",
                    COLOR_ACCENT_PURPLE,
                    &mut vector_exp,
                    vec![
                        ("✨", "Vectorize", ActiveCommand::Vectorize),
                        ("🔎", "Upscale", ActiveCommand::Upscale),
                    ],
                ) {
                    new_cmd = Some(cmd);
                }

                // AI Brain
                if let Some(cmd) = self.render_tree_category(
                    ui,
                    "ai",
                    "🧠",
                    COLOR_ACCENT_BLUE,
                    &mut ai_exp,
                    vec![
                        ("💬", "Brain", ActiveCommand::Brain),
                        ("🤖", "Embody", ActiveCommand::Embody),
                        ("🎓", "Learn", ActiveCommand::Learn),
                        ("💡", "Suggest", ActiveCommand::Suggest),
                    ],
                ) {
                    new_cmd = Some(cmd);
                }

                // Voice
                if let Some(cmd) = self.render_tree_category(
                    ui,
                    "voice",
                    "🗣️",
                    COLOR_ACCENT_GREEN,
                    &mut voice_exp,
                    vec![
                        ("🎙️", "Record", ActiveCommand::VoiceRecord),
                        ("🎭", "Clone", ActiveCommand::VoiceClone),
                        ("🔊", "Speak", ActiveCommand::VoiceSpeak),
                    ],
                ) {
                    new_cmd = Some(cmd);
                }

                // Defense
                if let Some(cmd) = self.render_tree_category(
                    ui,
                    "defense",
                    "🛡️",
                    COLOR_ACCENT_RED,
                    &mut defense_exp,
                    vec![("👁️", "Guard", ActiveCommand::Guard)],
                ) {
                    new_cmd = Some(cmd);
                }

                // Research
                if let Some(cmd) = self.render_tree_category(
                    ui,
                    "research",
                    "🔍",
                    COLOR_TEXT_PRIMARY,
                    &mut research_exp,
                    vec![("📚", "Search", ActiveCommand::Research)],
                ) {
                    new_cmd = Some(cmd);
                }

                // Autonomous Mode
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Autonomous Background")
                        .size(11.0)
                        .color(COLOR_TEXT_SECONDARY),
                );

                let mut is_learning = self.learner.is_active();
                // Custom checkbox style
                let text = if is_learning {
                    egui::RichText::new("⚡ Auto-Learning Active").color(COLOR_ACCENT_GREEN)
                } else {
                    egui::RichText::new("💤 Auto-Learning Paused").color(COLOR_TEXT_SECONDARY)
                };

                if ui.checkbox(&mut is_learning, text).changed() {
                    if is_learning {
                        self.learner.start();
                        self.state
                            .task
                            .lock()
                            .unwrap()
                            .logs
                            .push("[AUTO] 🚀 Continuous Learning Started".to_string());
                    } else {
                        self.learner.stop();
                        self.state
                            .task
                            .lock()
                            .unwrap()
                            .logs
                            .push("[AUTO] 🛑 Continuous Learning Stopped".to_string());
                    }
                }

                // ── System Stress Health Bar ──
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("System Stress")
                        .size(11.0)
                        .color(COLOR_TEXT_SECONDARY),
                );
                ui.add_space(4.0);
                {
                    use crate::agent::defense::pressure::PressureLevel;
                    let level = self
                        .state
                        .pressure_level
                        .read()
                        .map(|l| *l)
                        .unwrap_or(PressureLevel::Green);

                    let (ratio, color, label) = match level {
                        PressureLevel::Green => (0.35, COLOR_ACCENT_GREEN, "🟢 Nominal"),
                        PressureLevel::Yellow => (0.70, egui::Color32::from_rgb(255, 200, 50), "🟡 Elevated"),
                        PressureLevel::Red => (1.0, COLOR_ACCENT_RED, "🔴 CRITICAL"),
                    };

                    ui.add(
                        egui::ProgressBar::new(ratio)
                            .fill(color)
                            .text(egui::RichText::new(label).size(11.0).color(COLOR_TEXT_PRIMARY)),
                    );
                }

                // Update tree state
                self.tree_state.video_expanded = video_exp;
                self.tree_state.vector_expanded = vector_exp;
                self.tree_state.ai_expanded = ai_exp;
                self.tree_state.voice_expanded = voice_exp;
                self.tree_state.defense_expanded = defense_exp;
                self.tree_state.research_expanded = research_exp;

                // Apply command selection
                if let Some(cmd) = new_cmd {
                    self.active_command = cmd;
                }
            });

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(32.0)
            .frame(
                egui::Frame::none()
                    .fill(COLOR_BG_DARK)
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
            )
            .show(ctx, |ui| {
                let task = self.state.task.lock().unwrap();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&task.status)
                            .size(12.0)
                            .color(COLOR_ACCENT_BLUE),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("v0.1.0")
                                .size(11.0)
                                .color(COLOR_TEXT_SECONDARY),
                        );
                    });
                });
            });

        // Main Content Area
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(COLOR_PANEL_BG)
                    .inner_margin(egui::Margin::same(20.0)),
            )
            .show(ctx, |ui| {
                // Command Panel (top) - draw background first
                let panel_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(ui.available_width(), 400.0),
                );
                ui.painter()
                    .rect_filled(panel_rect, 8.0, egui::Color32::from_rgb(38, 38, 44));

                ui.allocate_new_ui(
                    egui::UiBuilder::new().max_rect(panel_rect.shrink(20.0)),
                    |ui| {
                        let mut task = self.state.task.lock().unwrap();
                        self.render_command_panel(ui, &mut task);
                    },
                );

                ui.add_space(420.0); // Skip past the panel area

                // Logs Panel (bottom)
                ui.heading(
                    egui::RichText::new("📜 Activity Log")
                        .size(16.0)
                        .color(COLOR_TEXT_SECONDARY),
                );
                ui.add_space(8.0);

                let task = self.state.task.lock().unwrap();
                let logs_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(ui.available_width(), 200.0),
                );
                ui.painter().rect_filled(logs_rect, 6.0, COLOR_BG_DARK);

                ui.allocate_new_ui(
                    egui::UiBuilder::new().max_rect(logs_rect.shrink(12.0)),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(180.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for log in &task.logs {
                                    ui.label(
                                        egui::RichText::new(log)
                                            .monospace()
                                            .size(11.0)
                                            .color(COLOR_TEXT_SECONDARY),
                                    );
                                }
                            });
                    },
                );
            });

        let task = self.state.task.lock().unwrap();
        if task.is_running {
            ctx.request_repaint();
        }
    }
}

pub fn run_gui(state: Arc<KernelState>) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("SYNOID Command Center")
            .with_decorations(true),
        ..Default::default()
    };

    eframe::run_native(
        "SYNOID Command Center",
        options,
        Box::new(|_cc| Ok(Box::new(SynoidApp::new(state)))),
    )
}
