// SYNOID Super Engine
// The Unified "Brain" and "Body" Orchestrator
// Integrates: Vector Engine, Voice Engine, Vision, SmartEditor, and GPT-OSS

use crate::agent::brain::{Brain, Intent};
use crate::agent::gpt_oss_bridge::SynoidAgent;
use crate::agent::multi_agent::DirectorAgent;
use crate::agent::vector_engine::{upscale_video, vectorize_video, VectorConfig};
use crate::agent::voice::engine::VoiceEngine;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

/// The Super Engine is the high-level controller for all Synoid capabilities.
/// It uses the Brain for intent classification and GPT-OSS for complex reasoning,
/// then delegates tasks to specialized engines (Vector, Voice, SmartEditor, etc.)
/// via a Mixture-of-Experts (MoE) dispatch pattern.
pub struct SuperEngine {
    brain: Brain,
    gpt_brain: Option<SynoidAgent>,
    voice_engine: Arc<VoiceEngine>,
    api_url: String,
    work_dir: PathBuf,
}

impl SuperEngine {
    /// Initialize the Super Engine with all sub-systems
    pub fn new(api_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!("[SUPER_ENGINE] Initializing Synoid Unified Systems...");
        
        // Brain utilizes GPT-OSS 20B
        let brain = Brain::new(api_url, "gpt-oss:20b");
        let gpt_brain = Some(SynoidAgent::new(api_url, "gpt-oss:20b"));
        
        // Initialize Voice Engine (might fail if models missing, but we shouldn't crash)
        let voice_engine = match VoiceEngine::new() {
            Ok(v) => Arc::new(v),
            Err(e) => {
                warn!("[SUPER_ENGINE] Voice Engine failed to init: {}. Voice features disabled.", e);
                return Err(e); 
            }
        };

        let work_dir = std::env::current_dir()?.join("synoid_workspace");
        if !work_dir.exists() {
            std::fs::create_dir_all(&work_dir)?;
        }

        info!("[SUPER_ENGINE] Systems Online.");
        Ok(Self {
            brain,
            gpt_brain,
            voice_engine,
            api_url: api_url.to_string(),
            work_dir,
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
        let mut director = DirectorAgent::new("gpt-oss:20b", &self.api_url);
        
        info!("[MoE] 📋 Consulting DirectorAgent for plan...");
        let plan = director.analyze_intent(goal, None).await
            .map_err(|e| format!("DirectorAgent failed: {}", e))?;
        
        info!("[MoE] ✅ StoryPlan received: \"{}\" ({} scenes)", plan.global_intent, plan.scenes.len());
        for (i, scene) in plan.scenes.iter().enumerate() {
            info!("[MoE]   Scene {}: {} ({:.1}s - {:.1}s) [{}]", 
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
                    goal,  // Pass the full NLP goal as the creative intent
                    &output, 
                    false, // funny_mode
                    Some(Box::new(|msg: &str| {
                        info!("[MoE/SmartEditor] {}", msg);
                    }))
                ).await {
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

        // Expert 2: VoiceEngine (if plan implies narration/voiceover)
        // Expert 2: VoiceEngine (Generate narration/dialogue from script)
        let voice_out_dir = self.work_dir.join("voice_output");
        if !voice_out_dir.exists() {
             let _ = std::fs::create_dir_all(&voice_out_dir);
        }

        let mut voice_tasks_count = 0;
        for (i, scene) in plan.scenes.iter().enumerate() {
            if let Some(script) = &scene.script {
                voice_tasks_count += 1;
                let filename = format!("scene_{}_{}.wav", i, scene.narrative_goal.chars().take(10).collect::<String>().replace(" ", "_"));
                let output_path = voice_out_dir.join(&filename);
                
                info!("[MoE] 🗣️ VoiceEngine generating for Scene {}: \"{}\"", i, script.chars().take(30).collect::<String>());
                
                let res = if let Some(profile) = &scene.voice_profile {
                    self.voice_engine.speak_as(script, profile, &output_path)
                } else {
                    self.voice_engine.speak(script, &output_path)
                };

                match res {
                    Ok(_) => results.push(format!("🗣️ Scene {}: Audio generated at {:?}", i, filename)),
                    Err(e) => {
                        warn!("[MoE] Voice generation failed: {}", e);
                        results.push(format!("⚠️ Voice failed for Scene {}: {}", i, e));
                    }
                }
            }
        }
        
        if voice_tasks_count == 0 {
             results.push("ℹ️ No scripts found in StoryPlan.".to_string());
        }

        // Expert 3: VectorEngine (if plan implies stylization)
        // Expert 3: VectorEngine (Vectorize if requested)
        let needs_vector = plan.scenes.iter().any(|s| {
            s.visual_constraints.iter().any(|c| {
                let c_lower = c.to_lowercase();
                c_lower.contains("vector") || c_lower.contains("svg") || c_lower.contains("styliz")
            })
        });

        if needs_vector {
            if let Some(video_path) = input_path {
                 info!("[MoE] 🎨 Dispatching to VectorEngine expert...");
                 let input = Path::new(video_path);
                 let output_dir = self.work_dir.join("vectors");
                 let config = crate::agent::vector_engine::VectorConfig::default();
                 
                 // Reuse vector_engine::vectorize_video (imported/available)
                 match crate::agent::vector_engine::vectorize_video(input, &output_dir, config).await {
                     Ok(msg) => {
                         results.push(format!("🎨 VectorEngine: {}", msg));
                         info!("[MoE] ✅ VectorEngine completed: {}", msg);
                     }
                     Err(e) => {
                         results.push(format!("⚠️ VectorEngine failed: {}", e));
                         warn!("[MoE] VectorEngine failed: {}", e);
                     }
                 }
            } else {
                 results.push("⚠️ Vectorization requested but no video input provided.".to_string());
            }
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
                 let output_dir = self.work_dir.join("downloads");
                 if !output_dir.exists() { std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?; }
                 
                 match source_tools::download_youtube(&url, &output_dir, None).await {
                     Ok(info) => Ok(format!("Downloaded: {} to {:?}", info.title, output_dir)),
                     Err(e) => Err(format!("Download failed: {}", e)),
                 }
            }
            Intent::Vectorize { input, preset } => {
                let input_path = Path::new(&input);
                let output_dir = self.work_dir.join("vectors");
                
                let config = match preset.as_str() {
                    "detailed" => VectorConfig {
                        filter_speckle: 2,
                        ..Default::default()
                    },
                    _ => VectorConfig::default(),
                };

                match vectorize_video(input_path, &output_dir, config).await {
                    Ok(msg) => Ok(msg),
                    Err(e) => Err(format!("Vectorization failed: {}", e)),
                }
            }
            Intent::Upscale { input, scale } => {
                let input_path = Path::new(&input);
                let output_path = self.work_dir.join(format!("upscaled_{}x.mp4", scale));

                match upscale_video(input_path, scale, &output_path).await {
                    Ok(msg) => Ok(msg),
                    Err(e) => Err(format!("Upscaling failed: {}", e)),
                }
            }
            Intent::VoiceClone { input, name } => {
                let input_path = Path::new(&input);
                match self.voice_engine.create_profile(&name, input_path) {
                    Ok(_) => Ok(format!("Voice profile '{}' created from {:?}", name, input_path)),
                    Err(e) => Err(format!("Voice cloning failed: {}", e)),
                }
            }
            Intent::Speak { text, profile } => {
                let _output_path = self.work_dir.join("speech_output.wav");
                Ok(format!("(Simulated) Spoke: \"{}\" as '{}'", text, profile))
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
            Intent::LearnStyle { input, name } => {
                 Ok(format!("Learning style '{}' from {} (SuperEngine implementation pending)", name, input))
            }
            Intent::CreateEdit { input, instruction } => {
                 Ok(format!("Embodied Edit Initiated: '{}' on {}", instruction, input))
            }
            Intent::Orchestrate { .. } => unreachable!("Handled in process_command"),
            Intent::Unknown { .. } => unreachable!("Handled in process_command"),
        }
    }
}
