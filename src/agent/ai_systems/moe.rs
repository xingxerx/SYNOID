// SYNOID Mixture of Experts (MoE) Router — v3
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Enhanced MoE with:
//   - 7 specialized expert roles (added Cinematographer + AudioEngineer)
//   - Confidence-weighted routing: returns scored expert list, not just top-1
//   - Ensemble execution: parallel top-2 experts, response merged when uncertain
//   - Task complexity detection: simple → fast single-expert; complex → ensemble
//   - Hermes Agent delegation: tool-heavy tasks routed to hermes-agent subprocess
//
// Architecture inspired by Mixtral/Switch Transformer sparse MoE gating, adapted
// for LLM-routed text tasks with no gradient descent — pure prompt-based gating.

use crate::agent::ai_systems::gpt_oss_bridge::SynoidAgent;
use crate::agent::engines::process_utils::CommandExt;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::info;

// ──────────────────────────────────────────────────────────────────────────────
// Expert Roles
// ──────────────────────────────────────────────────────────────────────────────

/// SYNOID's specialist expert pool.
///
/// Each expert has a dedicated system prompt that primes its persona.
/// The router selects the best expert(s) per task via LLM-based gating.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExpertRole {
    /// System design, high-level planning, structural integrity.
    Architect,
    /// Code generation, implementation, debugging — Rust/FFmpeg/React.
    Developer,
    /// Data processing, video analysis, pattern recognition, metrics.
    Analyst,
    /// Security audits, safety checks, input validation, cyberdefense.
    Guardian,
    /// Explanations, documentation, educational content.
    Scholar,
    /// Visual aesthetics: color grading, composition, pacing, cinematography.
    Cinematographer,
    /// Audio mixing, sync, normalization, music selection, sound design.
    AudioEngineer,
}

impl std::fmt::Display for ExpertRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Architect => write!(f, "Architect"),
            Self::Developer => write!(f, "Developer"),
            Self::Analyst => write!(f, "Analyst"),
            Self::Guardian => write!(f, "Guardian"),
            Self::Scholar => write!(f, "Scholar"),
            Self::Cinematographer => write!(f, "Cinematographer"),
            Self::AudioEngineer => write!(f, "AudioEngineer"),
        }
    }
}

