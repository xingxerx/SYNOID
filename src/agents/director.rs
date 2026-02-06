use crate::ai::intent::reasoning::{ReasoningManager, ReasoningEffort};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneOutline {
    pub timestamp_start: f64,
    pub timestamp_end: f64,
    pub narrative_goal: String,
    pub visual_constraints: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoryPlan {
    pub global_intent: String,
    pub scenes: Vec<SceneOutline>,
}

impl StoryPlan {
    pub fn expected_duration(&self) -> f64 {
        self.scenes.iter().map(|s| s.timestamp_end - s.timestamp_start).sum()
    }
}

pub struct DirectorAgent {
    pub model_id: String,
    pub system_prompt: String,
    pub reasoning: ReasoningManager,
}

impl DirectorAgent {
    pub fn new(model: &str) -> Self {
        Self {
            model_id: model.to_string(),
            system_prompt: "You are the SYNOID Director. Decompose instructions into sub-goals.".into(),
            reasoning: ReasoningManager::new(),
        }
    }

    pub async fn analyze_intent(
        &mut self,
        user_prompt: &str,
        style_profile: Option<&str>
    ) -> Result<StoryPlan, Box<dyn std::error::Error>> {

        if let Some(style) = style_profile {
            if style.to_lowercase().contains("cinematic") {
                self.reasoning.set_effort(ReasoningEffort::High);
            } else if style.to_lowercase().contains("action") {
                 self.reasoning.set_effort(ReasoningEffort::Medium);
            } else {
                 self.reasoning.set_effort(ReasoningEffort::Low);
            }
        }

        info!("[DIRECTOR] Analyzing intent: '{}' (Effort: {})", user_prompt, self.reasoning.get_config_param());

        // Mock LLM Response
        let plan = StoryPlan {
            global_intent: user_prompt.to_string(),
            scenes: vec![
                SceneOutline {
                    timestamp_start: 0.0,
                    timestamp_end: 5.0,
                    narrative_goal: "Intro".to_string(),
                    visual_constraints: vec!["Wide Shot".to_string()],
                },
                SceneOutline {
                    timestamp_start: 5.0,
                    timestamp_end: 15.0,
                    narrative_goal: "Action".to_string(),
                    visual_constraints: vec!["Fast Cuts".to_string()],
                },
            ],
        };
        Ok(plan)
    }
}
