// SYNOID Embodied Agent GUI with Tree-Organized Commands
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// "Command Center" Premium Interface Design
// Deep Dark Theme | Tree Sidebar | Professional Typography

use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::agent::core::AgentCore;

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

// --- WSL Helpers ---
fn is_wsl() -> bool {
    std::env::var("WSL_DISTRO_NAME").is_ok() || 
    std::fs::read_to_string("/proc/version").map(|s| s.contains("Microsoft") || s.contains("WSL")).unwrap_or(false)
}

fn get_default_videos_path() -> PathBuf {
    // Prefer the project-local Video directory
    let project_video = PathBuf::from("Video");
    if project_video.exists() {
        return project_video;
    }

    if is_wsl() {
        // Try the project Video dir via absolute WSL path
        let wsl_project_video = PathBuf::from("/mnt/d/SYNOID/Video");
        if wsl_project_video.exists() {
            return wsl_project_video;
        }

        // Fallback: Windows user Videos folder
        if let Ok(wsl_user) = std::env::var("USER") {
            let win_path = PathBuf::from(format!("/mnt/c/Users/{}/Videos", wsl_user));
            if win_path.exists() {
                return win_path;
            }
        }
    }
    
    // Final fallback: current directory
    PathBuf::from(".")
}

fn format_time(seconds: f64) -> String {
    let hrs = (seconds / 3600.0) as u32;
    let mins = ((seconds % 3600.0) / 60.0) as u32;
    let secs = (seconds % 60.0) as u32;
    format!("{:02}:{:02}:{:02}", hrs, mins, secs)
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum ActiveCommand {
    None,
    // Media
    Clip,
    Compress,
    Editor,
    // Visual

    // AI Core
    Brain,
    Embody,
    Learn,
    Suggest,
    // Security
    Guard,
    // Research
    Research,
    // Audio
    AudioMixer,
}

#[derive(Default, Clone)]
pub struct TreeState {
    pub media_expanded: bool,
    pub visual_expanded: bool,
    pub ai_core_expanded: bool,
    pub security_expanded: bool,
    pub research_expanded: bool,
    pub audio_expanded: bool,
}

/// Holds the temporary UI state (form inputs)
#[derive(Default)]
pub struct UiState {
    pub input_path: String,
    pub output_path: String,
    pub intent: String,
    #[allow(dead_code)]
    pub youtube_url: String,

    // Production params
    pub clip_start: String,
    pub clip_duration: String,
    pub compress_size: String,
    pub scale_factor: String,
    pub research_topic: String,
    pub style_name: String,
    pub guard_mode: String,
    pub guard_watch_path: String,
    pub is_autonomous_running: bool,
    // UI specific
    pub detected_tracks: Vec<crate::agent::audio_tools::AudioTrack>,
    pub hive_mind_status: String,
    pub preview_bytes: Option<Vec<u8>>,
    pub preview_image: Option<egui::ColorImage>,
    pub last_previewed_path: String,
    pub suggestions: Vec<String>,
    pub video_player: Option<crate::agent::video_player::VideoPlayer>,
    pub active_editor_tab: String,
    pub video_duration: f64,
    pub video_position: f64,
    pub is_transcribing: bool,
}



pub struct SynoidApp {
    core: Arc<AgentCore>,
    ui_state: Arc<Mutex<UiState>>,
    tree_state: TreeState,
    active_command: ActiveCommand,
    preview_texture: Option<egui::TextureHandle>,
}

impl SynoidApp {
    pub fn new(core: Arc<AgentCore>) -> Self {
        let mut ui_state = UiState::default();
        if let Ok(saved_intent) = std::fs::read_to_string("synoid_intent.txt") {
            ui_state.intent = saved_intent;
        }
        ui_state.output_path = "Video/output.mp4".to_string();
        ui_state.clip_start = "0.0".to_string();
        ui_state.clip_duration = "10.0".to_string();
        ui_state.compress_size = "25.0".to_string();
        ui_state.scale_factor = "2.0".to_string();
        ui_state.active_editor_tab = "Media".to_string();
        ui_state.guard_mode = "all".to_string();

        // Start background poller for Hive Mind status
        let core_clone = core.clone();
        let ui_state_clone = Arc::new(Mutex::new(ui_state));
        let return_state = ui_state_clone.clone();

        tokio::spawn(async move {
            loop {
                let status = core_clone.get_hive_status().await;
                if let Ok(mut state) = ui_state_clone.lock() {
                    state.hive_mind_status = status;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        });

        Self {
            core,
            ui_state: return_state,
            tree_state: TreeState {
                media_expanded: true,
                visual_expanded: true,
                ai_core_expanded: true,
                security_expanded: false,
                research_expanded: false,
                audio_expanded: true,
            },
            active_command: ActiveCommand::Editor,
            preview_texture: None,
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

    fn render_command_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        match self.active_command {
            ActiveCommand::None => self.render_dashboard(ui, state),
            ActiveCommand::Clip => self.render_clip_panel(ui, state),
            ActiveCommand::Compress => self.render_compress_panel(ui, state),

            ActiveCommand::Brain => self.render_brain_panel(ui, state),
            ActiveCommand::Embody => self.render_embody_panel(ui, state),
            ActiveCommand::Learn => self.render_learn_panel(ui, state),
            ActiveCommand::Suggest => self.render_suggest_panel(ui, state),
            ActiveCommand::Guard => self.render_guard_panel(ui, state),
            ActiveCommand::Research => self.render_research_panel(ui, state),
            ActiveCommand::AudioMixer => self.render_audio_mixer_panel(ui, state),
            ActiveCommand::Editor => (), // Editor has its own panel layout handled elsewhere
        }
    }

    fn render_dashboard(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.vertical_centered(|ui| {
             ui.add_space(20.0);
             ui.label(egui::RichText::new("🚀 SYNOID Dashboard").size(24.0).color(COLOR_ACCENT_ORANGE).strong());
             ui.add_space(10.0);
             ui.label(egui::RichText::new("Autonomous Video Kernel v0.1.1").color(COLOR_TEXT_SECONDARY));
             ui.add_space(30.0);
        });

        ui.columns(2, |cols| {
            cols[0].group(|ui| {
                ui.heading("🐝 Hive Status");
                ui.label(egui::RichText::new(&state.hive_mind_status).monospace());
                ui.add_space(10.0);
                if ui.button("🔄 Refresh Nodes").clicked() {
                    let core = self.core.clone();
                    tokio::spawn(async move { let _ = core.initialize_hive_mind().await; });
                }
            });

            cols[1].group(|ui| {
                ui.heading("🎓 Neuroplasticity");
                ui.label("Learning Loop: Active");
                ui.label(format!("Adaptation: {}", if state.is_autonomous_running { "Stable" } else { "Paused" }));
                ui.add_space(10.0);
                ui.toggle_value(&mut state.is_autonomous_running, "Autonomous Mode");
            });
        });

        if !state.suggestions.is_empty() {
            ui.add_space(20.0);
            ui.group(|ui| {
                ui.heading("💡 Creative Sparks");
                for suggestion in &state.suggestions {
                    ui.label(format!("• {}", suggestion));
                }
            });
        }

        // ── Human Control Index ──────────────────────────────────────────────
        ui.add_space(20.0);
        self.render_hci_panel(ui);
    }

    /// HCI (Human Control Index) authorship score panel.
    fn render_hci_panel(&self, ui: &mut egui::Ui) {
        let score = self.core.hci_score();
        let display = self.core.hci_display();
        let hci = &self.core.hci;
        let director = hci.director_decisions.load(std::sync::atomic::Ordering::Relaxed);
        let ai = hci.ai_decisions.load(std::sync::atomic::Ordering::Relaxed);
        let authorship_pct = hci.authorship_percent() as u32;

        // Colour gradient: green (human-dominated) → yellow (balanced) → orange (AI-dominated)
        let bar_color = if score >= 1.5 {
            COLOR_ACCENT_GREEN
        } else if score >= 0.75 {
            egui::Color32::from_rgb(220, 200, 60)
        } else {
            COLOR_ACCENT_ORANGE
        };

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Authorship Score")
                        .size(14.0)
                        .color(COLOR_ACCENT_PURPLE)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("HCI {:.2}", score))
                            .size(14.0)
                            .color(bar_color)
                            .strong(),
                    );
                });
            });

            ui.add_space(4.0);

            // Progress bar: authorship percentage
            let bar_pct = (authorship_pct as f32 / 100.0).clamp(0.0, 1.0);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 14.0),
                egui::Sense::hover(),
            );
            let painter = ui.painter();
            // Background track
            painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(50, 50, 60));
            // Filled portion
            let fill_rect = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width() * bar_pct, rect.height()),
            );
            painter.rect_filled(fill_rect, 4.0, bar_color);

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Human {}%", authorship_pct))
                        .size(12.0)
                        .color(COLOR_ACCENT_GREEN),
                );
                ui.label(
                    egui::RichText::new(format!("   Director: {}  |  AI: {}", director, ai))
                        .size(12.0)
                        .color(COLOR_TEXT_SECONDARY),
                );
            });

            // Interpretation label
            let interp = if score >= 2.0 {
                "Strong human authorship"
            } else if score >= 1.0 {
                "Balanced human-AI collaboration"
            } else if score >= 0.5 {
                "AI-assisted creation"
            } else {
                "Predominantly AI-generated"
            };
            ui.label(egui::RichText::new(interp).size(11.0).color(COLOR_TEXT_SECONDARY));
            ui.label(egui::RichText::new(&display).size(10.0).monospace().color(COLOR_TEXT_SECONDARY));
        });
    }

    fn render_preview_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.heading(egui::RichText::new("📺 Preview").color(COLOR_ACCENT_BLUE));
            ui.add_space(8.0);

            if let Some(texture) = &self.preview_texture {
                let size = texture.size_vec2();
                let max_width = ui.available_width() - 20.0;
                let scale = max_width / size.x;
                ui.image((texture.id(), size * scale));
            } else {
                ui.add_space(50.0);
                ui.label("No Preview Available");
                ui.label(egui::RichText::new("Select a video file to begin").small().color(COLOR_TEXT_SECONDARY));
                ui.add_space(50.0);
            }

            ui.add_space(10.0);
            if !state.input_path.is_empty() {
                ui.label(egui::RichText::new(&state.input_path).small().color(COLOR_TEXT_SECONDARY));
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    if state.video_player.is_some() {
                        if ui.button(egui::RichText::new("⏹ Stop").color(COLOR_ACCENT_RED)).clicked() {
                            state.video_player = None;
                        }
                    } else {
                        if ui.button(egui::RichText::new("▶ Play in Preview").color(COLOR_ACCENT_GREEN)).clicked() {
                            match crate::agent::video_player::VideoPlayer::new(&state.input_path, state.video_position) {
                                Ok(vp) => state.video_player = Some(vp),
                                Err(e) => self.core.log(&format!("[GUI] ❌ Failed to start video player: {}", e)),
                            }
                        }
                    }
                });
            }
        });
    }



    // --- Command Panels ---


    fn render_clip_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("✂️ Clip Video").color(COLOR_ACCENT_BLUE));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, state);
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Start (sec):");
            ui.add(egui::TextEdit::singleline(&mut state.clip_start).desired_width(80.0));
            ui.label("Duration (sec):");
            ui.add(egui::TextEdit::singleline(&mut state.clip_duration).desired_width(80.0));
        });
        ui.add_space(10.0);

        self.render_output_file_picker(ui, state);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("✂️ Trim Video").size(16.0))
                    .fill(COLOR_ACCENT_BLUE),
            )
            .clicked()
        {
            let core = self.core.clone();
            let input = PathBuf::from(&state.input_path);
            let start: f64 = state.clip_start.parse().unwrap_or(0.0);
            let duration: f64 = state.clip_duration.parse().unwrap_or(10.0);
            let output = if !state.output_path.is_empty() {
                Some(PathBuf::from(&state.output_path))
            } else {
                None
            };

            tokio::spawn(async move {
                let _ = core.clip_video(&input, start, duration, output).await;
            });
        }
    }

    fn render_compress_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("📦 Compress Video").color(COLOR_ACCENT_GREEN));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, state);
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Target Size (MB):");
            ui.add(egui::TextEdit::singleline(&mut state.compress_size).desired_width(80.0));
        });
        ui.add_space(10.0);

        self.render_output_file_picker(ui, state);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("📦 Compress").size(16.0))
                    .fill(COLOR_ACCENT_GREEN),
            )
            .clicked()
        {
            let core = self.core.clone();
            let input = PathBuf::from(&state.input_path);
            let size: f64 = state.compress_size.parse().unwrap_or(25.0);
            let output = if !state.output_path.is_empty() {
                Some(PathBuf::from(&state.output_path))
            } else {
                None
            };

            tokio::spawn(async move {
                let _ = core.compress_video(&input, size, output).await;
            });
        }
    }





    fn render_brain_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🧠 Brain Command").color(COLOR_ACCENT_BLUE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Natural Language Request:");
        ui.add(
            egui::TextEdit::multiline(&mut state.intent)
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
            let core = self.core.clone();
            let request = state.intent.clone();

            tokio::spawn(async move {
                let _ = core.process_brain_request(&request).await;
            });
        }
    }

    fn render_embody_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🤖 Embodied Agent").color(COLOR_ACCENT_PURPLE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("YouTube URL / Video File:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut state.input_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Video", &["mp4", "mkv", "avi", "mov", "webm"])
                    .set_directory(get_default_videos_path())
                    .pick_file()
                {
                    state.input_path = path.to_string_lossy().to_string();
                }
            }
        });
        ui.add_space(10.0);

        ui.label("Creative Intent:");
        if ui.add(
            egui::TextEdit::multiline(&mut state.intent)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        ).changed() {
            let _ = std::fs::write("synoid_intent.txt", &state.intent);
        }
        ui.add_space(5.0);

        ui.add_space(10.0);

        self.render_output_file_picker(ui, state);
        ui.add_space(20.0);

        let has_input = !state.input_path.is_empty();
        if !has_input {
            ui.label(egui::RichText::new("⚠️ Enter a URL or file path").size(12.0).color(COLOR_ACCENT_RED));
        }

        ui.horizontal(|ui| {
            let button_enabled = has_input;
            
            // Standard Embodiment (Logic from original embody_intent)
            let embody_btn = egui::Button::new(egui::RichText::new("🤖 Execute Intent").size(16.0)).fill(
                if button_enabled { COLOR_ACCENT_PURPLE } else { egui::Color32::from_rgb(80, 80, 80) }
            );
            if ui.add(embody_btn).clicked() && button_enabled {
                let core = self.core.clone();
                let input = PathBuf::from(&state.input_path);
                let output = PathBuf::from(&state.output_path);
                let intent = state.intent.clone();

                tokio::spawn(async move {
                    let _ = core.embody_intent(&input, &intent, &output, false).await;
                });
            }

            // Optimized Smart Edit (Logic from original process_youtube_intent)
            let smart_btn = egui::Button::new(egui::RichText::new("⚡ Optimized Edit").size(16.0)).fill(
                if button_enabled { COLOR_ACCENT_ORANGE } else { egui::Color32::from_rgb(80, 80, 80) }
            );
            if ui.add(smart_btn).clicked() && button_enabled {
                let core = self.core.clone();
                let input = state.input_path.clone();
                let output = if !state.output_path.is_empty() {
                    Some(PathBuf::from(&state.output_path))
                } else {
                    None
                };
                let intent = state.intent.clone();
                tokio::spawn(async move {
                    let _ = core.process_youtube_intent(&input, &intent, output, None, false, 0).await;
                });
            }
        });
        
        ui.add_space(10.0);
        ui.label(egui::RichText::new("Note: 'Execute Intent' uses full embodied reasoning. 'Optimized Edit' is faster for specific requests.").small().color(COLOR_TEXT_SECONDARY));
    }

    fn render_learn_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🎓 Learn Style").color(COLOR_ACCENT_GREEN));
        ui.separator();
        ui.add_space(10.0);

        if ui.checkbox(&mut state.is_autonomous_running, "🚀 Autonomous Learning Loop (Videos + Code + Wiki)").changed() {
            let core = self.core.clone();
            let is_running = state.is_autonomous_running;
            tokio::spawn(async move {
                if is_running {
                    core.start_autonomous_learning();
                } else {
                    core.stop_autonomous_learning();
                }
            });
        }
        ui.add_space(10.0);

        self.render_input_file_picker(ui, state);
        ui.add_space(10.0);

        ui.label("Style Name:");
        ui.text_edit_singleline(&mut state.style_name);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🎓 Analyze & Learn").size(16.0))
                    .fill(COLOR_ACCENT_GREEN),
            )
            .clicked()
        {
            let core = self.core.clone();
            let input = PathBuf::from(&state.input_path);
            let name = state.style_name.clone();

            tokio::spawn(async move {
                let _ = core.learn_style(&input, &name).await;
            });
        }
    }

    fn render_suggest_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("💡 Get Suggestions").color(COLOR_ACCENT_BLUE));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, state);
        ui.add_space(20.0);

        if ui.add(egui::Button::new(egui::RichText::new("💡 Analyze Video").size(16.0)).fill(COLOR_ACCENT_BLUE)).clicked() {
             let core = self.core.clone();
             let ui_ptr = self.ui_state.clone();
             // Clone input path to move into async block
             let input_path_str = state.input_path.clone();
             
             tokio::spawn(async move {
                 let path = std::path::PathBuf::from(input_path_str);
                 if let Ok(suggs) = core.get_suggestions(&path).await {
                     if let Ok(mut s) = ui_ptr.lock() {
                         s.suggestions = suggs;
                     }
                 }
             });
        }

        if !state.suggestions.is_empty() {
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label("Suggestions for this video:");
                for (i, sugg) in state.suggestions.iter().enumerate() {
                    if ui.button(format!("{}. {}", i+1, sugg)).clicked() {
                         state.intent = sugg.clone();
                    }
                }
            });
        }
    }



    fn render_guard_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🛡️ Cyberdefense Sentinel").color(COLOR_ACCENT_RED));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Monitor Mode:");
        ui.horizontal(|ui| {
            ui.radio_value(&mut state.guard_mode, "all".to_string(), "All");
            ui.radio_value(&mut state.guard_mode, "sys".to_string(), "Processes");
            ui.radio_value(&mut state.guard_mode, "file".to_string(), "Files");
        });
        ui.add_space(10.0);

        ui.label("Watch Path (optional):");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut state.guard_watch_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(get_default_videos_path())
                    .pick_folder() {
                    state.guard_watch_path = path.to_string_lossy().to_string();
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
            let core = self.core.clone();
            let mode = state.guard_mode.clone();
            let watch = if !state.guard_watch_path.is_empty() {
                Some(PathBuf::from(&state.guard_watch_path))
            } else {
                None
            };

            tokio::spawn(async move {
                core.activate_sentinel(&mode, watch).await;
            });
        }
        ui.add_space(5.0);
        ui.label(egui::RichText::new("Note: Requires SYNOID_ENABLE_SENTINEL=true environment variable.").small().color(COLOR_TEXT_SECONDARY));
    }

    fn render_research_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🔍 Research Topic").color(COLOR_ACCENT_BLUE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Research Topic:");
        ui.text_edit_singleline(&mut state.research_topic);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🔍 Search").size(16.0))
                    .fill(COLOR_ACCENT_BLUE),
            )
            .clicked()
        {
            let core = self.core.clone();
            let topic = state.research_topic.clone();

            tokio::spawn(async move {
                let _ = core.process_research(&topic, 5).await;
            });
        }
    }

    fn render_audio_mixer_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🎚️ Audio Mixer").color(COLOR_ACCENT_ORANGE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Select file to scan for adjustable audio tracks:");
        
        // Input File Picker with Scan side-effect
        ui.horizontal(|ui| {
            let res = ui.add(egui::TextEdit::singleline(&mut state.input_path).desired_width(ui.available_width() - 40.0));
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Media", &["mp4", "mkv", "avi", "mov", "wav", "mp3"])
                    .set_directory(get_default_videos_path())
                    .pick_file() {
                    state.input_path = path.to_string_lossy().to_string();
                    
                    // Trigger scan
                    let core = self.core.clone();
                    let ui_state_ptr = self.ui_state.clone();
                    let path_clone = path.clone();
                    tokio::spawn(async move {
                        if let Ok(tracks) = core.get_audio_tracks(&path_clone).await {
                            let mut s = ui_state_ptr.lock().unwrap();
                            s.detected_tracks = tracks;
                        }
                    });
                }
            }
            if res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                 // Trigger scan on enter
                 let core = self.core.clone();
                 let ui_state_ptr = self.ui_state.clone();
                 let path = std::path::PathBuf::from(&state.input_path);
                 tokio::spawn(async move {
                     if let Ok(tracks) = core.get_audio_tracks(&path).await {
                         let mut s = ui_state_ptr.lock().unwrap();
                         s.detected_tracks = tracks;
                     }
                 });
            }
        });

        ui.add_space(15.0);
        ui.label(egui::RichText::new("Adjustable Audio Tracks:").strong());
        
        if state.detected_tracks.is_empty() {
            ui.add_space(5.0);
            ui.label(egui::RichText::new("No tracks detected or file not scanned yet.").color(COLOR_TEXT_SECONDARY).italics());
        } else {
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for track in &state.detected_tracks {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("Track {}", track.index)).strong().color(COLOR_ACCENT_BLUE));
                            ui.label(&track.title);
                            if let Some(lang) = &track.language {
                                ui.label(egui::RichText::new(format!("({})", lang)).small().color(COLOR_TEXT_SECONDARY));
                            }
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("🔈 Solo").clicked() {
                                    // Future: Implement solo logic
                                }
                                if ui.button("🔇 Mute").clicked() {
                                    // Future: Implement mute logic
                                }
                            });
                        });
                        
                        // Heuristic: If title contains "Background", show a different icon or slider?
                        // For now just show "Adjustable" as requested
                        let slider_label = if track.title.to_lowercase().contains("background") {
                            "Background Volume"
                        } else if track.title.to_lowercase().contains("player") || track.title.to_lowercase().contains("mic") {
                            "Player/Voice Volume"
                        } else {
                            "Track Volume"
                        };
                        
                        ui.horizontal(|ui| {
                            ui.label(slider_label);
                            let mut vol = 1.0f32;
                            ui.add(egui::Slider::new(&mut vol, 0.0..=2.0).show_value(true));
                        });
                    });
                    ui.add_space(4.0);
                }
            });
        }

        ui.add_space(20.0);
        if ui.button(egui::RichText::new("🎚️ Apply Mix to File").size(16.0)).clicked() {
            self.core.log("Mixer application pending full audio-stitching implementation.");
        }
    }

    // --- Helper renders ---

    fn render_input_file_picker(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.label("Input File:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut state.input_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Video", &["mp4", "mkv", "avi", "mov"])
                    .set_directory(get_default_videos_path())
                    .pick_file() {
                    state.input_path = path.to_string_lossy().to_string();
                }
            }
        });
    }

    fn render_output_file_picker(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.label("Output File:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut state.output_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(get_default_videos_path())
                    .save_file() {
                    state.output_path = path.to_string_lossy().to_string();
                }
            }
        });
    }
    fn render_editor_layout(&mut self, ctx: &egui::Context, _state: &mut UiState) {
        let color_bg_darkest = egui::Color32::from_rgb(17, 17, 17); // #111111
        let color_panel_bg = egui::Color32::from_rgb(26, 26, 26);   // #1A1A1A
        let color_gold = egui::Color32::from_rgb(217, 178, 77);     // #D9B24D
        let color_text_light = egui::Color32::from_rgb(230, 230, 230);
        let color_text_dim = egui::Color32::from_rgb(120, 120, 120);

        // 1. Top Navbar
        egui::TopBottomPanel::top("editor_toolbar")
            .exact_height(50.0)
            .frame(egui::Frame::none().fill(color_panel_bg).inner_margin(egui::Margin::symmetric(16.0, 10.0)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.add(egui::Button::new(egui::RichText::new("◀  SYNOID").color(color_gold).strong()).fill(egui::Color32::TRANSPARENT)).clicked() {
                        self.active_command = ActiveCommand::None;
                    }
                    
                    ui.add_space(20.0);
                    if ui.add(egui::Button::new("↶").fill(egui::Color32::TRANSPARENT)).clicked() {}
                    if ui.add(egui::Button::new("↷").fill(egui::Color32::TRANSPARENT)).clicked() {}

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center).with_cross_align(egui::Align::Center), |ui| {
                         ui.add_space(ui.available_width() / 2.0 - 100.0); // Rough center
                         ui.label(egui::RichText::new("● My Project / ").color(color_text_dim));
                         let display_name = if _state.input_path.is_empty() { 
                             "New File".to_string() 
                         } else { 
                             std::path::Path::new(&_state.input_path)
                                 .file_name()
                                 .map(|n| n.to_string_lossy().to_string())
                                 .unwrap_or_else(|| "Unknown File".to_string())
                         };
                         ui.label(egui::RichText::new(display_name).color(color_text_light));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Export Button
                        let export_btn = egui::Button::new(egui::RichText::new("  🎬 Export  ").color(egui::Color32::BLACK).strong())
                            .fill(color_gold)
                            .rounding(egui::Rounding::same(16.0));

                        if ui.add(export_btn).clicked() {
                            println!("[GUI] Export clicked! Starting production pipeline...");
                        }
                        
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("👤").size(20.0)); // Profile icon
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("❓").size(20.0)); // Help icon
                    });
                });
            });

        // 2. Left Icon Nav (Slim)
        egui::SidePanel::left("editor_icon_nav")
            .exact_width(70.0)
            .frame(egui::Frame::none().fill(color_bg_darkest).inner_margin(egui::Margin::symmetric(0.0, 16.0)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let nav_items = [
                        ("📁", "Media"),
                        ("▶️", "Video"),
                        ("🖼️", "Photo"),
                        ("🎵", "Audio"),
                        ("T", "Text"),
                        ("💬", "Subtitles"),
                        ("✨", "AI Magic"),
                    ];
                    
                    for (icon, label) in nav_items {
                        let is_active = _state.active_editor_tab == label;
                        let text_color = if is_active { color_gold } else { color_text_dim };
                        let bg_color = if is_active { egui::Color32::from_rgb(30, 26, 17) } else { egui::Color32::TRANSPARENT };
                        
                        let btn_text = if label == "Text" && _state.is_transcribing { format!("⌛\nTranscribing...") } else { format!("{}\n{}", icon, label) };
                        let btn = ui.add_sized(
                            [60.0, 56.0], 
                            egui::Button::new(egui::RichText::new(btn_text).size(11.0).color(text_color))
                            .fill(bg_color)
                            .rounding(egui::Rounding::same(8.0))
                        );
                        
                        // Wiring functional bits based on clicks
                        if btn.clicked() {
                            _state.active_editor_tab = label.to_string();
                            match label {
                                "Text" | "Subtitles" => { 
                                    let input_path = _state.input_path.clone();
                                    if !input_path.is_empty() && !_state.is_transcribing {
                                        _state.is_transcribing = true;
                                        let ui_ptr = self.ui_state.clone();
                                        tokio::spawn(async move {
                                            tracing::info!("[GUI] Triggering transcription for {}", input_path);
                                            if let Ok(engine) = crate::agent::transcription::TranscriptionEngine::new(None).await {
                                                if let Ok(segments) = engine.transcribe(std::path::Path::new(&input_path)).await {
                                                    let srt_content = crate::agent::transcription::generate_srt(&segments);
                                                    let out_srt = std::path::Path::new(&input_path).with_extension("srt");
                                                    let _ = tokio::fs::write(&out_srt, srt_content).await;
                                                    tracing::info!("[GUI] Transcription complete! Saved to {:?}", out_srt);
                                                }
                                            }
                                            if let Ok(mut s) = ui_ptr.lock() {
                                                s.is_transcribing = false;
                                            }
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                        ui.add_space(8.0);
                    }
                });
            });

        // 3. Asset Browser Panel
        egui::SidePanel::left("editor_asset_browser")
            .exact_width(280.0)
            .frame(
                egui::Frame::none()
                    .fill(color_panel_bg)
                    .inner_margin(egui::Margin::same(16.0))
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                     ui.label(egui::RichText::new(&_state.active_editor_tab).color(color_text_light).strong());
                     ui.add_space(20.0);
                     ui.label(egui::RichText::new("Cloud").color(color_text_dim));
                });
                ui.add_space(16.0);
                
                if _state.active_editor_tab == "Media" {
                    // Current Selection
                    if !_state.input_path.is_empty() {
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Active Asset").color(color_gold).small());
                            ui.label(egui::RichText::new(std::path::Path::new(&_state.input_path).file_name().unwrap_or_default().to_string_lossy()).size(12.0).strong());
                            if ui.button("🗑 Remove").clicked() {
                                _state.input_path = String::new();
                            }
                        });
                        ui.add_space(12.0);
                    }

                    // Big Upload Button
                    if ui.add_sized(
                        [ui.available_width(), 48.0],
                        egui::Button::new(egui::RichText::new("☁  Upload Asset").color(color_gold).strong())
                            .fill(egui::Color32::from_rgb(30, 26, 17))
                            .stroke(egui::Stroke::new(1.0, color_gold))
                            .rounding(egui::Rounding::same(8.0))
                    ).clicked() {
                        tracing::info!("[GUI] Upload clicked, opening file dialog...");
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Video", &["mp4", "mkv", "avi", "mov"])
                            .set_directory(get_default_videos_path())
                            .pick_file() {
                            let path_str = path.to_string_lossy().to_string();
                            tracing::info!("[GUI] Selected file: {}", path_str);
                            _state.input_path = path_str;
                        } else {
                            tracing::warn!("[GUI] No file selected or dialog cancelled.");
                        }
                    }
                    
                    ui.add_space(20.0);
                    
                    // Asset Grid Placholder View
                    ui.columns(2, |cols| {
                         for i in 0..6 {
                             let col = if i % 2 == 0 { &mut cols[0] } else { &mut cols[1] };
                             let rect = col.available_rect_before_wrap();
                             let padded = rect.shrink(4.0);
                             
                             let item_rect = col.allocate_exact_size(egui::vec2(padded.width(), 80.0), egui::Sense::hover()).0;
                             col.painter().rect_filled(item_rect, 6.0, egui::Color32::from_rgb(40, 40, 40));
                             
                             col.painter().text(
                                 item_rect.min + egui::vec2(8.0, 60.0),
                                 egui::Align2::LEFT_TOP,
                                 &format!("00:1{}", i),
                                 egui::FontId::proportional(10.0),
                                 egui::Color32::WHITE,
                             );
                             col.add_space(8.0);
                         }
                    });
                } else if _state.active_editor_tab == "AI Magic" {
                    ui.vertical(|ui| {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("✨ Prompt-Based Edits").color(color_gold).strong());
                        ui.label(egui::RichText::new("Describe how you want to edit the active asset.").color(color_text_dim).small());
                        
                        ui.add_space(15.0);
                        ui.label("Your Prompt:");
                        ui.add(egui::TextEdit::multiline(&mut _state.intent).desired_rows(4).desired_width(ui.available_width()));
                        
                        ui.add_space(20.0);
                        let disabled = _state.input_path.is_empty() || _state.intent.trim().is_empty();
                        
                        let btn = egui::Button::new(egui::RichText::new("🪄 Execute AI Magic").strong().color(egui::Color32::BLACK))
                            .fill(if disabled { egui::Color32::from_rgb(100, 100, 100) } else { color_gold })
                            .rounding(egui::Rounding::same(8.0));
                            
                        if ui.add_sized([ui.available_width(), 40.0], btn).clicked() && !disabled {
                            let core = self.core.clone();
                            let input = _state.input_path.clone();
                            let output = if !_state.output_path.is_empty() {
                                Some(PathBuf::from(&_state.output_path))
                            } else {
                                Some(PathBuf::from("Video/magic_edit.mp4"))
                            };
                            let intent = _state.intent.clone();
                            tokio::spawn(async move {
                                tracing::info!("[GUI] Executing AI Magic Edit...");
                                let _ = core.process_youtube_intent(&input, &intent, output, None, false, 0).await;
                            });
                        }
                        
                        if disabled {
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new("⚠️ Please select an asset and enter a prompt.").color(COLOR_ACCENT_RED).small());
                        }
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(egui::RichText::new(format!("{} tools not implemented yet.", _state.active_editor_tab)).color(color_text_dim));
                        ui.label(egui::RichText::new("Placeholder for cloud assets").size(10.0).color(color_text_dim));
                    });
                }
            });

        // 4. Bottom Timeline
        egui::TopBottomPanel::bottom("editor_timeline")
            .exact_height(280.0)
            .frame(egui::Frame::none().fill(color_panel_bg).inner_margin(egui::Margin::same(16.0)))
            .show(ctx, |ui| {
                // Toolbar strip
                ui.horizontal(|ui| {
                    // Left tools
                    if ui.add(egui::Button::new(egui::RichText::new("⎌").size(16.0).color(color_text_dim)).fill(egui::Color32::TRANSPARENT)).clicked() {
                        println!("[GUI] Undo clicked");
                    }
                    if ui.add(egui::Button::new(egui::RichText::new("⎍").size(16.0).color(color_text_dim)).fill(egui::Color32::TRANSPARENT)).clicked() {
                        println!("[GUI] Redo clicked");
                    }
                    if ui.add(egui::Button::new(egui::RichText::new("✂").size(16.0).color(color_text_dim)).fill(egui::Color32::TRANSPARENT)).clicked() {
                        println!("[GUI] Cut clicked");
                    }
                    if ui.add(egui::Button::new(egui::RichText::new("🗑").size(16.0).color(color_text_dim)).fill(egui::Color32::TRANSPARENT)).clicked() {
                        println!("[GUI] Delete clicked");
                    }

                    // Center Playback
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center).with_cross_align(egui::Align::Center), |ui| {
                         ui.add_space(ui.available_width() / 2.0 - 150.0); // Rough center
                         if ui.add(egui::Button::new("⏮").fill(egui::Color32::TRANSPARENT)).clicked() {}
                         
                         let is_playing = _state.video_player.as_ref().map_or(false, |p| p.playing);
                         if ui.add(egui::Button::new(egui::RichText::new(if is_playing { "⏸" } else { "▶" }).size(20.0).color(color_gold)).fill(egui::Color32::TRANSPARENT)).clicked() {
                             if let Some(player) = &mut _state.video_player {
                                 player.stop();
                                 _state.video_player = None;
                             } else if !_state.input_path.is_empty() {
                                 if let Ok(player) = crate::agent::video_player::VideoPlayer::new(&_state.input_path, _state.video_position) {
                                     _state.video_player = Some(player);
                                 }
                             }
                         }
                         if ui.add(egui::Button::new("⏭").fill(egui::Color32::TRANSPARENT)).clicked() {}
                         
                         ui.add_space(16.0);
                         let pos_text = format_time(_state.video_position);
                         let dur_text = format_time(_state.video_duration);
                         ui.label(egui::RichText::new(format!("{} / {}", pos_text, dur_text)).color(color_text_light));
                    });

                    // Right tools
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("🔍 +");
                        let mut zoom = 0.5f32;
                        ui.add(egui::Slider::new(&mut zoom, 0.0..=1.0).show_value(false));
                        ui.label("-");
                    });
                });
                
                ui.add_space(12.0);
                
                // Track Area
                egui::ScrollArea::both().show(ui, |ui| {
                    let start_y = ui.cursor().min.y;
                    
                    // Ruler
                    {
                        let p = ui.painter();
                        let total_width = (_state.video_duration.max(60.0) as f32) * 10.0; // 10px per second
                        let ruler_rect = egui::Rect::from_min_size(egui::pos2(ui.cursor().min.x, start_y), egui::vec2(total_width, 20.0));
                        p.rect_filled(ruler_rect, 0.0, color_panel_bg);
                        
                        let steps = (_state.video_duration / 10.0) as i32 + 1;
                        for i in 0..steps.max(20) {
                            let x = ui.cursor().min.x + (i as f32) * 100.0; // 100px per 10s
                            p.text(egui::pos2(x, start_y + 4.0), egui::Align2::LEFT_TOP, format!("{}s", i * 10), egui::FontId::proportional(10.0), color_text_dim);
                            p.line_segment([egui::pos2(x, start_y + 15.0), egui::pos2(x, start_y + 20.0)], egui::Stroke::new(1.0, color_text_dim));
                        }
                    }
                    
                    ui.add_space(24.0);
                    
                    let tracks = vec![
                        ("Video", egui::Color32::from_rgb(117, 72, 196), 0.0),
                        ("Effects", egui::Color32::from_rgb(220, 90, 150), 40.0),
                        ("Audio", egui::Color32::from_rgb(45, 140, 110), 80.0),
                    ];
                    
                    {
                        let p = ui.painter();
                        for (i, (name, accent_color, y_offset)) in tracks.iter().enumerate() {
                            let track_y = start_y + 30.0 + y_offset;
                            
                            // Left label area
                            let label_rect = egui::Rect::from_min_size(egui::pos2(ui.cursor().min.x, track_y), egui::vec2(60.0, 32.0));
                            p.rect_filled(label_rect, 0.0, color_bg_darkest);
                            p.text(label_rect.center(), egui::Align2::CENTER_CENTER, *name, egui::FontId::proportional(11.0), color_text_dim);
                            
                            // Track background line
                            p.line_segment(
                                [egui::pos2(ui.cursor().min.x + 60.0, track_y + 16.0), egui::pos2(ui.cursor().min.x + 1000.0, track_y + 16.0)],
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 40))
                            );
                            
                            // Clip Segment
                            let clip_rect = egui::Rect::from_min_size(egui::pos2(ui.cursor().min.x + 80.0 + (i as f32 * 20.0), track_y + 2.0), egui::vec2(300.0, 28.0));
                            p.rect_filled(clip_rect, 6.0, *accent_color);
                        }
                        
                        // Playhead
                        let playhead_x = ui.cursor().min.x + 180.0;
                        p.line_segment([egui::pos2(playhead_x, start_y), egui::pos2(playhead_x, start_y + 150.0)], egui::Stroke::new(2.0, color_gold));
                        p.circle_filled(egui::pos2(playhead_x, start_y + 10.0), 6.0, color_gold);
                    }
                    
                    ui.add_space(180.0);
                });
            });

        // 5. Main Preview Window
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(color_bg_darkest).inner_margin(egui::Margin::same(32.0)))
            .show(ctx, |ui| {
                // Floating tools on right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                     ui.vertical(|ui| {
                         ui.add(egui::Button::new("🪄").fill(color_panel_bg).rounding(4.0));
                         ui.add_space(4.0);
                         ui.add(egui::Button::new("🔳").fill(color_panel_bg).rounding(4.0));
                         ui.add_space(4.0);
                         ui.add(egui::Button::new("◓").fill(color_panel_bg).rounding(4.0));
                     });
                     
                     // The Video Frame
                     let video_rect = ui.available_rect_before_wrap();
                     ui.painter().rect_filled(video_rect, 12.0, egui::Color32::from_rgb(0, 0, 0)); // Pure black
                     
                     // Texture render if available
                     if let Some(texture) = &self.preview_texture {
                         ui.painter().image(
                             texture.id(),
                             video_rect,
                             egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                             egui::Color32::WHITE,
                         );
                     } else {
                         // Placeholder Play button
                         let center = video_rect.center();
                         ui.painter().circle_filled(center, 40.0, egui::Color32::from_white_alpha(30));
                         ui.painter().add(egui::Shape::convex_polygon(
                             vec![
                                 center + egui::vec2(-10.0, -15.0),
                                 center + egui::vec2(-10.0, 15.0),
                                 center + egui::vec2(15.0, 0.0),
                             ],
                             color_gold,
                             egui::Stroke::NONE,
                         ));
                     }
                });
            });
    }
}