impl ExpertRole {
    /// System prompt for this expert's persona.
    pub fn system_prompt(&self) -> &'static str {
        match self {
            Self::Architect =>
                "You are the SYNOID Architect. Design robust, scalable solutions for video \
                 production workflows. Focus on system structure, data flow, and long-term \
                 maintainability. Output concise architectural decisions.",

            Self::Developer =>
                "You are the SYNOID Developer. Write clean, efficient Rust/FFmpeg/React code. \
                 Focus on correctness, performance, and minimal dependencies. Prefer idiomatic \
                 Rust. Output working code snippets.",

            Self::Analyst =>
                "You are the SYNOID Analyst. Analyze video style data, performance metrics, \
                 and pattern distributions. Identify trends, anomalies, and optimization \
                 opportunities. Output structured JSON analysis.",

            Self::Guardian =>
                "You are the SYNOID Guardian. Perform security audits, validate all inputs, \
                 and enforce safety constraints. Flag any command injection, path traversal, \
                 or unsafe operations. Output a risk assessment with severity levels.",

            Self::Scholar =>
                "You are the SYNOID Scholar. Explain complex concepts clearly and concisely. \
                 Maintain the project knowledge base. Prioritize accuracy over brevity. \
                 Output well-structured explanations with examples.",

            Self::Cinematographer =>
                "You are the SYNOID Cinematographer. You have deep expertise in visual \
                 storytelling: shot composition, color grading (LUTs, curves, saturation), \
                 camera movement, pacing, and cinematic aesthetics. Analyze and prescribe \
                 specific FFmpeg filter chains and edit rhythm decisions. Output concrete \
                 filter parameter recommendations.",

            Self::AudioEngineer =>
                "You are the SYNOID AudioEngineer. Expert in audio post-production: loudness \
                 normalization (LUFS targets), dynamic range, music sync, sound design, and \
                 FFmpeg audio filter chains (loudnorm, compand, equalizer). Output specific \
                 FFmpeg audio filter strings and timing cues.",
        }
    }

    /// One-line description for routing prompts.
    fn routing_label(&self) -> &'static str {
        match self {
            Self::Architect => "ARCHITECT: system design, architecture, planning",
            Self::Developer => "DEVELOPER: code, implementation, debugging, Rust, FFmpeg",
            Self::Analyst => "ANALYST: data analysis, video metrics, pattern recognition",
            Self::Guardian => "GUARDIAN: security, validation, safety checks",
            Self::Scholar => "SCHOLAR: explanation, documentation, general knowledge",
            Self::Cinematographer => "CINEMATOGRAPHER: color grading, visuals, shot pacing, LUTs",
            Self::AudioEngineer => "AUDIO_ENGINEER: audio mixing, loudness, music sync, sound design",
        }
    }

    /// All available roles (for routing prompt generation).
    pub fn all() -> &'static [ExpertRole] {
        &[
            Self::Architect,
            Self::Developer,
            Self::Analyst,
            Self::Guardian,
            Self::Scholar,
            Self::Cinematographer,
            Self::AudioEngineer,
        ]
    }

    /// Parse role from an uppercase string token.
    fn from_token(s: &str) -> Option<Self> {
        let s = s.to_uppercase();
        if s.contains("ARCHITECT") { Some(Self::Architect) }
        else if s.contains("DEVELOPER") || s.contains("DEV") { Some(Self::Developer) }
        else if s.contains("ANALYST") { Some(Self::Analyst) }
        else if s.contains("GUARDIAN") { Some(Self::Guardian) }
        else if s.contains("SCHOLAR") { Some(Self::Scholar) }
        else if s.contains("CINEMATOGRAPHER") || s.contains("CINEMA") { Some(Self::Cinematographer) }
        else if s.contains("AUDIO") || s.contains("ENGINEER") { Some(Self::AudioEngineer) }
        else { None }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Routing Result
// ──────────────────────────────────────────────────────────────────────────────

/// Routing decision with confidence scores for all experts.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Primary expert (highest confidence).
    pub primary: ExpertRole,
    /// Secondary expert (for ensemble when primary confidence < threshold).
    pub secondary: Option<ExpertRole>,
    /// Confidence in primary expert (0.0–1.0).
    pub confidence: f32,
    /// Whether the router recommends ensemble execution.
    pub use_ensemble: bool,
}

// ──────────────────────────────────────────────────────────────────────────────
// MoE Router
// ──────────────────────────────────────────────────────────────────────────────

pub struct MoeRouter {
    agent: SynoidAgent,
}

impl MoeRouter {
    pub fn new(agent: SynoidAgent) -> Self {
        Self { agent }
    }

    // ─── Simple routing (backwards-compatible) ────────────────────────────────

    /// Route a task to the single best expert (fast path, no confidence scoring).
    pub async fn route(&self, task: &str) -> ExpertRole {
        self.route_with_decision(task).await.primary
    }

    // ─── Confidence-scored routing ────────────────────────────────────────────

    /// Route with confidence scoring and ensemble recommendation.
    ///
    /// Asks the LLM to rank the top-2 experts and estimate confidence.
    /// If confidence < 0.65, enables ensemble mode (both experts respond).
    pub async fn route_with_decision(&self, task: &str) -> RoutingDecision {
        let labels: Vec<String> = ExpertRole::all()
            .iter()
            .map(|r| format!("  - {}", r.routing_label()))
            .collect();

        let route_prompt = format!(
            "You are a task routing system for SYNOID, an AI video production agent.\n\
             Select the TWO best expert roles for this task, in order of relevance.\n\n\
             Available experts:\n{}\n\n\
             Task: \"{}\"\n\n\
             Respond with EXACTLY this format (no other text):\n\
             PRIMARY: <ROLE>\n\
             SECONDARY: <ROLE>\n\
             CONFIDENCE: <0.0-1.0>",
            labels.join("\n"),
            task
        );

        match self.agent.fast_reason(&route_prompt).await {
            Ok(resp) => self.parse_routing_decision(&resp),
            Err(_) => RoutingDecision {
                primary: ExpertRole::Scholar,
                secondary: None,
                confidence: 0.5,
                use_ensemble: false,
            },
        }
    }

