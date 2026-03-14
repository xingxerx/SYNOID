// SYNOID Mixture of Experts (MoE) Router
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID

use crate::agent::gpt_oss_bridge::SynoidAgent;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ExpertRole {
    /// Focuses on system design, high-level planning, and structural integrity.
    Architect,
    /// Focuses on code generation, implementation details, and debugging.
    Developer,
    /// Focuses on data processing, video analysis, and pattern recognition.
    Analyst,
    /// Focuses on security audits, safety checks, and cyberdefense.
    Guardian,
    /// Focuses on explanations, documentation, and educational content.
    Scholar,
}

impl std::fmt::Display for ExpertRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Architect => write!(f, "Architect"),
            Self::Developer => write!(f, "Developer"),
            Self::Analyst => write!(f, "Analyst"),
            Self::Guardian => write!(f, "Guardian"),
            Self::Scholar => write!(f, "Scholar"),
        }
    }
}

pub struct MoeRouter {
    agent: SynoidAgent,
}

impl MoeRouter {
    pub fn new(agent: SynoidAgent) -> Self {
        Self { agent }
    }

    /// Route a task to the most appropriate expert.
    pub async fn route(&self, task: &str) -> ExpertRole {
        let route_prompt = format!(
            "Classify the following task into one of these expert roles: ARCHITECT, DEVELOPER, ANALYST, GUARDIAN, SCHOLAR.\n\n\
            - ARCHITECT: System design, high-level planning, architectural structure.\n\
            - DEVELOPER: Writing code, debugging, refactoring, implementation.\n\
            - ANALYST: Data analysis, video pattern recognition, performance metrics.\n\
            - GUARDIAN: Security, safety, validation, cyberdefense.\n\
            - SCHOLAR: Explanations, documentation, teaching, general knowledge.\n\n\
            Task: \"{}\"\n\n\
            Respond ONLY with the name of the role.",
            task
        );

        match self.agent.fast_reason(&route_prompt).await {
            Ok(resp) => {
                let r = resp.to_uppercase();
                if r.contains("ARCHITECT") { ExpertRole::Architect }
                else if r.contains("DEVELOPER") { ExpertRole::Developer }
                else if r.contains("ANALYST") { ExpertRole::Analyst }
                else if r.contains("GUARDIAN") { ExpertRole::Guardian }
                else { ExpertRole::Scholar }
            }
            Err(_) => ExpertRole::Scholar, // Default fallback
        }
    }

    /// Execute a task using a specific expert persona.
    pub async fn execute(&self, role: ExpertRole, task: &str) -> Result<String, String> {
        info!("[MOE] Dispatching task to {}: {}", role, task);

        let system_prompt = match role {
            ExpertRole::Architect => 
                "You are the SYNOID Architect. Focus on system design, high-level planning, and structural integrity. \
                 Design robust, scalable solutions for video production workflows.",
            ExpertRole::Developer => 
                "You are the SYNOID Developer. Focus on code generation, implementation details, and debugging. \
                 Write clean, efficient Rust/React code for the Synoid ecosystem.",
            ExpertRole::Analyst => 
                "You are the SYNOID Analyst. Focus on data processing, video analysis, and pattern recognition. \
                 Analyze video styles and performance metrics to optimize outputs.",
            ExpertRole::Guardian => 
                "You are the SYNOID Guardian. Focus on security audits, safety checks, and cyberdefense. \
                 Ensure all operations comply with safety protocols and protect system integrity.",
            ExpertRole::Scholar => 
                "You are the SYNOID Scholar. Focus on explanations, documentation, and educational content. \
                 Explain complex concepts simply and maintain the project knowledge base.",
        };

        let expert_prompt = format!("{}\n\nTask: {}", system_prompt, task);
        self.agent.reason(&expert_prompt).await
    }
}
