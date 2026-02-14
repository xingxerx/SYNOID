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
    if is_wsl() {
        // Try to find the Windows user name from the WSL environment
        if let Ok(wsl_user) = std::env::var("USER") {
            let win_path = PathBuf::from(format!("/mnt/c/Users/{}/Videos", wsl_user));
            if win_path.exists() {
                return win_path;
            }
        }
        // Fallback to C:\Users\ (common if user name matches)
        let fallback = PathBuf::from("/mnt/c/Users/xing/Videos");
        if fallback.exists() {
            return fallback;
        }
    }
    
    // Default to current directory if not WSL or path not found
    PathBuf::from(".")
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum ActiveCommand {
    None,
    // Media
    Clip,
    Compress,
    // Visual
    Vectorize,
    Upscale,
    // AI Core
    Brain,
    Embody,
    Learn,
    Suggest,
    // Voice (Unified)
    Voice,
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
    pub voice_expanded: bool,
    pub security_expanded: bool,
    pub research_expanded: bool,
    pub audio_expanded: bool,
}

/// Holds the temporary UI state (form inputs)
#[derive(Default, Clone)]
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
    pub voice_text: String,
    pub voice_profile: String,
    pub guard_mode: String,
    pub guard_watch_path: String,
    pub is_funny_bits_enabled: bool,
    pub is_autonomous_running: bool,
    // UI specific
    pub voice_tab: VoiceTab,
    pub detected_tracks: Vec<crate::agent::audio_tools::AudioTrack>,
}

#[derive(Clone, PartialEq, Default)]
pub enum VoiceTab {
    #[default]
    Record,
    Clone,
    Speak,
}

pub struct SynoidApp {
    core: Arc<AgentCore>,
    ui_state: Arc<Mutex<UiState>>,
    tree_state: TreeState,
    active_command: ActiveCommand,
}

impl SynoidApp {
    pub fn new(core: Arc<AgentCore>) -> Self {
        let mut ui_state = UiState::default();
        ui_state.output_path = "output.mp4".to_string();
        ui_state.clip_start = "0.0".to_string();
        ui_state.clip_duration = "10.0".to_string();
        ui_state.compress_size = "25.0".to_string();
        ui_state.scale_factor = "2.0".to_string();
        ui_state.guard_mode = "all".to_string();

        Self {
            core,
            ui_state: Arc::new(Mutex::new(ui_state)),
            tree_state: TreeState {
                media_expanded: true,
                visual_expanded: true,
                ai_core_expanded: true,
                voice_expanded: false,
                security_expanded: false,
                research_expanded: false,
                audio_expanded: true,
            },
            active_command: ActiveCommand::None,
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
            ActiveCommand::Clip => self.render_clip_panel(ui, state),
            ActiveCommand::Compress => self.render_compress_panel(ui, state),
            ActiveCommand::Vectorize => self.render_vectorize_panel(ui, state),
            ActiveCommand::Upscale => self.render_upscale_panel(ui, state),
            ActiveCommand::Brain => self.render_brain_panel(ui, state),
            ActiveCommand::Embody => self.render_embody_panel(ui, state),
            ActiveCommand::Learn => self.render_learn_panel(ui, state),
            ActiveCommand::Suggest => self.render_suggest_panel(ui, state),
            ActiveCommand::Voice => self.render_voice_unified_panel(ui, state),
            ActiveCommand::Guard => self.render_guard_panel(ui, state),
            ActiveCommand::Research => self.render_research_panel(ui, state),
            ActiveCommand::AudioMixer => self.render_audio_mixer_panel(ui, state),
        }
    }

    fn render_voice_unified_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🗣️ Voice Studio").color(COLOR_ACCENT_GREEN));
        ui.separator();
        ui.add_space(10.0);

        ui.horizontal(|ui| {
             ui.selectable_value(&mut state.voice_tab, VoiceTab::Record, "🎙️ Record");
             ui.selectable_value(&mut state.voice_tab, VoiceTab::Clone, "🎭 Clone");
             ui.selectable_value(&mut state.voice_tab, VoiceTab::Speak, "🔊 Speak");
        });
        ui.add_space(15.0);

        match state.voice_tab {
            VoiceTab::Record => self.render_voice_record_panel(ui, state),
            VoiceTab::Clone => self.render_voice_clone_panel(ui, state),
            VoiceTab::Speak => self.render_voice_speak_panel(ui, state),
        }
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

