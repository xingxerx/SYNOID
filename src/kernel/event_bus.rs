// SYNOID™ Kernel Event Bus
// Async message passing between modules

use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    // Project Events
    ProjectLoaded(String),
    ProjectSaved(String),

    // Engine Events
    VideoProcessingStarted(String),
    VideoProcessingCompleted(String),
    VideoProcessingFailed(String, String),

    // AI Events
    IntentAnalyzed(String),
    ReasoningEffortChanged(String),

    // UI Events
    GuiAction(String),
}

pub struct EventBus {
    tx: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: SystemEvent) -> Result<usize, broadcast::error::SendError<SystemEvent>> {
        self.tx.send(event)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