    fn parse_routing_decision(&self, resp: &str) -> RoutingDecision {
        let mut primary = ExpertRole::Scholar;
        let mut secondary: Option<ExpertRole> = None;
        let mut confidence = 0.7f32;

        for line in resp.lines() {
            let line = line.trim();
            if line.starts_with("PRIMARY:") {
                if let Some(role) = ExpertRole::from_token(&line["PRIMARY:".len()..]) {
                    primary = role;
                }
            } else if line.starts_with("SECONDARY:") {
                secondary = ExpertRole::from_token(&line["SECONDARY:".len()..]);
            } else if line.starts_with("CONFIDENCE:") {
                let val = line["CONFIDENCE:".len()..].trim();
                if let Ok(c) = val.parse::<f32>() {
                    confidence = c.clamp(0.0, 1.0);
                }
            }
        }

        // Remove secondary if it's the same as primary
        if secondary == Some(primary) {
            secondary = None;
        }

        let use_ensemble = confidence < 0.65 && secondary.is_some();

        RoutingDecision {
            primary,
            secondary,
            confidence,
            use_ensemble,
        }
    }

    // ─── Single Expert Execution ──────────────────────────────────────────────

    /// Execute a task using a specific expert persona (single expert).
    pub async fn execute(&self, role: ExpertRole, task: &str) -> Result<String, String> {
        info!("[MOE] {} ← {}", role, &task[..task.len().min(80)]);
        let prompt = format!("{}\n\nTask: {}", role.system_prompt(), task);
        self.agent.reason(&prompt).await
    }

    // ─── Ensemble Execution ───────────────────────────────────────────────────

    /// Execute a task with automatic routing + ensemble if confidence is low.
    ///
    /// Tool-heavy tasks (download, browse, run command, etc.) are first attempted
    /// via the Hermes Agent subprocess (90+ tools). All other tasks route through
    /// the MoE expert pool, with ensemble when confidence < 0.65.
    pub async fn smart_execute(&self, task: &str) -> Result<String, String> {
        // Try Hermes first for tasks that need real tool execution
        if Self::is_tool_task(task) {
            info!("[MOE] Tool task detected — delegating to Hermes Agent");
            match Self::delegate_to_hermes(task).await {
                Ok(resp) => return Ok(resp),
                Err(e) => info!("[MOE] Hermes unavailable ({}), falling back to expert routing", e),
            }
        }

        let decision = self.route_with_decision(task).await;
        info!(
            "[MOE] Route → {} (conf={:.2}, ensemble={})",
            decision.primary, decision.confidence, decision.use_ensemble
        );

        if !decision.use_ensemble {
            return self.execute(decision.primary, task).await;
        }

        let secondary = decision.secondary.unwrap_or(ExpertRole::Scholar);

        // Run both experts concurrently
        let (res_primary, res_secondary) = tokio::join!(
            self.execute(decision.primary, task),
            self.execute(secondary, task)
        );

        let primary_text = res_primary.unwrap_or_else(|e| format!("[{} error: {}]", decision.primary, e));
        let secondary_text = res_secondary.unwrap_or_else(|e| format!("[{} error: {}]", secondary, e));

        // Merge responses
        self.merge_expert_responses(
            task,
            decision.primary,
            &primary_text,
            secondary,
            &secondary_text,
        )
        .await
    }

    /// Synthesize two expert responses into a unified answer.
    async fn merge_expert_responses(
        &self,
        task: &str,
        role_a: ExpertRole,
        resp_a: &str,
        role_b: ExpertRole,
        resp_b: &str,
    ) -> Result<String, String> {
        let merge_prompt = format!(
            "Two SYNOID experts have responded to this task:\n\
             Task: \"{task}\"\n\n\
             [{role_a} response]:\n{resp_a}\n\n\
             [{role_b} response]:\n{resp_b}\n\n\
             Synthesize the best elements from both responses into a single, \
             coherent answer. Preserve all concrete technical details (FFmpeg \
             commands, Rust code, file paths). Remove any redundancy.",
            task = task,
            role_a = role_a,
            resp_a = resp_a,
            role_b = role_b,
            resp_b = resp_b,
        );

        info!("[MOE] Merging {} + {} responses", role_a, role_b);
        self.agent.reason(&merge_prompt).await
    }