        if ui.add(egui::Button::new(egui::RichText::new("✂️ Trim Video").size(16.0)).fill(COLOR_ACCENT_BLUE)).clicked() {
            let core = self.core.clone();
            let input = PathBuf::from(&state.input_path);
            let start: f64 = state.clip_start.parse().unwrap_or(0.0);
            let duration: f64 = state.clip_duration.parse().unwrap_or(10.0);
            let output = if !state.output_path.is_empty() { Some(PathBuf::from(&state.output_path)) } else { None };

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

        if ui.add(egui::Button::new(egui::RichText::new("📦 Compress").size(16.0)).fill(COLOR_ACCENT_GREEN)).clicked() {
            let core = self.core.clone();
            let input = PathBuf::from(&state.input_path);
            let size: f64 = state.compress_size.parse().unwrap_or(25.0);
            let output = if !state.output_path.is_empty() { Some(PathBuf::from(&state.output_path)) } else { None };

            tokio::spawn(async move {
                let _ = core.compress_video(&input, size, output).await;
            });
        }
    }

    fn render_vectorize_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🎨 Vectorize to SVG").color(COLOR_ACCENT_PURPLE));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, state);
        ui.add_space(10.0);

        ui.label("Output Directory:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut state.output_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(get_default_videos_path())
                    .pick_folder() {
                    state.output_path = path.to_string_lossy().to_string();
                }
            }
        });
        ui.add_space(20.0);

        if ui.add(egui::Button::new(egui::RichText::new("🎨 Convert to SVG").size(16.0)).fill(COLOR_ACCENT_PURPLE)).clicked() {
            let core = self.core.clone();
            let input = PathBuf::from(&state.input_path);
            let output = PathBuf::from(&state.output_path);

            tokio::spawn(async move {
                let _ = core.vectorize_video(&input, &output, "color").await;
            });
        }
    }

    fn render_upscale_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🔎 Infinite Upscale").color(COLOR_ACCENT_ORANGE));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, state);
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Scale Factor:");
            ui.add(egui::TextEdit::singleline(&mut state.scale_factor).desired_width(60.0));
            ui.label("x");
        });
        ui.add_space(10.0);

        self.render_output_file_picker(ui, state);
        ui.add_space(20.0);

        if ui.add(egui::Button::new(egui::RichText::new("🔎 Upscale Video").size(16.0)).fill(COLOR_ACCENT_ORANGE)).clicked() {
            let core = self.core.clone();
            let input = PathBuf::from(&state.input_path);
            let output = PathBuf::from(&state.output_path);
            let scale: f64 = state.scale_factor.parse().unwrap_or(2.0);

            tokio::spawn(async move {
                let _ = core.upscale_video(&input, scale, &output).await;
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

        if ui.add(egui::Button::new(egui::RichText::new("🧠 Process Request").size(16.0)).fill(COLOR_ACCENT_BLUE)).clicked() {
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
        ui.add(
            egui::TextEdit::multiline(&mut state.intent)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
        ui.add_space(5.0);
        ui.checkbox(
            &mut state.is_funny_bits_enabled,
            "🎭 Enable Funny Mode (Commentary + Transitions)",
        );
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
                    let _ = core.embody_intent(&input, &intent, &output).await;
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
                let funny = state.is_funny_bits_enabled;

                tokio::spawn(async move {
                    let _ = core.process_youtube_intent(&input, &intent, output, None, funny).await;
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
        ui.text_edit_singleline(&mut state.voice_profile);
        ui.add_space(20.0);

        if ui.add(egui::Button::new(egui::RichText::new("🎓 Analyze & Learn").size(16.0)).fill(COLOR_ACCENT_GREEN)).clicked() {
            let core = self.core.clone();
            let input = PathBuf::from(&state.input_path);
            let name = state.voice_profile.clone();

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
             self.core.log("Suggest feature pending core implementation.");
        }
    }

    fn render_voice_record_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🎙️ Record Voice").color(COLOR_ACCENT_RED));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Recording Duration (seconds):");
        ui.add(egui::TextEdit::singleline(&mut state.clip_duration).desired_width(80.0));
        ui.add_space(10.0);

        self.render_output_file_picker(ui, state);
        ui.add_space(20.0);

        if ui.add(egui::Button::new(egui::RichText::new("🎙️ Start Recording").size(16.0)).fill(COLOR_ACCENT_RED)).clicked() {
            let core = self.core.clone();
            let duration: u32 = state.clip_duration.parse().unwrap_or(5);
            let output = if !state.output_path.is_empty() { Some(PathBuf::from(&state.output_path)) } else { None };

            tokio::spawn(async move {
                let _ = core.voice_record(output, duration).await;
            });
        }
    }

    fn render_voice_clone_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🎭 Clone Voice").color(COLOR_ACCENT_PURPLE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Voice Sample (Audio File):");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut state.input_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Audio", &["wav", "mp3"])
                    .set_directory(get_default_videos_path())
                    .pick_file() {
                    state.input_path = path.to_string_lossy().to_string();
                }
            }
        });
        ui.add_space(10.0);

        ui.label("Profile Name:");
        ui.text_edit_singleline(&mut state.voice_profile);
        ui.add_space(20.0);

        if ui.add(egui::Button::new(egui::RichText::new("🎭 Create Voice Profile").size(16.0)).fill(COLOR_ACCENT_PURPLE)).clicked() {
            let core = self.core.clone();
            let input = PathBuf::from(&state.input_path);
            let name = if !state.voice_profile.is_empty() { Some(state.voice_profile.clone()) } else { None };

            tokio::spawn(async move {
                let _ = core.voice_clone(&input, name).await;
            });
        }
    }

    fn render_voice_speak_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🗣️ Text to Speech").color(COLOR_ACCENT_ORANGE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Text to Speak:");
        ui.add(
            egui::TextEdit::multiline(&mut state.voice_text)
                .desired_rows(4)
                .desired_width(f32::INFINITY),
        );
        ui.add_space(10.0);

        ui.label("Voice Profile (optional):");
        ui.text_edit_singleline(&mut state.voice_profile);
        ui.add_space(10.0);

        self.render_output_file_picker(ui, state);
        ui.add_space(20.0);

        if ui.add(egui::Button::new(egui::RichText::new("🗣️ Generate Speech").size(16.0)).fill(COLOR_ACCENT_ORANGE)).clicked() {
            let core = self.core.clone();
            let text = state.voice_text.clone();
            let profile = if !state.voice_profile.is_empty() { Some(state.voice_profile.clone()) } else { None };
            let output = if !state.output_path.is_empty() { Some(PathBuf::from(&state.output_path)) } else { None };

            tokio::spawn(async move {
                let _ = core.voice_speak(&text, profile, output).await;
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

        if ui.add(egui::Button::new(egui::RichText::new("🛡️ Activate Sentinel").size(16.0)).fill(COLOR_ACCENT_RED)).clicked() {
            let core = self.core.clone();
            let mode = state.guard_mode.clone();
            let watch = if !state.guard_watch_path.is_empty() { Some(PathBuf::from(&state.guard_watch_path)) } else { None };

            tokio::spawn(async move {
                core.activate_sentinel(&mode, watch).await;
            });
        }
    }

    fn render_research_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🔍 Research Topic").color(COLOR_ACCENT_BLUE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Research Topic:");
        ui.text_edit_singleline(&mut state.research_topic);
        ui.add_space(20.0);

        if ui.add(egui::Button::new(egui::RichText::new("🔍 Search").size(16.0)).fill(COLOR_ACCENT_BLUE)).clicked() {
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
                let mut media_exp = self.tree_state.media_expanded;
                let mut visual_exp = self.tree_state.visual_expanded;
                let mut ai_exp = self.tree_state.ai_core_expanded;
                let mut voice_exp = self.tree_state.voice_expanded;
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
                    ],
                ) {
                    new_cmd = Some(cmd);
                }

                // Visual
                if let Some(cmd) = self.render_tree_category(
                    ui,
                    "Visual",
                    "🎨",
                    COLOR_ACCENT_PURPLE,
                    &mut visual_exp,
                    vec![
                        ("✨", "Vectorize", ActiveCommand::Vectorize),
                        ("🔎", "Upscale", ActiveCommand::Upscale),
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

                // Voice
                if let Some(cmd) = self.render_tree_category(
                    ui,
                    "Voice",
                    "🗣️",
                    COLOR_ACCENT_GREEN,
                    &mut voice_exp,
                    vec![
                        ("🎙️", "Voice", ActiveCommand::Voice),
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
                self.tree_state.voice_expanded = voice_exp;
                self.tree_state.security_expanded = security_exp;
                self.tree_state.research_expanded = research_exp;
                self.tree_state.audio_expanded = audio_exp;

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

        // Always request repaint to show log updates from background threads
        ctx.request_repaint();
    }
}

pub fn run_gui(core: Arc<AgentCore>) -> Result<(), eframe::Error> {
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
        Box::new(|_cc| Ok(Box::new(SynoidApp::new(core)))),
    )
}
