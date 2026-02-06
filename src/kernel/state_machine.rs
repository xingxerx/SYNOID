// SYNOID™ Kernel State Machine
// Project state management

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppState {
    Idle,
    Loading,
    Processing,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub name: String,
    pub path: Option<String>,
    pub dirty: bool,
    // Placeholder for actual project data structure (e.g., Timeline)
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            name: "Untitled Project".to_string(),
            path: None,
            dirty: false,
        }
    }
}

pub struct StateMachine {
    pub state: Arc<Mutex<AppState>>,
    pub project: Arc<Mutex<ProjectState>>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AppState::Idle)),
            project: Arc::new(Mutex::new(ProjectState::default())),
        }
    }

    pub fn set_state(&self, new_state: AppState) {
        let mut state = self.state.lock().unwrap();
        *state = new_state;
    }

    pub fn get_state(&self) -> AppState {
        let state = self.state.lock().unwrap();
        state.clone()
    }

    pub fn load_project(&self, name: String) {
        let mut project = self.project.lock().unwrap();
        project.name = name;
        project.dirty = false;
        self.set_state(AppState::Idle);
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}
