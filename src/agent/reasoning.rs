// SYNOID™ Reasoning Manager
// Logic: Switch gpt-oss-20b effort levels based on task priority

use tracing::info;

#[derive(Debug, Clone, PartialEq)]
pub enum ReasoningEffort {
    Low,    // For rapid timeline edits
    Medium, // For style profile application
    High,   // For final cinematic validation
}

pub struct ReasoningManager {
    pub current_effort: ReasoningEffort,
}

impl ReasoningManager {
    pub fn new() -> Self {
        Self {
            current_effort: ReasoningEffort::Low,
        }
    }

    /// Updates the model's reasoning effort for the next agentic task.
    /// High effort increases latency but provides deeper narrative CoT.
    pub fn set_effort(&mut self, effort: ReasoningEffort) {
        if self.current_effort != effort {
            info!("[REASONING] Switching effort level: {:?} -> {:?}", self.current_effort, effort);
            self.current_effort = effort;
            // In a real implementation, this would update the 'reasoning_effort' parameter
            // in the gpt-oss API config payload.
        }
    }

    pub fn get_config_param(&self) -> &str {
        match self.current_effort {
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
        }
    }
}