    // ─── Batch routing ────────────────────────────────────────────────────────

    /// Route and execute multiple tasks sequentially, returning (task, expert, result) triples.
    pub async fn batch_execute(
        &self,
        tasks: Vec<String>,
    ) -> Vec<(String, ExpertRole, Result<String, String>)> {
        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            let decision = self.route_with_decision(&task).await;
            let result = self.execute(decision.primary, &task).await;
            results.push((task, decision.primary, result));
        }
        results
    }

    // ─── Hermes Agent Delegation ──────────────────────────────────────────────

    /// Returns true if the task requires real tool execution that Hermes handles
    /// better than an LLM persona (file ops, web browsing, terminal commands, etc.).
    fn is_tool_task(task: &str) -> bool {
        let lower = task.to_lowercase();
        lower.contains("download") ||
        lower.contains("search the web") ||
        lower.contains("browse") ||
        lower.contains("run command") ||
        lower.contains("execute") ||
        lower.contains("read file") ||
        lower.contains("write file") ||
        lower.contains("open url") ||
        lower.contains("terminal")
    }

    /// Delegate a task to the Hermes Agent subprocess using its single-query mode.
    /// Returns Ok(response) on success, Err if Hermes is unavailable or fails.
    pub async fn delegate_to_hermes(task: &str) -> Result<String, String> {
        let hermes_dir = std::env::current_dir()
            .map(|d| d.join(".agent/repos/hermes-agent"))
            .map_err(|e| format!("cwd error: {e}"))?;

        if !hermes_dir.exists() {
            return Err("Hermes agent directory not found".to_string());
        }

        let task_owned = task.to_string();
        let output = tokio::task::spawn_blocking(move || {
            Command::new("python")
                .arg("cli.py")
                .arg("-q")
                .arg(&task_owned)
                .current_dir(&hermes_dir)
                .stealth()
                .output()
        })
        .await
        .map_err(|e| format!("Hermes spawn error: {e}"))?
        .map_err(|e| format!("Hermes IO error: {e}"))?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !text.is_empty() {
                info!("[MOE] Hermes delegate succeeded ({} chars)", text.len());
                return Ok(text);
            }
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Hermes exited {}: {}", output.status, stderr.trim()))
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_roles_have_system_prompts() {
        for role in ExpertRole::all() {
            let prompt = role.system_prompt();
            assert!(!prompt.is_empty(), "{} has empty system prompt", role);
        }
    }

    #[test]
    fn expert_role_display() {
        assert_eq!(ExpertRole::Cinematographer.to_string(), "Cinematographer");
        assert_eq!(ExpertRole::AudioEngineer.to_string(), "AudioEngineer");
    }

    #[test]
    fn parse_routing_decision_valid() {
        let router = MoeRouter {
            agent: SynoidAgent::new("http://localhost:11434", "llama3.2"),
        };
        let resp = "PRIMARY: CINEMATOGRAPHER\nSECONDARY: AUDIO_ENGINEER\nCONFIDENCE: 0.55";
        let decision = router.parse_routing_decision(resp);
        assert_eq!(decision.primary, ExpertRole::Cinematographer);
        assert_eq!(decision.secondary, Some(ExpertRole::AudioEngineer));
        assert!((decision.confidence - 0.55).abs() < 0.01);
        assert!(decision.use_ensemble, "Should use ensemble at confidence 0.55");
    }

    #[test]
    fn parse_routing_decision_high_confidence() {
        let router = MoeRouter {
            agent: SynoidAgent::new("http://localhost:11434", "llama3.2"),
        };
        let resp = "PRIMARY: DEVELOPER\nSECONDARY: ARCHITECT\nCONFIDENCE: 0.92";
        let decision = router.parse_routing_decision(resp);
        assert_eq!(decision.primary, ExpertRole::Developer);
        assert!(!decision.use_ensemble, "Should not use ensemble at confidence 0.92");
    }

    #[test]
    fn all_roles_parseable() {
        for role in ExpertRole::all() {
            let parsed = ExpertRole::from_token(&role.to_string().to_uppercase());
            assert!(parsed.is_some(), "Could not parse role: {}", role);
        }
    }
}
