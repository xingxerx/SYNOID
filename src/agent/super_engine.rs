// SYNOID Super Engine
// The Unified "Brain" and "Body" Orchestrator
// Integrates: Vision, SmartEditor, and GPT-OSS

use crate::agent::brain::{Brain, Intent};
use crate::agent::gpt_oss_bridge::SynoidAgent;
use crate::agent::multi_agent::DirectorAgent;


use std::path::Path;
use tracing::{info, warn};

/// The Super Engine is the high-level controller for all Synoid capabilities.
/// It uses the Brain for intent classification and GPT-OSS for complex reasoning,
/// then delegates tasks to specialized engines (SmartEditor, etc.)
/// via a Mixture-of-Experts (MoE) dispatch pattern.
pub struct SuperEngine {
    brain: Brain,
    gpt_brain: Option<SynoidAgent>,
    api_url: String,
}

impl SuperEngine {
    /// Initialize the Super Engine with all sub-systems
    pub fn new(api_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!("[SUPER_ENGINE] Initializing Synoid Unified Systems...");

        // Brain utilizes LLM
        let brain = Brain::new(api_url, "llama3:latest");
        let gpt_brain = Some(SynoidAgent::new(api_url, "llama3:latest"));


        info!("[SUPER_ENGINE] Systems Online.");
        Ok(Self {
            brain,
            gpt_brain,
            api_url: api_url.to_string(),
        })
    }

    /// Primary entry point for any user command.
    /// Flow: NLP Input -> Brain Classification -> MoE Dispatch
    pub async fn process_command(&mut self, command: &str) -> Result<String, String> {
        info!("[SUPER_ENGINE] Processing: \"{}\"", command);

        // 1. Brain: Classify the NLP command
        let intent = self.brain.fast_classify(command);

        match intent {
            // MoE Dispatcher: Complex creative requests
            Intent::Orchestrate { goal, input_path } => {
                self.orchestrate(&goal, input_path.as_deref()).await
            }
            Intent::Unknown { request } => {
                // Fallback: GPT-OSS Reasoning for truly unknown requests
                info!("[SUPER_ENGINE] Unknown request. Consulting GPT-OSS...");
                if let Some(gpt) = &self.gpt_brain {
                    match gpt.reason(&request).await {
                        Ok(response) => Ok(format!("GPT-OSS Analysis: {}", response)),
                        Err(e) => Err(format!("GPT-OSS failed: {}", e)),
                    }
                } else {
                    Err("GPT-OSS Brain not available".to_string())
                }
            }
            // Direct Execution Paths for simple intents
            known_intent => self.execute_intent(known_intent).await,
        }
    }

    /// Mixture-of-Experts Orchestration
    /// 1. DirectorAgent (Brain/LLM) creates a StoryPlan from the NLP goal
    /// 2. Dispatcher distributes tasks to the right expert engines
    async fn orchestrate(&self, goal: &str, input_path: Option<&str>) -> Result<String, String> {
        info!("[MoE] 🧠 ORCHESTRATION MODE ACTIVATED");
        info!("[MoE] Goal: \"{}\"", goal);

        // === Phase 1: Director Agent (The Brain) ===
        let mut director = DirectorAgent::new("llama3:latest", &self.api_url);

        info!("[MoE] 📋 Consulting DirectorAgent for plan...");
        let plan = director
            .analyze_intent(goal, None)
            .await
            .map_err(|e| format!("DirectorAgent failed: {}", e))?;

        info!(
            "[MoE] ✅ StoryPlan received: \"{}\" ({} scenes)",
            plan.global_intent,
            plan.scenes.len()
        );
        for (i, scene) in plan.scenes.iter().enumerate() {
            info!(
                "[MoE]   Scene {}: {} ({:.1}s - {:.1}s) [{}]",
                i + 1,
                scene.narrative_goal,
                scene.timestamp_start,
                scene.timestamp_end,
                scene.visual_constraints.join(", ")
            );
        }

        // === Phase 2: Expert Dispatch ===
        let mut results: Vec<String> = Vec::new();

        // Expert 1: SmartEditor (Video Cutting & Assembly)
        if let Some(video_path) = input_path {
            let input = Path::new(video_path);
            if input.exists() {
                info!("[MoE] 🎬 Dispatching to SmartEditor expert...");
                let output = input.with_file_name(format!(
                    "{}_orchestrated.mp4",
                    input.file_stem().unwrap_or_default().to_string_lossy()
                ));

                match crate::agent::smart_editor::smart_edit(
                    input,
                    goal, // Pass the full NLP goal as the creative intent
                    &output,
                    false, // unused param placeholder
                    Some(Box::new(|msg: &str| {
                        info!("[MoE/SmartEditor] {}", msg);
                    })),
                    None,
                    None,
                    None,
                )
                .await
                {
                    Ok(result) => {
                        results.push(format!("🎬 SmartEditor: {}", result));
                        info!("[MoE] ✅ SmartEditor completed: {}", result);
                    }
                    Err(e) => {
                        warn!("[MoE] ⚠️ SmartEditor failed: {}", e);
                        results.push(format!("⚠️ SmartEditor failed: {}", e));
                    }
                }
            } else {
                results.push(format!("⚠️ Input file not found: {}", video_path));
            }
        } else {
            info!("[MoE] ℹ️ No input video path provided. Skipping SmartEditor.");
            results.push("ℹ️ No input video provided for editing.".to_string());
        }





        // === Phase 3: Summary ===
        let summary = format!(
            "🧠 MoE Orchestration Complete\n   Goal: \"{}\"\n   Plan: {} scenes\n   Experts dispatched: {}\n   Results:\n   {}",
            plan.global_intent,
            plan.scenes.len(),
            results.len(),
            results.join("\n   ")
        );

        info!("[MoE] {}", summary);
        Ok(summary)
    }

    async fn execute_intent(&mut self, intent: Intent) -> Result<String, String> {
        match intent {
            Intent::DownloadYoutube { url } => {
                use crate::agent::source_tools;
                let output_dir = Path::new("D:\\SYNOID\\Download");
                if !output_dir.exists() {
                    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
                }

                match source_tools::download_youtube(&url, &output_dir, None).await {
                    Ok(info) => Ok(format!("Downloaded: {} to {:?}", info.title, output_dir)),
                    Err(e) => Err(format!("Download failed: {}", e)),
                }
            }


            Intent::Research { topic } => {
                use crate::agent::source_tools;
                match source_tools::search_youtube(&topic, 3).await {
                    Ok(results) => Ok(format!("Found {} videos about '{}'", results.len(), topic)),
                    Err(e) => Err(e.to_string()),
                }
            }
            Intent::ScanVideo { path } => {
                use crate::agent::vision_tools;
                let p = Path::new(&path);
                match vision_tools::scan_visual(p).await {
                    Ok(scenes) => Ok(format!("Scanned {} scenes in {:?}", scenes.len(), p)),
                    Err(e) => Err(e.to_string()),
                }
            }
            Intent::LearnStyle { input, name } => Ok(format!(
                "Learning style '{}' from {} (SuperEngine implementation pending)",
                name, input
            )),
            Intent::CreateEdit { input, instruction } => Ok(format!(
                "Embodied Edit Initiated: '{}' on {}",
                instruction, input
            )),
            Intent::Orchestrate { .. } => unreachable!("Handled in process_command"),
            Intent::Unknown { .. } => unreachable!("Handled in process_command"),
        }
    }
}
