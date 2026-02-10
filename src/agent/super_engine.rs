// SYNOID Super Engine
// The Unified "Brain" and "Body" Orchestrator
// Integrates: Vector Engine, Voice Engine, Vision, and GPT-OSS

use crate::agent::brain::{Brain, Intent};
use crate::agent::gpt_oss_bridge::SynoidAgent;
use crate::agent::vector_engine::{upscale_video, vectorize_video, VectorConfig};
use crate::agent::voice::engine::VoiceEngine;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

/// The Super Engine is the high-level controller for all Synoid capabilities.
/// It uses the Brain for intent classification and GPT-OSS for complex reasoning,
/// then delegates tasks to specialized engines (Vector, Voice, etc.).
pub struct SuperEngine {
    brain: Brain,
    gpt_brain: Option<SynoidAgent>,
    voice_engine: Option<Arc<VoiceEngine>>,
    // Vector Engine is largely stateless functions, so we don't need a struct instance
    work_dir: PathBuf,
}

impl SuperEngine {
    /// Initialize the Super Engine with all sub-systems.
    /// Uses graceful degradation: subsystems that fail to init are disabled, not fatal.
    pub fn new(api_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!("[SUPER_ENGINE] Initializing Synoid Unified Systems...");

        let brain = Brain::new(api_url);
        let gpt_brain = Some(SynoidAgent::new(api_url));

        // Initialize Voice Engine with graceful degradation
        let voice_engine = match VoiceEngine::new() {
            Ok(v) => {
                info!("[SUPER_ENGINE] Voice Engine: Online ✓");
                Some(Arc::new(v))
            }
            Err(e) => {
                warn!("[SUPER_ENGINE] Voice Engine failed to init: {}. Voice features disabled.", e);
                None
            }
        };

        let work_dir = std::env::current_dir()?.join("synoid_workspace");
        if !work_dir.exists() {
            std::fs::create_dir_all(&work_dir)?;
        }

        info!("[SUPER_ENGINE] Systems Online. Voice: {}", if voice_engine.is_some() { "Active" } else { "Disabled" });
        Ok(Self {
            brain,
            gpt_brain,
            voice_engine,
            work_dir,
        })
    }

    /// Primary entry point for any user command
    pub async fn process_command(&mut self, command: &str) -> Result<String, String> {
        info!("[SUPER_ENGINE] Processing: \"{}\"", command);

        // 1. Fast Path: Heuristic Classification via Brain
        let intent = self.brain.fast_classify(command);

        match intent {
            Intent::Unknown { request } => {
                // 2. Slow Path: GPT-OSS Reasoning
                info!("[SUPER_ENGINE] Complex request detected. Consulting GPT-OSS...");
                if let Some(gpt) = &self.gpt_brain {
                    match gpt.reason(&request).await {
                        Ok(response) => {
                            // In a full implementation, we would parse `response` for tool calls.
                            // For now, we return the reasoning.
                            Ok(format!("GPT-OSS Analysis: {}", response))
                        }
                        Err(e) => Err(format!("GPT-OSS failed: {}", e)),
                    }
                } else {
                    Err("GPT-OSS Brain not available".to_string())
                }
            }
            // 3. Execution Paths for Known Intents
            known_intent => self.execute_intent(known_intent).await,
        }
    }

    async fn execute_intent(&mut self, intent: Intent) -> Result<String, String> {
        match intent {
            Intent::DownloadYoutube { url } => {
                 // Delegate back to Brain's handler or implement here.
                 // Brain::process handles this well, so we can reuse, OR implement clean here.
                 // To avoid ownership issues with Brain::process taking &mut self, we implement logic here.
                 use crate::agent::source_tools;
                 let output_dir = self.work_dir.join("downloads");
                 if !output_dir.exists() { std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?; }
                 
                 match source_tools::download_youtube(&url, &output_dir, None).await {
                     Ok(info) => Ok(format!("Downloaded: {} to {:?}", info.title, output_dir)),
                     Err(e) => Err(format!("Download failed: {}", e)),
                 }
            }
            Intent::Vectorize { input, preset } => {
                let input_path = Path::new(&input); // In real app, resolve relative paths carefully
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
                if let Some(voice) = &self.voice_engine {
                    let input_path = Path::new(&input);
                    match voice.create_profile(&name, input_path) {
                        Ok(_) => Ok(format!("Voice profile '{}' created from {:?}", name, input_path)),
                        Err(e) => Err(format!("Voice cloning failed: {}", e)),
                    }
                } else {
                    Err("Voice Engine is not available (failed to initialize). Voice features are disabled.".to_string())
                }
            }
            Intent::Speak { text, profile } => {
                if self.voice_engine.is_none() {
                    return Err("Voice Engine is not available. Voice features are disabled.".to_string());
                }
                let _output_path = self.work_dir.join("speech_output.wav");
                // TODO: Wire up actual TTS call in VoiceEngine
                Ok(format!("(Simulated) Spoke: \"{}\" as '{}'", text, profile))
            }
            Intent::Research { topic } => {
                 // Reuse Brain's logic
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
                 // TODO: Wire up MotorCortex with real visual/audio data
                 // use crate::agent::motor_cortex::MotorCortex;
                 // let cortex = MotorCortex::new("http://localhost:11434/v1");
                 // Requires scanning video first to get VisualScene/AudioAnalysis data

                 Ok(format!("Embodied Edit Initiated: '{}' on {}", instruction, input))
            }
            Intent::Unknown { .. } => unreachable!("Handled in process_command"),
        }
    }
}