impl eframe::App for SynoidApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.configure_style(ctx);

        // --- BACKGROUND LOGIC ---
        {
            let mut state = self.ui_state.lock().unwrap();
            
            // 1. Texture conversion
            if let Some(color_image) = state.preview_image.take() {
                self.preview_texture = Some(ctx.load_texture("preview_frame", color_image, Default::default()));
            }

            // 2. Auto-preview and auto-suggest when path changes
            if !state.input_path.is_empty() && state.input_path != state.last_previewed_path {
                state.last_previewed_path = state.input_path.clone();
                let core = self.core.clone();
                let ui_ptr = self.ui_state.clone();
                let path = std::path::PathBuf::from(&state.input_path);
                tracing::info!("[GUI] Auto-previewing changed input path: {:?}", path);
                
                let ctx_clone = ctx.clone();
                tokio::spawn(async move {
                    // 1. Duration & Info
                    if let Ok(duration) = crate::agent::source_tools::get_video_duration(&path).await {
                        if let Ok(mut s) = ui_ptr.lock() {
                            s.video_duration = duration;
                            s.video_position = 0.0;
                            ctx_clone.request_repaint();
                        }
                    }

                    // 2. Preview Frame
                    match core.get_video_frame(&path, 0.0).await {
                        Ok(frame) => {
                            if frame.is_empty() {
                                tracing::warn!("[GUI] get_video_frame returned 0 bytes.");
                            } else {
                                match image::load_from_memory(&frame) {
                                    Ok(img) => {
                                        let size = [img.width() as _, img.height() as _];
                                        let buffer = img.to_rgba8();
                                        let color_img = egui::ColorImage::from_rgba_unmultiplied(size, buffer.as_raw());
                                        
                                        if let Ok(mut s) = ui_ptr.lock() {
                                            s.preview_image = Some(color_img);
                                            ctx_clone.request_repaint();
                                        }
                                    }
                                    Err(e) => tracing::error!("[GUI] Failed to decode preview frame: {}", e),
                                }
                            }
                        }
                        Err(e) => tracing::error!("[GUI] get_video_frame failed: {}", e),
                    }
                });
            }

            // 3. Video player frame update
            if let Some(player) = &mut state.video_player {
                let size = [player.width as _, player.height as _];
                if let Some(frame) = player.get_next_frame() {
                    let color_image = egui::ColorImage::from_rgb(size, frame);
                    self.preview_texture = Some(ctx.load_texture("video_frame", color_image, Default::default()));
                    
                    // Update position roughly based on FPS
                    state.video_position += 1.0 / player.fps;
                    if state.video_position > state.video_duration {
                        state.video_position = state.video_duration;
                    }
                    
                    ctx.request_repaint();
                }
            }
        }


        if self.active_command != ActiveCommand::Editor {
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
                    
                    ui.add_space(8.0);
                    // Hive Mind Status Display
                    let hive_status = {
                        let state = self.ui_state.lock().unwrap();
                        state.hive_mind_status.clone()
                    };
                    
                    if !hive_status.is_empty() {
                         ui.group(|ui| {
                             ui.label(egui::RichText::new("🐝 Hive Mind").color(COLOR_ACCENT_ORANGE).strong());
                             ui.label(egui::RichText::new(hive_status).size(10.0));
                         });
                    }

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);

                    // Clone expanded states for mutable borrow
                    let mut media_exp = self.tree_state.media_expanded;
                    let visual_exp = self.tree_state.visual_expanded;
                    let mut ai_exp = self.tree_state.ai_core_expanded;
                    let mut security_exp = self.tree_state.security_expanded;
                    let mut research_exp = self.tree_state.research_expanded;
                    let mut audio_exp = self.tree_state.audio_expanded;

                    let mut new_cmd: Option<ActiveCommand> = None;

                    // Media
                    if let Some(cmd) = self.render_tree_category(
                        ui,
                        "Media",
                        "📹",
                        COLOR_ACCENT_ORANGE,
                        &mut media_exp,
                        vec![
                            ("✂️", "Clip", ActiveCommand::Clip),
                            ("📦", "Compress", ActiveCommand::Compress),
                            ("🎬", "Editor", ActiveCommand::Editor),
                        ],
                    ) {
                        new_cmd = Some(cmd);
                    }



                    // AI Core
                    if let Some(cmd) = self.render_tree_category(
                        ui,
                        "AI Core",
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


                    // Security
                    if let Some(cmd) = self.render_tree_category(
                        ui,
                        "Security",
                        "🛡️",
                        COLOR_ACCENT_RED,
                        &mut security_exp,
                        vec![("👁️", "Defense", ActiveCommand::Guard)],
                    ) {
                        new_cmd = Some(cmd);
                    }

                    if let Some(cmd) = self.render_tree_category(
                        ui,
                        "Research",
                        "🔍",
                        COLOR_TEXT_PRIMARY,
                        &mut research_exp,
                        vec![("📚", "Research", ActiveCommand::Research)],
                    ) {
                        new_cmd = Some(cmd);
                    }

                    // Audio
                    if let Some(cmd) = self.render_tree_category(
                        ui,
                        "Audio",
                        "🔊",
                        COLOR_ACCENT_ORANGE,
                        &mut audio_exp,
                        vec![("🎚️", "Mixer", ActiveCommand::AudioMixer)],
                    ) {
                        new_cmd = Some(cmd);
                    }

                    // Update tree state
                    self.tree_state.media_expanded = media_exp;
                    self.tree_state.visual_expanded = visual_exp;
                    self.tree_state.ai_core_expanded = ai_exp;
                    self.tree_state.security_expanded = security_exp;
                    self.tree_state.research_expanded = research_exp;
                    self.tree_state.audio_expanded = audio_exp;

                    // Apply command selection
                    if let Some(cmd) = new_cmd {
                        self.active_command = cmd;
                    }
                });
        }

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(32.0)
            .frame(
                egui::Frame::none()
                    .fill(COLOR_BG_DARK)
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
            )
            .show(ctx, |ui| {
                let status = self.core.get_status();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&status)
                            .size(12.0)
                            .color(COLOR_ACCENT_BLUE),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("v0.1.1")
                                .size(11.0)
                                .color(COLOR_TEXT_SECONDARY),
                        );
                    });
                });
            });

        if self.active_command == ActiveCommand::Editor {
            let ui_state_arc = self.ui_state.clone();
            let mut state = ui_state_arc.lock().unwrap();
            self.render_editor_layout(ctx, &mut state);
        } else {
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
                            let mut state = self.ui_state.lock().unwrap();
                            self.render_command_panel(ui, &mut state);
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

                    let logs = self.core.get_logs();
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
                                    for log in &logs {
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

            // Right Sidebar - Preview
            egui::SidePanel::right("preview_panel")
                .default_width(500.0)
                .resizable(true)
                .frame(
                    egui::Frame::none()
                        .fill(COLOR_SIDEBAR_BG)
                        .inner_margin(egui::Margin::same(12.0)),
                )
                .show(ctx, |ui| {
                    let mut state = self.ui_state.lock().unwrap();
                    self.render_preview_panel(ui, &mut state);
                });
        }

        // Always request repaint to show log updates from background threads
        ctx.request_repaint();
    }
}

pub fn run_gui(core: Arc<AgentCore>) -> Result<(), eframe::Error> {
    // WSLg's Wayland compositor silently fails to forward eframe/winit windows
    // to the Windows desktop. Force X11 (via XWayland) which reliably works.
    if is_wsl() {
        // 1. Remove WAYLAND_DISPLAY so winit won't attempt the Wayland backend
        std::env::remove_var("WAYLAND_DISPLAY");
        // 2. Ensure DISPLAY is set for X11 (WSLg default is :0)
        if std::env::var("DISPLAY").is_err() {
            std::env::set_var("DISPLAY", ":0");
        }
        // 3. Explicitly tell winit to use the X11 backend
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
        tracing::info!("[GUI] WSL detected → forced X11 backend (DISPLAY={:?})", std::env::var("DISPLAY").ok());
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("SYNOID Command Center")
            .with_decorations(true),
        renderer: if is_wsl() { eframe::Renderer::Glow } else { eframe::Renderer::Wgpu },
        ..Default::default()
    };

    eframe::run_native(
        "SYNOID Command Center",
        options,
        Box::new(|_cc| Ok(Box::new(SynoidApp::new(core)))),
    )
}
