use std::sync::{Arc, Mutex, RwLock};

use serde::Serialize;

use crate::agent::defense::pressure::{PressureLevel, PressureWatcher};
use crate::agent::core::AgentCore;

pub struct KernelState {
    pub task: Mutex<TaskState>,
    pub core: Arc<AgentCore>,

    /// Shared pressure level for the GUI health bar.
    pub pressure_level: Arc<RwLock<PressureLevel>>,
}

impl KernelState {
    pub fn new(core: Arc<AgentCore>) -> Self {
        let watcher = PressureWatcher::new();
        let pressure_handle = watcher.level_handle();

        Self {
            task: Mutex::new(TaskState::default()),
            core,

            pressure_level: pressure_handle,
        }
    }
}

pub struct TaskState {
    pub input_path: String,
    pub output_path: String,
    pub intent: String,
    pub youtube_url: String,
    pub status: String,
    pub is_running: bool,
    pub logs: Vec<String>,
    // Production params
    pub clip_start: String,
    pub clip_duration: String,
    pub compress_size: String,
    pub scale_factor: String,
    pub research_topic: String,
    pub guard_mode: String,
}

impl Default for TaskState {
    fn default() -> Self {
        Self {
            input_path: String::new(),
            output_path: "output.mp4".to_string(),
            intent: String::new(),
            youtube_url: String::new(),
            status: "⚡ System Ready".to_string(),
            is_running: false,
            logs: vec!["[SYSTEM] SYNOID Core initialized.".to_string()],
            clip_start: "0.0".to_string(),
            clip_duration: "10.0".to_string(),
            compress_size: "25.0".to_string(),
            scale_factor: "2.0".to_string(),
            research_topic: String::new(),
            guard_mode: "all".to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct DashboardStatus {
    pub tasks: TasksStatus,
    pub productivity: i32,
}

#[derive(Serialize)]
pub struct TasksStatus {
    pub active: i32,
    pub total: i32,
}

#[derive(Serialize)]
pub struct DashboardTask {
    pub title: String,
    pub category: String,
    pub due: String,
    pub completed: bool,
    pub priority: String,
}
