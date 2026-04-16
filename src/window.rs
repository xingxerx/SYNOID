// SYNOID Embodied Agent GUI with Tree-Organized Commands
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// "Command Center" Premium Interface Design
// Deep Dark Theme | Tree Sidebar | Professional Typography

use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::agent::core_systems::core::AgentCore;

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
    std::env::var("WSL_DISTRO_NAME").is_ok()
        || std::fs::read_to_string("/proc/version")
            .map(|s| s.contains("Microsoft") || s.contains("WSL"))
            .unwrap_or(false)
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

#[derive(PartialEq, Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum ActiveCommand {
    Dashboard,
    // Media
    Clip,
    Compress,
    Combine,
    Youtube,
    Editor,
    // Visual

    // AI Core
    Brain,
    Embody,
    Learn,
    Suggest,
    Process,
    // Security
    Guard,
    // Research
    Research,
    // Audio
    AudioMixer,
    Discovery,
    // System
    GpuStatus,
    // Self-improvement
    AutoImprove,
    // Gemma 4 builder/improver
    Gemma4,
}

impl Default for ActiveCommand {
    fn default() -> Self {
        Self::Dashboard
    }
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
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
    // Timeline and editing
    pub timeline_zoom: f32,
    pub intent_history: Vec<String>,
    pub intent_history_index: usize,
    pub track_audio: String,
    pub track_overlay: String,
    // Editor API session tracking
    pub editor_session_id: Option<String>,
    pub editor_api_status: String,
    pub ai_edit_running: bool,
    pub discovered_files: Vec<crate::agent::global_discovery::DiscoveredFile>,
    pub is_scanning: bool,
    pub discovery_query: String,
    pub recent_jobs: Vec<crate::agent::editor_queue::EditJob>,
    // Editor feature toggles
    pub enable_subtitles: bool,
    pub enable_censoring: bool,
    pub enable_audio_enhancement: bool,
    pub enable_silence_removal: bool,
    // AutoImprove
    pub improve_benchmark: String,
    pub improve_candidates: String,
    pub improve_iterations: String,
    pub improve_status: String,
    // Gemma 4 harness
    pub gemma4_task: String,
    pub gemma4_max_steps: String,
    pub gemma4_dry_run: bool,
    pub gemma4_log: String,
    // System
    pub is_restarting: bool,
    pub port: u16,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct PersistedSettings {
    input_path: String,
    output_path: String,
    intent: String,
    youtube_url: String,
    clip_start: String,
    clip_duration: String,
    compress_size: String,
    scale_factor: String,
    research_topic: String,
    style_name: String,
    is_autonomous_running: bool,
    guard_mode: String,
    guard_watch_path: String,
    active_editor_tab: String,
    active_command: ActiveCommand,
    tree_state: TreeState,
    timeline_zoom: f32,
    track_audio: String,
    track_overlay: String,
    discovery_query: String,
    enable_subtitles: bool,
    enable_censoring: bool,
    enable_audio_enhancement: bool,
    enable_silence_removal: bool,
    improve_benchmark: String,
    improve_candidates: String,
    improve_iterations: String,
}

impl Default for PersistedSettings {
    fn default() -> Self {
        Self {
            input_path: String::new(),
            output_path: "Video/output.mp4".to_string(),
            intent: String::new(),
            youtube_url: String::new(),
            clip_start: "0.0".to_string(),
            clip_duration: "10.0".to_string(),
            compress_size: "25.0".to_string(),
            scale_factor: "2.0".to_string(),
            research_topic: String::new(),
            style_name: String::new(),
            is_autonomous_running: false,
            guard_mode: "all".to_string(),
            guard_watch_path: String::new(),
            active_editor_tab: "Media".to_string(),
            active_command: ActiveCommand::Dashboard,
            tree_state: TreeState {
                media_expanded: true,
                visual_expanded: true,
                ai_core_expanded: true,
                security_expanded: false,
                research_expanded: false,
                audio_expanded: true,
            },
            timeline_zoom: 0.5,
            track_audio: String::new(),
            track_overlay: String::new(),
            discovery_query: String::new(),
            enable_subtitles: true,
            enable_censoring: false,
            enable_audio_enhancement: true,
            enable_silence_removal: false,
            improve_benchmark: String::new(),
            improve_candidates: "4".to_string(),
            improve_iterations: String::new(),
        }
    }
}

fn load_settings(instance_id: &str) -> PersistedSettings {
    let filename = format!("synoid_settings_{}.json", instance_id);

    if let Ok(data) = std::fs::read_to_string(&filename) {
        if let Ok(settings) = serde_json::from_str(&data) {
            return settings;
        }
    }
    PersistedSettings::default()
}

fn save_settings(
    instance_id: &str,
    state: &UiState,
    active_command: ActiveCommand,
    tree_state: &TreeState,
) {
    let filename = format!("synoid_settings_{}.json", instance_id);
    let intent_filename = format!("synoid_intent_{}.txt", instance_id);

    // Save settings
    let settings = PersistedSettings {
        input_path: state.input_path.clone(),
        output_path: state.output_path.clone(),
        intent: state.intent.clone(),
        youtube_url: state.youtube_url.clone(),
        clip_start: state.clip_start.clone(),
        clip_duration: state.clip_duration.clone(),
        compress_size: state.compress_size.clone(),
        scale_factor: state.scale_factor.clone(),
        research_topic: state.research_topic.clone(),
        style_name: state.style_name.clone(),
        is_autonomous_running: state.is_autonomous_running,
        guard_mode: state.guard_mode.clone(),
        guard_watch_path: state.guard_watch_path.clone(),
        active_editor_tab: state.active_editor_tab.clone(),
        active_command,
        tree_state: tree_state.clone(),
        timeline_zoom: state.timeline_zoom,
        track_audio: state.track_audio.clone(),
        track_overlay: state.track_overlay.clone(),
        discovery_query: state.discovery_query.clone(),
        enable_subtitles: state.enable_subtitles,
        enable_censoring: state.enable_censoring,
        enable_audio_enhancement: state.enable_audio_enhancement,
        enable_silence_removal: state.enable_silence_removal,
        improve_benchmark: state.improve_benchmark.clone(),
        improve_candidates: state.improve_candidates.clone(),
        improve_iterations: state.improve_iterations.clone(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(filename, json);
    }

    // Save intent
    let _ = std::fs::write(intent_filename, &state.intent);
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
        let instance_id = &core.instance_id;
        let intent_filename = format!("synoid_intent_{}.txt", instance_id);
        let mut settings = load_settings(instance_id);

        if settings.intent.is_empty() {
            if let Ok(saved_intent) = std::fs::read_to_string(intent_filename) {
                settings.intent = saved_intent;
            }
        }

        ui_state.input_path = settings.input_path.clone();
        ui_state.output_path = settings.output_path.clone();
        ui_state.intent = settings.intent.clone();
        ui_state.youtube_url = settings.youtube_url.clone();
        ui_state.clip_start = settings.clip_start.clone();
        ui_state.clip_duration = settings.clip_duration.clone();
        ui_state.compress_size = settings.compress_size.clone();
        ui_state.scale_factor = settings.scale_factor.clone();
        ui_state.research_topic = settings.research_topic.clone();
        ui_state.style_name = settings.style_name.clone();
        ui_state.active_editor_tab = settings.active_editor_tab.clone();
        ui_state.guard_mode = settings.guard_mode.clone();
        ui_state.guard_watch_path = settings.guard_watch_path.clone();
        ui_state.timeline_zoom = settings.timeline_zoom;
        ui_state.intent_history = vec![ui_state.intent.clone()];
        ui_state.intent_history_index = 0;
        ui_state.track_audio = settings.track_audio.clone();
        ui_state.track_overlay = settings.track_overlay.clone();
        ui_state.editor_session_id = None;
        ui_state.editor_api_status = "No active session".to_string();
        ui_state.ai_edit_running = false;
        ui_state.is_autonomous_running = settings.is_autonomous_running;
        ui_state.discovery_query = settings.discovery_query.clone();
        ui_state.enable_subtitles = settings.enable_subtitles;
        ui_state.enable_censoring = settings.enable_censoring;
        ui_state.enable_audio_enhancement = settings.enable_audio_enhancement;
        ui_state.enable_silence_removal = settings.enable_silence_removal;
        ui_state.improve_benchmark = settings.improve_benchmark.clone();
        ui_state.improve_candidates = settings.improve_candidates.clone();
        ui_state.improve_iterations = settings.improve_iterations.clone();
        ui_state.improve_status = String::new();
        ui_state.is_restarting = false;
        
        // Extract port from instance_id (e.g., "_3005" -> 3005) or default to 3000
        let port = if core.instance_id.starts_with('_') {
            core.instance_id[1..].parse::<u16>().unwrap_or(3000)
        } else {
            3000
        };
        ui_state.port = port;

        let tree_state = settings.tree_state.clone();
        let active_command = settings.active_command;

        // Auto-start autonomous learning if enabled in settings
        if ui_state.is_autonomous_running {
            let core_auton = core.clone();
            let inst_id = core.instance_id.clone();
            tokio::spawn(async move {
                tracing::info!(
                    "[GUI] Auto-starting Autonomous Learner for instance {}",
                    inst_id
                );
                core_auton.start_autonomous_learning();
            });
        }

        // Start background poller for Hive Mind status
        let core_clone = core.clone();
        let ui_state_clone = Arc::new(Mutex::new(ui_state));
        let return_state = ui_state_clone.clone();

        tokio::spawn(async move {
            loop {
                let status = core_clone.get_hive_status().await;
                let jobs = core_clone.list_jobs().await;
                if let Ok(mut state) = ui_state_clone.lock() {
                    state.hive_mind_status = status;
                    state.recent_jobs = jobs;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
        });

        Self {
            core,
            ui_state: return_state,
            tree_state,
            active_command,
            preview_texture: None,
        }
    }

    fn configure_style(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = COLOR_BG_DARK;
        visuals.panel_fill = COLOR_PANEL_BG;
        visuals.extreme_bg_color = COLOR_BG_DARK;
        visuals.faint_bg_color = egui::Color32::from_rgb(35, 35, 40);
        visuals.widgets.noninteractive.bg_fill = COLOR_PANEL_BG;
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, COLOR_TEXT_PRIMARY);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, COLOR_TEXT_SECONDARY);
        visuals.widgets.inactive.bg_fill = COLOR_BG_DARK;
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, COLOR_TEXT_PRIMARY);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, COLOR_TEXT_SECONDARY);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(45, 45, 55);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, COLOR_ACCENT_BLUE);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, COLOR_ACCENT_BLUE);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(50, 50, 60);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, COLOR_ACCENT_ORANGE);
        visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(80, 160, 255, 50);
        visuals.selection.stroke = egui::Stroke::new(1.0, COLOR_ACCENT_BLUE);
        visuals.window_stroke = egui::Stroke::new(1.0, COLOR_TEXT_SECONDARY);
        visuals.override_text_color = Some(COLOR_TEXT_PRIMARY);

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        // All text in monospace — enforces the terminal aesthetic
        style.text_styles = [
            (
                egui::TextStyle::Heading,
                egui::FontId::new(16.0, egui::FontFamily::Monospace),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::new(13.0, egui::FontFamily::Monospace),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(12.0, egui::FontFamily::Monospace),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(12.0, egui::FontFamily::Monospace),
            ),
            (
                egui::TextStyle::Small,
                egui::FontId::new(10.0, egui::FontFamily::Monospace),
            ),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        ctx.set_style(style);
    }

    /// Draws a CRT-style bordered panel with a floating uppercase label.
    /// Returns the inner `Rect` where content should be drawn.
    fn crt_panel(
        ui: &mut egui::Ui,
        label: &str,
        color: egui::Color32,
        add_contents: impl FnOnce(&mut egui::Ui),
    ) {
        let frame = egui::Frame::none()
            .stroke(egui::Stroke::new(1.0, color))
            .inner_margin(egui::Margin::same(10.0))
            .fill(egui::Color32::TRANSPARENT);

        let response = frame.show(ui, |ui| {
            // Floating label painted above top-left corner of the frame
            let rect = ui.min_rect();
            let label_pos = egui::pos2(rect.min.x + 10.0, rect.min.y - 8.0);
            let galley = ui.painter().layout_no_wrap(
                format!("[ {} ]", label.to_uppercase()),
                egui::FontId::new(10.0, egui::FontFamily::Monospace),
                color,
            );
            // Erase background behind label text
            ui.painter().rect_filled(
                egui::Rect::from_min_size(label_pos, galley.size())
                    .expand2(egui::vec2(3.0, 0.0)),
                0.0,
                COLOR_BG_DARK,
            );
            ui.painter().galley(label_pos, galley, color);
            add_contents(ui);
        });
        let _ = response;
    }

    /// Paints a subtle CRT scanline overlay over the entire window.
    fn render_crt_overlay(ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("crt_scanlines"),
        ));
        // Horizontal scanlines every 3px
        let scanline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 40);
        let mut y = screen.min.y;
        while y < screen.max.y {
            painter.rect_filled(
                egui::Rect::from_min_size(egui::pos2(screen.min.x, y), egui::vec2(screen.width(), 1.0)),
                0.0,
                scanline_color,
            );
            y += 3.0;
        }
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
            ActiveCommand::Dashboard => self.render_dashboard(ui, state),
            ActiveCommand::Clip => self.render_clip_panel(ui, state),
            ActiveCommand::Compress => self.render_compress_panel(ui, state),
            ActiveCommand::Combine => self.render_combine_panel(ui, state),
            ActiveCommand::Youtube => self.render_youtube_panel(ui, state),
            ActiveCommand::Brain => self.render_brain_panel(ui, state),
            ActiveCommand::Embody => self.render_embody_panel(ui, state),
            ActiveCommand::Learn => self.render_learn_panel(ui, state),
            ActiveCommand::Suggest => self.render_suggest_panel(ui, state),
            ActiveCommand::Process => self.render_process_panel(ui, state),
            ActiveCommand::Guard => self.render_guard_panel(ui, state),
            ActiveCommand::Research => self.render_research_panel(ui, state),
            ActiveCommand::AudioMixer => self.render_audio_mixer_panel(ui, state),
            ActiveCommand::Discovery => self.render_discovery_panel(ui, state),
            ActiveCommand::GpuStatus => self.render_gpu_status_panel(ui, state),
            ActiveCommand::AutoImprove => self.render_auto_improve_panel(ui, state),
            ActiveCommand::Gemma4 => self.render_gemma4_panel(ui, state),
            ActiveCommand::Editor => {
                // Create/reuse session then open React editor in browser
                let _core = self.core.clone();
                let ui_ptr = self.ui_state.clone();
                let session_id = state.editor_session_id.clone();
                if session_id.is_none() {
                    tokio::spawn(async move {
                        match reqwest::Client::new()
                            .post("http://127.0.0.1:3000/api/editor/sessions")
                            .send()
                            .await
                        {
                            Ok(r) => {
                                if let Ok(json) = r.json::<serde_json::Value>().await {
                                    let id = json["id"].as_str().unwrap_or("").to_string();
                                    if let Ok(mut s) = ui_ptr.lock() {
                                        s.editor_session_id = Some(id.clone());
                                        s.editor_api_status =
                                            format!("Session: {}", &id[..8.min(id.len())]);
                                    }
                                }
                            }
                            Err(_) => {
                                if let Ok(mut s) = ui_ptr.lock() {
                                    s.editor_api_status = "⚠ Server not running".to_string();
                                }
                            }
                        }
                    });
                }
            }
        }
    }

    fn render_dashboard(&self, ui: &mut egui::Ui, state: &mut UiState) {
        // ── Top header bar ──────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("SYNOID_OS v2.0")
                    .size(18.0)
                    .color(COLOR_ACCENT_ORANGE)
                    .strong(),
            );
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("[ LIVE ]")
                    .size(10.0)
                    .color(COLOR_BG_DARK)
                    .background_color(COLOR_ACCENT_ORANGE)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Reload Button
                let is_restarting = state.is_restarting;
                let (btn_label, btn_color) = if is_restarting {
                    ("RESTARTING...", COLOR_TEXT_SECONDARY)
                } else {
                    ("⟳ RELOAD", COLOR_ACCENT_PURPLE)
                };

                if ui.add_enabled(!is_restarting, egui::Button::new(egui::RichText::new(btn_label).size(10.0).color(COLOR_BG_DARK)).fill(btn_color)).clicked() {
                    state.is_restarting = true;
                    let core = self.core.clone();
                    let port = state.port;
                    tokio::spawn(async move {
                        core.initiate_graceful_restart(port).await;
                    });
                }
                
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("SENTINEL: SECURE")
                        .size(10.0)
                        .color(COLOR_ACCENT_BLUE),
                );
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("GPU: RTX_5080_NVENC")
                        .size(10.0)
                        .color(COLOR_TEXT_SECONDARY),
                );
            });
        });

        ui.add_space(6.0);
        ui.painter().hline(
            ui.min_rect().min.x..=ui.min_rect().min.x + ui.available_width(),
            ui.cursor().min.y,
            egui::Stroke::new(1.0, COLOR_TEXT_SECONDARY),
        );
        ui.add_space(8.0);

        // ── 3-column CRT grid ───────────────────────────────────────────────
        ui.columns(3, |cols| {
            // LEFT: Kernel Health + Active Processes
            let col = &mut cols[0];
            Self::crt_panel(col, "Kernel Health", COLOR_ACCENT_ORANGE, |ui| {
                let stats = [
                    ("GHOST_THREAD", 0.98f32),
                    ("MOTOR_CORTEX", 0.45f32),
                    ("NEURAL_BRAIN", 1.0f32),
                ];
                for (label, pct) in stats {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(label).size(9.0).color(COLOR_TEXT_SECONDARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.0}%", pct * 100.0))
                                    .size(9.0)
                                    .color(COLOR_ACCENT_ORANGE),
                            );
                        });
                    });
                    // Stat bar
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 6.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 0.0, COLOR_TEXT_SECONDARY);
                    let fill = egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(rect.width() * pct, rect.height()),
                    );
                    ui.painter().rect_filled(fill, 0.0, COLOR_ACCENT_ORANGE);
                    ui.add_space(4.0);
                }
            });

            col.add_space(8.0);
            Self::crt_panel(col, "Active Processes", COLOR_ACCENT_ORANGE, |ui| {
                let procs = [
                    ("[OK]  ffmpeg_service.bin", COLOR_ACCENT_BLUE),
                    ("[OK]  neural_brain.ghost", COLOR_ACCENT_BLUE),
                    ("[WAIT] prod_queue.sync", COLOR_ACCENT_PURPLE),
                    ("[OK]  sentinel.guard", COLOR_ACCENT_BLUE),
                ];
                for (label, color) in procs {
                    ui.label(egui::RichText::new(label).size(10.0).color(color));
                }

                let auton_label = if state.is_autonomous_running {
                    "[ON]  autonomous_learner"
                } else {
                    "[OFF] autonomous_learner"
                };
                let auton_color = if state.is_autonomous_running {
                    COLOR_ACCENT_BLUE
                } else {
                    COLOR_ACCENT_RED
                };
                if ui
                    .add(
                        egui::Label::new(
                            egui::RichText::new(auton_label).size(10.0).color(auton_color),
                        )
                        .sense(egui::Sense::click()),
                    )
                    .clicked()
                {
                    state.is_autonomous_running = !state.is_autonomous_running;
                    let core = self.core.clone();
                    let running = state.is_autonomous_running;
                    tokio::spawn(async move {
                        if running {
                            core.start_autonomous_learning();
                        } else {
                            core.stop_autonomous_learning();
                        }
                    });
                }
            });

            // CENTER: Command Central terminal
            let col = &mut cols[1];
            Self::crt_panel(col, "Command Central", COLOR_ACCENT_ORANGE, |ui| {
                // Terminal log scroll
                let log_height = 160.0;
                egui::ScrollArea::vertical()
                    .max_height(log_height)
                    .id_salt("dash_term_scroll")
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("SYNOID Kernel Initialized.")
                                .size(11.0)
                                .color(COLOR_TEXT_SECONDARY),
                        );
                        ui.label(
                            egui::RichText::new("Loading cortex modules...")
                                .size(11.0)
                                .color(COLOR_TEXT_SECONDARY),
                        );
                        ui.label(
                            egui::RichText::new("> Welcome back, Operator.")
                                .size(11.0)
                                .color(COLOR_ACCENT_BLUE),
                        );
                        ui.label(
                            egui::RichText::new("> Brain ready for intent processing.")
                                .size(11.0)
                                .color(COLOR_ACCENT_BLUE),
                        );
                        let hive = &state.hive_mind_status;
                        if !hive.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("> {}", hive))
                                    .size(11.0)
                                    .color(COLOR_ACCENT_PURPLE),
                            );
                        }
                        for suggestion in state.suggestions.iter().take(3) {
                            ui.label(
                                egui::RichText::new(format!("  >> {}", suggestion))
                                    .size(10.0)
                                    .color(COLOR_TEXT_SECONDARY),
                            );
                        }
                    });

                ui.add_space(4.0);
                ui.painter().hline(
                    ui.min_rect().min.x..=ui.min_rect().min.x + ui.available_width(),
                    ui.cursor().min.y,
                    egui::Stroke::new(1.0, COLOR_TEXT_SECONDARY),
                );
                ui.add_space(4.0);

                // Intent input line
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(">").size(13.0).color(COLOR_ACCENT_ORANGE));
                    let te = egui::TextEdit::singleline(&mut state.intent)
                        .desired_width(f32::INFINITY)
                        .frame(false)
                        .text_color(COLOR_ACCENT_ORANGE)
                        .hint_text("Type creative intent or command...");
                    ui.add(te);
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let btn_style = |text: &str, color: egui::Color32| {
                        egui::Button::new(
                            egui::RichText::new(text).size(11.0).color(color),
                        )
                        .stroke(egui::Stroke::new(1.0, color))
                        .fill(egui::Color32::TRANSPARENT)
                    };
                    if ui.add(btn_style("EXECUTE", COLOR_ACCENT_ORANGE)).clicked() {
                        let core = self.core.clone();
                        let intent = state.intent.clone();
                        tokio::spawn(async move {
                            let _ = core.process_brain_request(&intent).await;
                        });
                    }
                    if ui.add(btn_style("REFRESH", COLOR_ACCENT_BLUE)).clicked() {
                        let core = self.core.clone();
                        tokio::spawn(async move {
                            let _ = core.initialize_hive_mind().await;
                        });
                    }
                });
            });

            // RIGHT: Log Feed
            let col = &mut cols[2];
            Self::crt_panel(col, "Log Feed", COLOR_ACCENT_ORANGE, |ui| {
                let logs = self.core.get_logs();
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .id_salt("dash_log_scroll")
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for log in logs.iter().rev().take(20) {
                            let color = if log.contains("ERR") || log.contains("WARN") {
                                COLOR_ACCENT_RED
                            } else if log.contains("OK") || log.contains("SUCCESS") {
                                COLOR_ACCENT_BLUE
                            } else {
                                COLOR_TEXT_SECONDARY
                            };
                            ui.label(egui::RichText::new(log).size(9.0).color(color));
                        }
                    });
            });
        });

        // ── Job History ──────────────────────────────────────────────────────
        ui.add_space(10.0);
        self.render_job_history(ui, state);

        // ── Human Control Index ──────────────────────────────────────────────
        ui.add_space(8.0);
        self.render_hci_panel(ui);
    }

    fn render_job_history(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading("🎬 Recent Job History");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑 Clear Completed").clicked() {
                        let core = self.core.clone();
                        tokio::spawn(async move {
                            let _ = core.editor_queue.clear_completed().await;
                        });
                    }
                });
            });
            ui.separator();
            ui.add_space(5.0);

            if state.recent_jobs.is_empty() {
                ui.label(
                    egui::RichText::new("No active or recent jobs.")
                        .color(COLOR_TEXT_SECONDARY)
                        .italics(),
                );
            } else {
                egui::ScrollArea::vertical()
                    .max_height(350.0)
                    .show(ui, |ui| {
                        for job in state.recent_jobs.iter().rev() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    let status_color = match job.status {
                                        crate::agent::editor_queue::JobStatus::Queued => {
                                            egui::Color32::from_rgb(150, 150, 150)
                                        }
                                        crate::agent::editor_queue::JobStatus::Processing {
                                            ..
                                        } => COLOR_ACCENT_BLUE,
                                        crate::agent::editor_queue::JobStatus::Completed {
                                            ..
                                        } => COLOR_ACCENT_GREEN,
                                        crate::agent::editor_queue::JobStatus::Failed(_) => {
                                            COLOR_ACCENT_RED
                                        }
                                    };

                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Job {}",
                                            &job.id.to_string()[..8]
                                        ))
                                        .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:?}", job.status))
                                            .color(status_color),
                                    );

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(format!(
                                                "{:.1}s",
                                                job.created_at.elapsed().as_secs_f32()
                                            ));
                                        },
                                    );
                                });

                                ui.label(
                                    egui::RichText::new(&job.intent)
                                        .small()
                                        .color(COLOR_TEXT_SECONDARY),
                                );

                                if let crate::agent::editor_queue::JobStatus::Completed {
                                    kept_ratio,
                                    duration_secs,
                                    ..
                                } = job.status
                                {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Kept: {:.0}% | Final: {:.1}s",
                                                kept_ratio * 100.0,
                                                duration_secs
                                            ))
                                            .small(),
                                        );

                                        ui.add_space(10.0);
                                        ui.label("Rate Edit:");
                                        for i in 1..=5 {
                                            let star = if i <= 3 { "⭐" } else { "☆" }; // Placeholder or real
                                            if ui.button(format!("{} {}", i, star)).clicked() {
                                                let core = self.core.clone();
                                                let job_id = job.id;
                                                tokio::spawn(async move {
                                                    core.record_user_rating(job_id, i as u8).await;
                                                });
                                            }
                                        }
                                    });
                                }
                            });
                            ui.add_space(4.0);
                        }
                    });
            }
        });
    }

    /// HCI (Human Control Index) authorship score panel.
    fn render_hci_panel(&self, ui: &mut egui::Ui) {
        let score = self.core.hci_score();
        let display = self.core.hci_display();
        let hci = &self.core.hci;
        let director = hci
            .director_decisions
            .load(std::sync::atomic::Ordering::Relaxed);
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
            let (rect, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 14.0), egui::Sense::hover());
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
            ui.label(
                egui::RichText::new(interp)
                    .size(11.0)
                    .color(COLOR_TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(&display)
                    .size(10.0)
                    .monospace()
                    .color(COLOR_TEXT_SECONDARY),
            );
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
                ui.label(
                    egui::RichText::new("Select a video file to begin")
                        .small()
                        .color(COLOR_TEXT_SECONDARY),
                );
                ui.add_space(50.0);
            }

            ui.add_space(10.0);
            if !state.input_path.is_empty() {
                ui.label(
                    egui::RichText::new(&state.input_path)
                        .small()
                        .color(COLOR_TEXT_SECONDARY),
                );
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    if state.video_player.is_some() {
                        if ui
                            .button(egui::RichText::new("⏹ Stop").color(COLOR_ACCENT_RED))
                            .clicked()
                        {
                            state.video_player = None;
                        }
                    } else {
                        if ui
                            .button(
                                egui::RichText::new("▶ Play in Preview").color(COLOR_ACCENT_GREEN),
                            )
                            .clicked()
                        {
                            match crate::agent::video_player::VideoPlayer::new(
                                &state.input_path,
                                state.video_position,
                            ) {
                                Ok(vp) => state.video_player = Some(vp),
                                Err(e) => self
                                    .core
                                    .log(&format!("[GUI] ❌ Failed to start video player: {}", e)),
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
        if ui
            .add(
                egui::TextEdit::multiline(&mut state.intent)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            )
            .changed()
        {
            save_settings(
                &self.core.instance_id,
                state,
                self.active_command,
                &self.tree_state,
            );
        }
        ui.add_space(5.0);

        ui.add_space(10.0);

        self.render_output_file_picker(ui, state);
        ui.add_space(20.0);

        let has_input = !state.input_path.is_empty();
        if !has_input {
            ui.label(
                egui::RichText::new("⚠️ Enter a URL or file path")
                    .size(12.0)
                    .color(COLOR_ACCENT_RED),
            );
        }

        ui.horizontal(|ui| {
            let button_enabled = has_input;

            // Standard Embodiment (Logic from original embody_intent)
            let embody_btn = egui::Button::new(egui::RichText::new("🤖 Execute Intent").size(16.0))
                .fill(if button_enabled {
                    COLOR_ACCENT_PURPLE
                } else {
                    egui::Color32::from_rgb(80, 80, 80)
                });
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
            let smart_btn = egui::Button::new(egui::RichText::new("⚡ Optimized Edit").size(16.0))
                .fill(if button_enabled {
                    COLOR_ACCENT_ORANGE
                } else {
                    egui::Color32::from_rgb(80, 80, 80)
                });
            if ui.add(smart_btn).clicked() && button_enabled {
                let core = self.core.clone();
                let input = state.input_path.clone();
                let output = if !state.output_path.is_empty() {
                    Some(PathBuf::from(&state.output_path))
                } else {
                    None
                };
                let intent = state.intent.clone();
                let enable_subtitles = state.enable_subtitles;
                tokio::spawn(async move {
                    let _ = core
                        .process_youtube_intent(&input, &intent, output, None, false, 0, enable_subtitles)
                        .await;
                });
            }
        });

        ui.add_space(10.0);
        ui.label(egui::RichText::new("Note: 'Execute Intent' uses full embodied reasoning. 'Optimized Edit' is faster for specific requests.").small().color(COLOR_TEXT_SECONDARY));
    }

    fn render_discovery_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🔍 Global File Discovery").color(COLOR_ACCENT_BLUE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Search for media files across your system:");
        ui.horizontal(|ui| {
            let resp = ui.text_edit_singleline(&mut state.intent);
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let core = self.core.clone();
                let query = state.intent.clone();
                tokio::spawn(async move {
                    let _ = core
                        .process_brain_request(&format!("find video {}", query))
                        .await;
                });
            }
        });

        ui.add_space(20.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Search Results").strong());
            if state.discovered_files.is_empty() {
                ui.label("No files discovered yet. Type a query above.");
            } else {
                for file in &state.discovered_files {
                    ui.horizontal(|ui| {
                        ui.label(format!("📄 {}", file.name));
                        if ui.button("📂 Use").clicked() {
                            state.input_path = file.path.to_string_lossy().to_string();
                        }
                    });
                }
            }
        });
    }

    fn render_learn_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        use crate::agent::video_style_learner::get_download_dir;

        ui.heading(egui::RichText::new("🎓 Learn").color(COLOR_ACCENT_GREEN));
        ui.separator();
        ui.add_space(10.0);

        // ── Section 1: Learn from Downloads ─────────────────────────────────
        ui.label(egui::RichText::new("FROM DOWNLOADS").size(10.0).color(COLOR_TEXT_SECONDARY).strong());
        ui.add_space(6.0);

        let dl_dir = get_download_dir();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Folder:").strong());
            ui.label(
                egui::RichText::new(dl_dir.to_string_lossy().as_ref())
                    .monospace()
                    .color(COLOR_ACCENT_ORANGE),
            );
        });
        ui.add_space(4.0);

        let videos: Vec<std::path::PathBuf> = std::fs::read_dir(&dl_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| matches!(e.to_lowercase().as_str(), "mp4" | "mkv" | "mov" | "avi"))
                    .unwrap_or(false)
            })
            .collect();

        if videos.is_empty() {
            ui.label(
                egui::RichText::new("⚠ No video files found in Download folder.")
                    .color(COLOR_ACCENT_ORANGE),
            );
        } else {
            ui.label(format!("{} video(s) ready to learn from:", videos.len()));
            ui.add_space(4.0);
            egui::ScrollArea::vertical().max_height(120.0).id_salt("dl_list").show(ui, |ui| {
                for v in &videos {
                    let name = v.file_name().unwrap_or_default().to_string_lossy();
                    let size_mb = v.metadata().map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("▶").color(COLOR_ACCENT_GREEN));
                        ui.label(format!("{} ({:.1} MB)", name, size_mb));
                    });
                }
            });
        }
        ui.add_space(8.0);

        let is_running = self.core.status.lock()
            .map(|s| s.contains("Learning"))
            .unwrap_or(false);

        if is_running {
            ui.add_enabled(
                false,
                egui::Button::new(egui::RichText::new("⏳ Learning…").size(15.0))
                    .fill(egui::Color32::from_rgb(60, 80, 60)),
            );
        } else {
            let btn_enabled = !videos.is_empty();
            let btn = egui::Button::new(
                egui::RichText::new("📚 Learn from Downloads").size(15.0),
            )
            .fill(if btn_enabled { COLOR_ACCENT_BLUE } else { egui::Color32::from_rgb(50, 50, 60) });

            if ui.add_enabled(btn_enabled, btn).clicked() {
                let core = self.core.clone();
                tokio::spawn(async move {
                    core.learn_from_downloads().await;
                });
            }
        }

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);

        // ── Section 2: Single File / YouTube ────────────────────────────────
        ui.label(egui::RichText::new("SINGLE FILE / YOUTUBE").size(10.0).color(COLOR_TEXT_SECONDARY).strong());
        ui.add_space(8.0);

        self.render_input_file_picker(ui, state);
        ui.add_space(8.0);

        ui.label("Style Name:");
        ui.text_edit_singleline(&mut state.style_name);
        ui.add_space(12.0);

        ui.label("YouTube / Reference URL:");
        ui.text_edit_singleline(&mut state.youtube_url);
        ui.add_space(6.0);

        let download_enabled = state.youtube_url.starts_with("http");
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("📥 Download & Learn from YouTube").size(14.0),
                )
                .fill(if download_enabled {
                    COLOR_ACCENT_BLUE
                } else {
                    egui::Color32::from_rgb(80, 80, 80)
                }),
            )
            .clicked()
            && download_enabled
        {
            let core = self.core.clone();
            let url = state.youtube_url.clone();

            tokio::spawn(async move {
                if let Err(e) = crate::agent::download_guard::DownloadGuard::validate_url(&url) {
                    tracing::warn!("[GUI] Download blocked by Sentinel: {}", e);
                    return;
                }
                let academy_dir = std::path::Path::new("D:\\SYNOID\\Academy");
                let _ = tokio::fs::create_dir_all(academy_dir).await;
                tracing::info!("[GUI] Fetching reference video for Academy: {}", url);
                if let Ok(info) =
                    crate::agent::source_tools::download_youtube(&url, academy_dir, None).await
                {
                    let local_path = info.local_path;
                    if let Err(e) =
                        crate::agent::download_guard::DownloadGuard::validate_downloaded_file(
                            &local_path,
                        )
                    {
                        tracing::warn!("[GUI] Downloaded file blocked by Sentinel: {}", e);
                        let _ = tokio::fs::remove_file(local_path).await;
                        return;
                    }
                    tracing::info!("[GUI] Extracting neural style templates into Brain...");
                    let _ = core.learn_style(&local_path, &info.title).await;
                } else {
                    tracing::error!("[GUI] Failed to fetch video from YouTube.");
                }
            });
        }

        ui.add_space(10.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🎓 Analyze Local File & Learn").size(15.0))
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

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);

        // ── Section 3: Autonomous Loop ───────────────────────────────────────
        ui.label(egui::RichText::new("AUTONOMOUS").size(10.0).color(COLOR_TEXT_SECONDARY).strong());
        ui.add_space(8.0);

        if ui
            .checkbox(
                &mut state.is_autonomous_running,
                "🚀 Autonomous Learning Loop (Videos + Code + Wiki)",
            )
            .changed()
        {
            save_settings(
                &self.core.instance_id,
                state,
                self.active_command,
                &self.tree_state,
            );
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
    }

    fn render_suggest_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("💡 Get Suggestions").color(COLOR_ACCENT_BLUE));
        ui.separator();
        ui.add_space(10.0);

        self.render_input_file_picker(ui, state);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("💡 Analyze Video").size(16.0))
                    .fill(COLOR_ACCENT_BLUE),
            )
            .clicked()
        {
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
                    if ui.button(format!("{}. {}", i + 1, sugg)).clicked() {
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
        if ui
            .horizontal(|ui| {
                let mut changed = false;
                changed |= ui
                    .radio_value(&mut state.guard_mode, "all".to_string(), "All")
                    .changed();
                changed |= ui
                    .radio_value(&mut state.guard_mode, "sys".to_string(), "Processes")
                    .changed();
                changed |= ui
                    .radio_value(&mut state.guard_mode, "file".to_string(), "Files")
                    .changed();
                changed
            })
            .inner
        {
            save_settings(
                &self.core.instance_id,
                state,
                self.active_command,
                &self.tree_state,
            );
        }
        ui.add_space(10.0);

        ui.label("Watch Path (optional):");
        ui.horizontal(|ui| {
            if ui
                .text_edit_singleline(&mut state.guard_watch_path)
                .changed()
            {
                save_settings(
                    &self.core.instance_id,
                    state,
                    self.active_command,
                    &self.tree_state,
                );
            }
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(get_default_videos_path())
                    .pick_folder()
                {
                    state.guard_watch_path = path.to_string_lossy().to_string();
                    save_settings(
                        &self.core.instance_id,
                        state,
                        self.active_command,
                        &self.tree_state,
                    );
                }
            }
        });
        ui.add_space(20.0);

        let sentinel_active = self
            .core
            .sentinel_active
            .load(std::sync::atomic::Ordering::Relaxed);

        if sentinel_active {
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("🛑 Stop Sentinel").size(16.0))
                        .fill(egui::Color32::from_rgb(100, 100, 100)),
                )
                .clicked()
            {
                self.core.stop_sentinel();
            }
        } else {
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
        }
        ui.add_space(5.0);
        ui.label(
            egui::RichText::new("Note: Requires SYNOID_ENABLE_SENTINEL=true environment variable.")
                .small()
                .color(COLOR_TEXT_SECONDARY),
        );
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
            let res = ui.add(
                egui::TextEdit::singleline(&mut state.input_path)
                    .desired_width(ui.available_width() - 40.0),
            );
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Media", &["mp4", "mkv", "avi", "mov", "wav", "mp3"])
                    .set_directory(get_default_videos_path())
                    .pick_file()
                {
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
            ui.label(
                egui::RichText::new("No tracks detected or file not scanned yet.")
                    .color(COLOR_TEXT_SECONDARY)
                    .italics(),
            );
        } else {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for track in &state.detected_tracks {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Track {}", track.index))
                                        .strong()
                                        .color(COLOR_ACCENT_BLUE),
                                );
                                ui.label(&track.title);
                                if let Some(lang) = &track.language {
                                    ui.label(
                                        egui::RichText::new(format!("({})", lang))
                                            .small()
                                            .color(COLOR_TEXT_SECONDARY),
                                    );
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("🔈 Solo").clicked() {
                                            // Future: Implement solo logic
                                        }
                                        if ui.button("🔇 Mute").clicked() {
                                            // Future: Implement mute logic
                                        }
                                    },
                                );
                            });

                            // Heuristic: If title contains "Background", show a different icon or slider?
                            // For now just show "Adjustable" as requested
                            let slider_label = if track.title.to_lowercase().contains("background")
                            {
                                "Background Volume"
                            } else if track.title.to_lowercase().contains("player")
                                || track.title.to_lowercase().contains("mic")
                            {
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
        if ui
            .button(egui::RichText::new("🎚️ Apply Mix to File").size(16.0))
            .clicked()
        {
            self.core
                .log("Mixer application pending full audio-stitching implementation.");
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
                    .pick_file()
                {
                    state.input_path = path.to_string_lossy().to_string();
                }
            }
        });
    }

    fn render_combine_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("🔗 Combine Videos").color(COLOR_ACCENT_PURPLE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("This feature combines multiple video files into a single output.");
        ui.add_space(10.0);

        self.render_input_file_picker(ui, state);
        ui.add_space(10.0);

        self.render_output_file_picker(ui, state);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("🔗 Combine").size(16.0))
                    .fill(COLOR_ACCENT_PURPLE),
            )
            .clicked()
        {
            ui.label("Combine functionality to be implemented.");
        }
    }

    fn render_youtube_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("📥 YouTube Downloader").color(COLOR_ACCENT_RED));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Download videos from YouTube:");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("URL:");
            ui.add(egui::TextEdit::singleline(&mut state.input_path).desired_width(300.0));
        });
        ui.add_space(10.0);

        self.render_output_file_picker(ui, state);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("📥 Download").size(16.0))
                    .fill(COLOR_ACCENT_RED),
            )
            .clicked()
        {
            ui.label("YouTube download functionality to be implemented.");
        }
    }

    fn render_auto_improve_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        use crate::agent::specialized::auto_improve::ImproveLog;
        use crate::agent::video_style_learner::get_download_dir;

        ui.heading(
            egui::RichText::new("🧬 AutoImprove — Self-Recursing Optimizer")
                .color(COLOR_ACCENT_PURPLE),
        );
        ui.separator();
        ui.add_space(6.0);

        ui.label(egui::RichText::new(
            "Picks a benchmark video from your Download folder, then autonomously \
             mutates EditingStrategy parameters and compounds improvements over time.",
        ).color(COLOR_TEXT_SECONDARY));
        ui.add_space(10.0);

        // ── Quick-pick from Download folder ────────────────────────────────
        let dl_dir = get_download_dir();
        ui.label(egui::RichText::new(format!("Analyzing folder: {}", dl_dir.to_string_lossy())).color(COLOR_ACCENT_GREEN));
        ui.add_space(8.0);

        // ── Candidates & iterations ─────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Candidates/iter:");
            ui.add(
                egui::TextEdit::singleline(&mut state.improve_candidates)
                    .desired_width(50.0),
            );
            ui.add_space(16.0);
            ui.label("Max iterations:");
            ui.add(
                egui::TextEdit::singleline(&mut state.improve_iterations)
                    .hint_text("∞")
                    .desired_width(60.0),
            );
        });
        ui.add_space(12.0);

        // ── Start / Stop ────────────────────────────────────────────────────
        let is_running = self.core.improve_running.load(std::sync::atomic::Ordering::Relaxed);

        ui.horizontal(|ui| {
            if is_running {
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("⏹ Stop").size(15.0))
                            .fill(egui::Color32::from_rgb(100, 100, 100)),
                    )
                    .clicked()
                {
                    self.core.stop_auto_improve();
                }
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("● Running…")
                        .color(COLOR_ACCENT_GREEN)
                        .strong(),
                );
            } else {
                let btn = egui::Button::new(egui::RichText::new("🧬 Start AutoImprove").size(15.0))
                    .fill(COLOR_ACCENT_PURPLE);
                if ui.add(btn).clicked() {
                    let candidates: usize = state.improve_candidates.parse().unwrap_or(4);
                    let iterations: Option<u64> = state.improve_iterations.trim().parse().ok();
                    save_settings(
                        &self.core.instance_id,
                        state,
                        self.active_command,
                        &self.tree_state,
                    );
                    self.core.start_auto_improve(candidates, iterations);
                }
            }

            ui.add_space(16.0);
            if ui.button("🔄 Refresh Status").clicked() {
                let log = ImproveLog::load();
                state.improve_status = format!(
                    "Iterations: {}  |  Experiments: {}  |  Improvements: {}\nBaseline: {:.4}  |  Best: {:.4}  |  Gain: {:.4}{}",
                    log.iterations_run,
                    log.experiments_run,
                    log.improvements,
                    log.baseline_quality,
                    log.best_quality,
                    log.best_quality - log.baseline_quality,
                    if let Some(last) = log.recent.last() {
                        format!("\nLast experiment: quality={:.4}  kept={:.1}%", last.quality_score, last.kept_ratio * 100.0)
                    } else {
                        String::new()
                    }
                );
            }
        });
        ui.add_space(8.0);

        // ── Status display ──────────────────────────────────────────────────
        if !state.improve_status.is_empty() {
            egui::Frame::default()
                .fill(COLOR_PANEL_BG)
                .inner_margin(egui::Margin::same(8.0))
                .rounding(egui::Rounding::same(4.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(&state.improve_status)
                            .monospace()
                            .color(COLOR_ACCENT_GREEN),
                    );
                });
            ui.add_space(8.0);
        }

        // ── Recent experiments table ────────────────────────────────────────
        let log = ImproveLog::load();
        if !log.recent.is_empty() {
            ui.label(egui::RichText::new("Recent Experiments (last 10)").strong());
            ui.add_space(4.0);
            egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                egui::Grid::new("improve_exp_grid")
                    .striped(true)
                    .min_col_width(60.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Iter").strong());
                        ui.label(egui::RichText::new("Quality").strong());
                        ui.label(egui::RichText::new("Kept %").strong());
                        ui.label(egui::RichText::new("Scenes").strong());
                        ui.label(egui::RichText::new("Better?").strong());
                        ui.end_row();

                        for exp in log.recent.iter().rev().take(10) {
                            ui.label(format!("{}", exp.iteration));
                            ui.label(
                                egui::RichText::new(format!("{:.4}", exp.quality_score))
                                    .color(if exp.improved { COLOR_ACCENT_GREEN } else { COLOR_TEXT_PRIMARY }),
                            );
                            ui.label(format!("{:.1}%", exp.kept_ratio * 100.0));
                            ui.label(format!("{}", exp.scene_count));
                            ui.label(if exp.improved {
                                egui::RichText::new("✓").color(COLOR_ACCENT_GREEN)
                            } else {
                                egui::RichText::new("–").color(COLOR_TEXT_SECONDARY)
                            });
                            ui.end_row();
                        }
                    });
            });
            ui.add_space(8.0);
        }

        // ── Program guidance hint ───────────────────────────────────────────
        ui.separator();
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Steer the optimizer by editing  cortex_cache/improve_program.md  \
                 (re-read every iteration while running).",
            )
            .color(COLOR_TEXT_SECONDARY)
            .italics(),
        );
        ui.label(
            egui::RichText::new(
                "Directives: increase <param> | decrease <param> | preserve <param>",
            )
            .color(COLOR_TEXT_SECONDARY)
            .small(),
        );
    }

    fn render_gemma4_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        use std::sync::atomic::Ordering;

        ui.heading(
            egui::RichText::new("🤖 Gemma 4 — Builder & Improver")
                .color(egui::Color32::from_rgb(100, 220, 180)),
        );
        ui.separator();
        ui.add_space(8.0);

        ui.label(
            egui::RichText::new(
                "Give Gemma 4 a task and it will read, write, and cargo-check SYNOID source \
                 code autonomously — looping until the task is done.",
            )
            .color(COLOR_TEXT_SECONDARY)
            .small(),
        );
        ui.add_space(10.0);

        // ── Task input ──────────────────────────────────────────────────────
        ui.label(egui::RichText::new("Task").strong());
        ui.add(
            egui::TextEdit::multiline(&mut state.gemma4_task)
                .desired_rows(4)
                .desired_width(f32::INFINITY)
                .hint_text("e.g. \"improve smart_editor scene detection accuracy\""),
        );
        ui.add_space(8.0);

        // ── Options row ─────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Max steps:");
            ui.add(
                egui::TextEdit::singleline(&mut state.gemma4_max_steps)
                    .desired_width(48.0)
                    .hint_text("16"),
            );
            ui.add_space(16.0);
            ui.checkbox(&mut state.gemma4_dry_run, "Dry run (plan only, no writes)");
        });
        ui.add_space(10.0);

        // ── Start / Stop ────────────────────────────────────────────────────
        let is_running = self.core.gemma4_running.load(Ordering::Relaxed);

        ui.horizontal(|ui| {
            if is_running {
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("⏹ Stop").size(15.0))
                            .fill(egui::Color32::from_rgb(100, 100, 100)),
                    )
                    .clicked()
                {
                    self.core.stop_gemma4();
                }
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("● Gemma 4 working…")
                        .color(egui::Color32::from_rgb(100, 220, 180))
                        .strong(),
                );
            } else {
                let btn = egui::Button::new(egui::RichText::new("🤖 Run Gemma 4").size(15.0))
                    .fill(egui::Color32::from_rgb(40, 140, 100));
                if ui.add(btn).clicked() && !state.gemma4_task.trim().is_empty() {
                    let max_steps: usize = state.gemma4_max_steps.trim().parse().unwrap_or(16);
                    let dry_run = state.gemma4_dry_run;
                    let task = state.gemma4_task.trim().to_string();
                    let ui_state = self.ui_state.clone();
                    self.core.start_gemma4(task, max_steps, dry_run, ui_state);
                }
            }
        });
        ui.add_space(10.0);

        // ── Live log ────────────────────────────────────────────────────────
        if !state.gemma4_log.is_empty() {
            ui.label(egui::RichText::new("Output").strong());
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .max_height(340.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    egui::Frame::default()
                        .fill(COLOR_PANEL_BG)
                        .inner_margin(egui::Margin::same(8.0))
                        .rounding(egui::Rounding::same(4.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(&state.gemma4_log)
                                    .monospace()
                                    .color(egui::Color32::from_rgb(100, 220, 180))
                                    .size(11.0),
                            );
                        });
                });
        }
    }

    fn render_process_panel(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.heading(egui::RichText::new("⚙️ Process Video").color(COLOR_ACCENT_ORANGE));
        ui.separator();
        ui.add_space(10.0);

        ui.label("Advanced video processing options:");
        ui.add_space(10.0);

        self.render_input_file_picker(ui, state);
        ui.add_space(10.0);

        self.render_output_file_picker(ui, state);
        ui.add_space(20.0);

        if ui
            .add(
                egui::Button::new(egui::RichText::new("⚙️ Process").size(16.0))
                    .fill(COLOR_ACCENT_ORANGE),
            )
            .clicked()
        {
            ui.label("Process functionality to be implemented.");
        }
    }

    fn render_gpu_status_panel(&self, ui: &mut egui::Ui, _state: &mut UiState) {
        ui.heading(egui::RichText::new("🖥️ GPU Status").color(COLOR_ACCENT_GREEN));
        ui.separator();
        ui.add_space(10.0);

        ui.label("GPU utilization and status information:");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Status:").strong());
            ui.label("Monitoring GPU...");
        });
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Usage:").strong());
            ui.label("N/A");
        });
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Memory:").strong());
            ui.label("N/A");
        });
        ui.add_space(10.0);

        ui.label(
            egui::RichText::new("Full GPU monitoring to be implemented.")
                .color(COLOR_TEXT_SECONDARY)
                .italics(),
        );
    }

    fn render_output_file_picker(&self, ui: &mut egui::Ui, state: &mut UiState) {
        ui.label("Output File:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut state.output_path);
            if ui.button("📂").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(get_default_videos_path())
                    .save_file()
                {
                    state.output_path = path.to_string_lossy().to_string();
                }
            }
        });
    }
    fn render_editor_layout(&mut self, ctx: &egui::Context, _state: &mut UiState) {
        let color_bg_darkest = egui::Color32::from_rgb(17, 17, 17); // #111111
        let color_panel_bg = egui::Color32::from_rgb(26, 26, 26); // #1A1A1A
        let color_gold = egui::Color32::from_rgb(217, 178, 77); // #D9B24D
        let color_text_light = egui::Color32::from_rgb(230, 230, 230);
        let color_text_dim = egui::Color32::from_rgb(120, 120, 120);

        // 1. Top Navbar
        egui::TopBottomPanel::top("editor_toolbar")
            .exact_height(50.0)
            .frame(
                egui::Frame::none()
                    .fill(color_panel_bg)
                    .inner_margin(egui::Margin::symmetric(16.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("◀  SYNOID").color(color_gold).strong(),
                            )
                            .fill(egui::Color32::TRANSPARENT),
                        )
                        .clicked()
                    {
                        self.active_command = ActiveCommand::Dashboard;
                    }

                    ui.add_space(20.0);
                    if ui
                        .add(egui::Button::new("↶").fill(egui::Color32::TRANSPARENT))
                        .clicked()
                    {}
                    if ui
                        .add(egui::Button::new("↷").fill(egui::Color32::TRANSPARENT))
                        .clicked()
                    {}

                    // Session status pill
                    {
                        let session_status = _state.editor_api_status.clone();
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(&session_status).size(10.0).color(
                            if session_status.starts_with('⚠') {
                                egui::Color32::from_rgb(255, 100, 60)
                            } else {
                                egui::Color32::from_rgb(80, 200, 120)
                            },
                        ));
                    }

                    ui.with_layout(
                        egui::Layout::left_to_right(egui::Align::Center)
                            .with_cross_align(egui::Align::Center),
                        |ui| {
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
                        },
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Open React Editor in browser
                        let open_btn = egui::Button::new(
                            egui::RichText::new("  🌐 Web Editor  ").color(color_text_light),
                        )
                        .fill(egui::Color32::from_rgb(40, 40, 55))
                        .rounding(egui::Rounding::same(16.0));
                        if ui.add(open_btn).clicked() {
                            let url = "http://127.0.0.1:3000/editor";
                            if let Err(e) = open::that(url) {
                                tracing::warn!("[GUI] Failed to open browser: {}", e);
                            }
                        }

                        ui.add_space(8.0);

                        // Export Button
                        let export_btn = egui::Button::new(
                            egui::RichText::new("  🎬 Export  ")
                                .color(egui::Color32::BLACK)
                                .strong(),
                        )
                        .fill(color_gold)
                        .rounding(egui::Rounding::same(16.0));

                        if ui.add(export_btn).clicked() {
                            println!("[GUI] Export clicked! Starting production pipeline...");
                            let core = self.core.clone();
                            let input = std::path::PathBuf::from(&_state.input_path);
                            let output = if !_state.output_path.is_empty() {
                                std::path::PathBuf::from(&_state.output_path)
                            } else {
                                std::path::PathBuf::from("Video/export.mp4")
                            };
                            let intent = if !_state.intent.trim().is_empty() {
                                Some(_state.intent.clone())
                            } else {
                                None
                            };

                            tokio::spawn(async move {
                                tracing::info!(
                                    "[GUI] Pipeline starting for export to {:?}",
                                    output
                                );
                                let _ = core
                                    .run_unified_pipeline(
                                        &input, &output, "all", "cuda", intent, 1.0,
                                    )
                                    .await;
                            });
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
                    egui::ScrollArea::vertical()
                        .id_salt("editor_nav_scroll")
                        .show(ui, |ui| {
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
                                    save_settings(&self.core.instance_id, _state, self.active_command, &self.tree_state);
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
            });

        // 3. Asset Browser Panel (Media Pool)
        egui::SidePanel::left("editor_asset_browser")
            .resizable(true)
            .default_width(240.0)
            .width_range(150.0..=400.0)
            .frame(
                egui::Frame::none()
                    .fill(color_panel_bg)
                    .inner_margin(egui::Margin::same(12.0))
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("editor_asset_scroll")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                             ui.label(egui::RichText::new("MEDIA POOL").color(color_text_light).strong().size(11.0));
                             ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                 ui.label(egui::RichText::new("Local").color(color_text_dim).small());
                             });
                        });
                        ui.add_space(12.0);

                        // Upload Button
                        if ui.add_sized(
                            [ui.available_width(), 32.0],
                            egui::Button::new(egui::RichText::new("+ Import Media").color(color_gold))
                                .fill(egui::Color32::from_rgb(30, 26, 17))
                                .rounding(egui::Rounding::same(4.0))
                        ).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Video", &["mp4", "mkv", "avi", "mov"])
                                .pick_file() {
                                _state.input_path = path.to_string_lossy().to_string();
                            }
                        }

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);

                        // Active Item
                        if !_state.input_path.is_empty() {
                            let name = std::path::Path::new(&_state.input_path).file_name().unwrap_or_default().to_string_lossy();
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("🎞");
                                    ui.label(egui::RichText::new(name).size(11.0).strong());
                                });
                            });
                        } else {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(egui::RichText::new("No media imported").color(color_text_dim).small());
                            });
                        }
                    });
            });

        // 4. Right Inspector Panel
        egui::SidePanel::right("editor_inspector")
            .resizable(true)
            .default_width(300.0)
            .width_range(200.0..=500.0)
            .frame(egui::Frame::none().fill(color_panel_bg).inner_margin(egui::Margin::same(16.0)))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("INSPECTOR").color(color_text_light).strong().size(11.0));
                ui.add_space(16.0);

                egui::ScrollArea::vertical()
                    .id_salt("inspector_scroll")
                    .show(ui, |ui| {
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("AI Magic Configuration").color(color_gold).strong());
                            ui.add_space(8.0);
                            
                            ui.checkbox(&mut _state.enable_subtitles, "Generate Subtitles");
                            ui.checkbox(&mut _state.enable_censoring, "Censor Profanity");
                            ui.checkbox(&mut _state.enable_audio_enhancement, "Enhance Audio");
                            ui.checkbox(&mut _state.enable_silence_removal, "Remove Silence");
                        });

                        ui.add_space(16.0);
                        ui.label("Directorship Intent:");
                        ui.add(egui::TextEdit::multiline(&mut _state.intent)
                            .hint_text("e.g. Cut this into a snappy highlight reel...")
                            .desired_rows(6)
                            .desired_width(ui.available_width()));

                        ui.add_space(12.0);
                        let disabled = _state.input_path.is_empty() || _state.intent.trim().is_empty() || _state.ai_edit_running;
                        let btn_label = if _state.ai_edit_running { "⏳ Processing..." } else { "🪄 Apply AI Magic" };
                        
                        if ui.add_sized([ui.available_width(), 40.0],
                            egui::Button::new(egui::RichText::new(btn_label).strong().color(egui::Color32::BLACK))
                            .fill(if disabled { egui::Color32::from_rgb(100, 100, 100) } else { color_gold })
                        ).clicked() && !disabled {
                            // Logic remains the same, just moved panel location
                            _state.ai_edit_running = true;
                            let core = self.core.clone();
                            let input = _state.input_path.clone();
                            let intent = _state.intent.clone();
                            let enable_subtitles = _state.enable_subtitles;
                            let ui_ptr = self.ui_state.clone();
                            tokio::spawn(async move {
                                let _ = core.process_youtube_intent(&input, &intent, None, None, false, 0, enable_subtitles).await;
                                if let Ok(mut s) = ui_ptr.lock() { s.ai_edit_running = false; }
                            });
                        }
                    });
            });

        // 5. Central Viewer
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(color_bg_darkest))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                let center = rect.center();
                
                // Draw a nice dark background for the viewer
                ui.painter().rect_filled(rect, 0.0, color_bg_darkest);
                
                if let Some(texture) = &self.preview_texture {
                    let tex_size = texture.size_vec2();
                    let ratio = tex_size.x / tex_size.y;
                    
                    let mut display_size = rect.size();
                    if display_size.x / display_size.y > ratio {
                        display_size.x = display_size.y * ratio;
                    } else {
                        display_size.y = display_size.x / ratio;
                    }
                    
                    let display_rect = egui::Rect::from_center_size(center, display_size * 0.95);
                    ui.painter().image(texture.id(), display_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                } else {
                    ui.painter().text(center, egui::Align2::CENTER_CENTER, "NO MEDIA LOADED", egui::FontId::proportional(14.0), color_text_dim);
                }
            });

        // 4. Bottom Timeline
        egui::TopBottomPanel::bottom("editor_timeline")
            .resizable(true)
            .default_height(280.0)
            .height_range(100.0..=600.0)
            .frame(egui::Frame::none().fill(color_panel_bg).inner_margin(egui::Margin::same(16.0)))
            .show(ctx, |ui| {
                 ui.horizontal(|ui| {
                     // Left tools
                     if ui.add(egui::Button::new(egui::RichText::new("⎌").size(16.0).color(color_text_dim)).fill(egui::Color32::TRANSPARENT)).clicked() {
                         if _state.intent_history_index > 0 {
                             _state.intent_history_index -= 1;
                             _state.intent = _state.intent_history[_state.intent_history_index].clone();
                         }
                     }
                     if ui.add(egui::Button::new(egui::RichText::new("⎍").size(16.0).color(color_text_dim)).fill(egui::Color32::TRANSPARENT)).clicked() {
                         if _state.intent_history_index + 1 < _state.intent_history.len() {
                             _state.intent_history_index += 1;
                             _state.intent = _state.intent_history[_state.intent_history_index].clone();
                         }
                     }
                     ui.add_space(10.0);
                     ui.separator();
                     ui.add_space(10.0);

                     if ui.add(egui::Button::new(egui::RichText::new("✂").size(16.0).color(color_text_dim)).fill(egui::Color32::TRANSPARENT)).clicked() {
                         let core = self.core.clone();
                         let input = std::path::PathBuf::from(&_state.input_path);
                         let start = _state.video_position;
                         tokio::spawn(async move {
                             tracing::info!("[GUI] Cutting 5 seconds at {}", start);
                             let _ = core.clip_video(&input, start, 5.0, Some(std::path::PathBuf::from("Video/cut_temp.mp4"))).await;
                         });
                     }
                     if ui.add(egui::Button::new(egui::RichText::new("🗑").size(16.0).color(color_text_dim)).fill(egui::Color32::TRANSPARENT)).clicked() {
                         _state.input_path.clear();
                         _state.intent.clear();
                     }

                     // Center Playback
                     ui.with_layout(egui::Layout::left_to_right(egui::Align::Center).with_cross_align(egui::Align::Center), |ui| {
                          ui.add_space(ui.available_width() / 2.0 - 150.0); // Rough center
                          if ui.add(egui::Button::new("⏮").fill(egui::Color32::TRANSPARENT)).clicked() {
                              let was_playing = _state.video_player.as_ref().map_or(false, |p| p.playing);
                              if let Some(player) = &mut _state.video_player {
                                  player.stop();
                                  _state.video_player = None;
                              }
                              _state.video_position = 0.0;
                              if was_playing && !_state.input_path.is_empty() {
                                  if let Ok(player) = crate::agent::video_player::VideoPlayer::new(&_state.input_path, _state.video_position) {
                                      _state.video_player = Some(player);
                                  }
                              }
                          }

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
                          if ui.add(egui::Button::new("⏭").fill(egui::Color32::TRANSPARENT)).clicked() {
                              let was_playing = _state.video_player.as_ref().map_or(false, |p| p.playing);
                              if let Some(player) = &mut _state.video_player {
                                  player.stop();
                                  _state.video_player = None;
                              }
                              _state.video_position = _state.video_duration;
                              if was_playing && !_state.input_path.is_empty() {
                                  if let Ok(player) = crate::agent::video_player::VideoPlayer::new(&_state.input_path, _state.video_position) {
                                      _state.video_player = Some(player);
                                  }
                              }
                          }

                         ui.add_space(16.0);
                         let pos_text = format_time(_state.video_position);
                         let dur_text = format_time(_state.video_duration);
                         ui.label(egui::RichText::new(format!("{} / {}", pos_text, dur_text)).color(color_text_light));
                     });

                     // Right tools
                     ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                         ui.label("🔍 +");
                         ui.add(egui::Slider::new(&mut _state.timeline_zoom, 0.1..=2.0).show_value(false));
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
                        let total_width = (_state.video_duration.max(60.0) as f32) * 10.0 * _state.timeline_zoom; // 10px per second rescaled
                        let ruler_rect = egui::Rect::from_min_size(egui::pos2(ui.cursor().min.x, start_y), egui::vec2(total_width, 20.0));
                        p.rect_filled(ruler_rect, 0.0, color_panel_bg);

                        let steps = (_state.video_duration / 10.0) as i32 + 1;
                        for i in 0..steps.max(20) {
                            let x = ui.cursor().min.x + (i as f32) * 100.0 * _state.timeline_zoom; // 100px per 10s scaled
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
            .frame(
                egui::Frame::none()
                    .fill(color_bg_darkest)
                    .inner_margin(egui::Margin::same(32.0)),
            )
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
                    let mut video_rect = ui.available_rect_before_wrap();

                    if let Some(texture) = &self.preview_texture {
                        let tex_size = texture.size_vec2();
                        let aspect = tex_size.x / tex_size.y;
                        let mut new_size = video_rect.size();
                        if new_size.x / new_size.y > aspect {
                            new_size.x = new_size.y * aspect;
                        } else {
                            new_size.y = new_size.x / aspect;
                        }
                        let center = video_rect.center();
                        video_rect = egui::Rect::from_center_size(center, new_size);
                    }

                    // Handle click interaction to play/pause
                    let response = ui.allocate_rect(video_rect, egui::Sense::click());
                    if response.clicked() && !_state.input_path.is_empty() {
                        let is_playing = _state.video_player.as_ref().map_or(false, |p| p.playing);
                        if is_playing {
                            if let Some(player) = &mut _state.video_player {
                                player.stop();
                            }
                            _state.video_player = None;
                        } else {
                            if let Ok(player) = crate::agent::video_player::VideoPlayer::new(
                                &_state.input_path,
                                _state.video_position,
                            ) {
                                _state.video_player = Some(player);
                            }
                        }
                    }
                    if response.hovered() && !_state.input_path.is_empty() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    ui.painter()
                        .rect_filled(video_rect, 12.0, egui::Color32::from_rgb(0, 0, 0)); // Pure black

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
                        ui.painter().circle_filled(
                            center,
                            40.0,
                            egui::Color32::from_white_alpha(30),
                        );
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
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        tracing::info!("[GUI] 🛑 Graceful shutdown initiated...");

        // Save UI state and settings
        if let Ok(state) = self.ui_state.lock() {
            save_settings(
                &self.core.instance_id,
                &state,
                self.active_command,
                &self.tree_state,
            );
            tracing::info!("[GUI] ✅ Settings saved successfully.");
        } else {
            tracing::warn!("[GUI] ⚠️ Failed to lock UI state for saving settings.");
        }

        // Note: Heavy cleanup (waiting for video jobs, stopping background tasks)
        // is handled in main.rs after GUI closes to avoid blocking the UI thread.
        tracing::info!("[GUI] 🔄 Shutdown sequence complete. Returning to main cleanup...");
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.configure_style(ctx);
        Self::render_crt_overlay(ctx);

        // --- BACKGROUND LOGIC ---
        {
            let mut state = self.ui_state.lock().unwrap();

            // 1. Texture conversion
            if let Some(color_image) = state.preview_image.take() {
                self.preview_texture =
                    Some(ctx.load_texture("preview_frame", color_image, Default::default()));
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
                    if let Ok(duration) =
                        crate::agent::source_tools::get_video_duration(&path).await
                    {
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
                                        let color_img = egui::ColorImage::from_rgba_unmultiplied(
                                            size,
                                            buffer.as_raw(),
                                        );

                                        if let Ok(mut s) = ui_ptr.lock() {
                                            s.preview_image = Some(color_img);
                                            ctx_clone.request_repaint();
                                        }
                                    }
                                    Err(e) => tracing::error!(
                                        "[GUI] Failed to decode preview frame: {}",
                                        e
                                    ),
                                }
                            }
                        }
                        Err(e) => tracing::error!("[GUI] get_video_frame failed: {}", e),
                    }
                });
            }

            // 3. Video player frame update
            let mut new_texture_pixels: Option<(Vec<u8>, [usize; 2])> = None;
            let mut new_position: Option<f64> = None;
            let mut player_is_playing = false;

            // Snapshot immutable fields before mutably borrowing video_player
            let cur_pos = state.video_position;
            let max_dur = state.video_duration;

            if let Some(player) = &mut state.video_player {
                let size = [player.width, player.height];
                let fps = player.fps;
                player_is_playing = player.playing;
                if let Some((is_new, frame)) = player.get_next_frame() {
                    if is_new {
                        new_texture_pixels = Some((frame.clone(), [size[0], size[1]]));
                        let new_pos = cur_pos + 1.0 / fps;
                        new_position = Some(new_pos.min(max_dur));
                    }
                    player_is_playing = player.playing;
                }
            }

            if let Some((pixels, size)) = new_texture_pixels {
                let color_image = egui::ColorImage::from_rgb([size[0], size[1]], &pixels);
                // Safety check: Wrap in a block to ensure we don't hold lock if something fails
                self.preview_texture = Some(ctx.load_texture("video_frame", color_image, Default::default()));
            }
            if let Some(pos) = new_position {
                state.video_position = pos;
            }
            if player_is_playing {
                ctx.request_repaint();
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
                egui::ScrollArea::vertical()
                    .id_salt("command_tree_scroll")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("SYNOID_OS")
                                    .size(16.0)
                                    .color(COLOR_ACCENT_ORANGE)
                                    .strong(),
                            );
                        });
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("// COMMAND CENTER")
                                .size(10.0)
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
                                ui.label(
                                    egui::RichText::new("🐝 Hive Mind")
                                        .color(COLOR_ACCENT_ORANGE)
                                        .strong(),
                                );
                                ui.add(egui::Label::new(egui::RichText::new(hive_status).size(10.0)).selectable(true));
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
                                ("🎵", "Combine", ActiveCommand::Combine),
                                ("📺", "YouTube", ActiveCommand::Youtube),
                                ("🎬", "Editor", ActiveCommand::Editor),
                                ("🔍", "Global Discovery", ActiveCommand::Discovery),
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
                                ("⚡", "Process Pipeline", ActiveCommand::Process),
                                ("🧬", "AutoImprove", ActiveCommand::AutoImprove),
                                ("🤖", "Gemma 4", ActiveCommand::Gemma4),
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
                            vec![
                                ("👁️", "Defense", ActiveCommand::Guard),
                                ("🖥️", "GPU Status", ActiveCommand::GpuStatus),
                            ],
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
                });
        }

        // Bottom Status Bar — CRT terminal footer
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(28.0)
            .frame(
                egui::Frame::none()
                    .fill(COLOR_BG_DARK)
                    .stroke(egui::Stroke::new(1.0, COLOR_TEXT_SECONDARY))
                    .inner_margin(egui::Margin::symmetric(12.0, 6.0)),
            )
            .show(ctx, |ui| {
                let status = self.core.get_status();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("TERMINAL_ID: 0x88F2  |  {}", status))
                            .size(10.0)
                            .color(COLOR_ACCENT_BLUE),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("v2.0.0  |  RTX_5080_NVENC  |  SENTINEL: SECURE")
                                .size(10.0)
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
                egui::ScrollArea::vertical()
                    .id_salt("central_panel_scroll")
                    .show(ui, |ui| {
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
                                    .max_height(f32::INFINITY)
                                    .id_salt("log_scroll")
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        for log in &logs {
                                            ui.add(egui::Label::new(
                                                egui::RichText::new(log)
                                                    .monospace()
                                                    .size(11.0)
                                                    .color(COLOR_TEXT_SECONDARY),
                                            ).selectable(true));
                                        }
                                    });
                            },
                        );
                    });
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
                egui::ScrollArea::vertical()
                    .id_salt("preview_panel_scroll")
                    .show(ui, |ui| {
                        let mut state = self.ui_state.lock().unwrap();
                        self.render_preview_panel(ui, &mut state);
                    });
                });
        }

        // Only request repaint if a video is playing or an AI job is running
        let repainting = {
            let state = self.ui_state.lock().unwrap();
            let video_playing = state.video_player.as_ref().map_or(false, |p| p.playing);
            video_playing || state.ai_edit_running || state.is_scanning || state.is_transcribing
        };

        if repainting {
            ctx.request_repaint();
        }
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
        tracing::info!(
            "[GUI] WSL detected → forced X11 backend (DISPLAY={:?})",
            std::env::var("DISPLAY").ok()
        );
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("SYNOID Command Center")
            .with_decorations(true),
        renderer: if is_wsl() {
            eframe::Renderer::Glow
        } else {
            eframe::Renderer::Wgpu
        },
        ..Default::default()
    };

    eframe::run_native(
        "SYNOID Command Center",
        options,
        Box::new(|_cc| Ok(Box::new(SynoidApp::new(core)))),
    )
}
