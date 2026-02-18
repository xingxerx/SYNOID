use crate::agent::academy::StyleLibrary;
use crate::agent::audio_tools::AudioAnalysis;
use crate::agent::vision_tools::VisualScene;
use crate::agent::voice::transcription::TranscriptSegment;
use crate::agent::smart_editor;
use crate::agent::production_tools::safe_arg_path;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;
use tracing::{info, warn, error};

/// Structured plan for LLM-directed editing (Intermediate Representation)
#[derive(Debug, Deserialize, Serialize)]
pub struct EditPlan {
    pub trim_silence: bool,
    pub silence_threshold_db: f32,
    pub normalize_audio: bool,
    pub target_duration_secs: Option<f64>,
    pub transitions: Vec<TransitionSpec>,
    pub funny_moments: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransitionSpec {
    pub time: f64,
    pub r#type: String, // "wipe", "fade", "cut"
    pub duration: f32,
}

impl EditPlan {
    /// Validates and parses the JSON response from an LLM
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let plan: Self = serde_json::from_str(json_str)?;
        // Add additional validation here if needed
        Ok(plan)
    }
}

#[allow(dead_code)]
pub struct MotorCortex {
    api_url: String,
}

#[allow(dead_code)]
pub struct EditGraph {
    pub commands: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum TransitionType {
    Cut,
    Mix,
    WipeLeft,
    WipeRight,
    SlideLeft,
    SlideRight,
    CircleOpen,
    ZoomPan, // Custom zoom
    Glitch,  // Custom glitch logic
}

pub trait TransitionAgent {
    fn generate_filter(
        &self,
        input_idx_a: usize,
        input_idx_b: usize,
        duration: f32,
        offset: f32,
    ) -> String;
}

pub struct SmartTransition {
    pub transition_type: TransitionType,
}

impl TransitionAgent for SmartTransition {
    fn generate_filter(
        &self,
        input_idx_a: usize,
        input_idx_b: usize,
        duration: f32,
        offset: f32,
    ) -> String {
        match self.transition_type {
            TransitionType::Cut => {
                format!(
                    "[{0}][{1}]xfade=transition=fade:duration=0.1:offset={2}[v{1}]",
                    input_idx_a, input_idx_b, offset
                )
            }
            TransitionType::Mix => format!(
                "[{0}][{1}]xfade=transition=fade:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::WipeLeft => format!(
                "[{0}][{1}]xfade=transition=wipeleft:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::WipeRight => format!(
                "[{0}][{1}]xfade=transition=wiperight:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::SlideLeft => format!(
                "[{0}][{1}]xfade=transition=slideleft:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::SlideRight => format!(
                "[{0}][{1}]xfade=transition=slideright:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::CircleOpen => format!(
                "[{0}][{1}]xfade=transition=circleopen:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::ZoomPan => {
                format!(
                    "[{0}][{1}]xfade=transition=circleopen:duration={2}:offset={3}[v{1}]",
                    input_idx_a, input_idx_b, duration, offset
                )
            }
            TransitionType::Glitch => {
                format!(
                    "[{0}][{1}]xfade=transition=pixelize:duration={2}:offset={3}[v{1}]",
                    input_idx_a, input_idx_b, duration, offset
                )
            }
        }
    }
}

impl MotorCortex {
    pub fn new(api_url: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
        }
    }

    pub async fn execute_smart_render(
        &mut self,
        intent: &str,
        input: &Path,
        output: &Path,
        visual_data: &[VisualScene],
        transcript: &[TranscriptSegment],
        _audio_data: &AudioAnalysis,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        info!("[CORTEX] 🧠 Planning Smart Render based on Visual Analysis & Sovereign Ear...");

        // 1. Convert VisualScene to SmartEditor::Scene
        let mut editor_scenes = Vec::new();
        
        for i in 0..visual_data.len() {
            let start = visual_data[i].timestamp;
            let end = if i + 1 < visual_data.len() {
                visual_data[i+1].timestamp
            } else {
                start + 5.0 // Placeholder for last shot
            };
            
            if end > start {
                editor_scenes.push(smart_editor::Scene {
                    start_time: start,
                    end_time: end,
                    duration: end - start,
                    score: 0.5,
                });
            }
        }

        info!(
            "[CORTEX] 🛠️ Integrating {} visual scenes into Smart Editor pipeline.",
            editor_scenes.len()
        );

        let callback: Box<dyn Fn(&str) + Send + Sync> = Box::new(|msg: &str| {
            info!("{}", msg);
        });

        let funny_mode = intent.to_lowercase().contains("funny") || intent.to_lowercase().contains("comedy");

        let transcript_opt = if transcript.is_empty() {
            None
        } else {
            Some(transcript.to_vec())
        };

        match smart_editor::smart_edit(
            input,
            intent,
            output,
            funny_mode,
            Some(callback),
            Some(editor_scenes),
            transcript_opt,
        ).await {
            Ok(summary) => {
                info!("[CORTEX] ✅ Smart Edit completed via high-order logic.");
                Ok(summary)
            }
            Err(e) => {
                warn!("[CORTEX] ⚠️ Smart Edit pipeline failed: {}. Falling back to one-shot filter.", e);
                // Fallback to the old filter-only method if the complex edit fails
                self.execute_one_shot_render(intent, input, output, visual_data, _audio_data).await
            }
        }
    }

    pub async fn execute_one_shot_render(
        &mut self,
        intent: &str,
        input: &Path,
        output: &Path,
        _visual_data: &[VisualScene],
        _audio_data: &AudioAnalysis,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let library = StyleLibrary::new();
        let profile = library.get_profile(intent);

        info!("[CORTEX] Applying Style Profile: {}", profile.name);

        let mut filters = Vec::new();

        if profile.anamorphic {
            filters.push("crop=in_w:in_w/2.39".to_string()); // 2.39:1 Cinematic Mask
        }

        if let Some(lut) = &profile.color_lut {
            filters.push(format!("lut3d={}", lut));
        }

        let mut audio_filters = Vec::new();
        let intent_lower = intent.to_lowercase();

        if intent_lower.contains("ruthless")
            || intent_lower.contains("cut")
            || intent_lower.contains("short")
        {
            info!("[CORTEX] ✂️ Applying Ruthless Silence Removal");
            audio_filters.push(
                "silenceremove=stop_periods=-1:stop_duration=1:stop_threshold=-40dB".to_string(),
            );
        }

        if intent_lower.contains("enhance")
            || intent_lower.contains("fix")
            || intent_lower.contains("voice")
            || intent_lower.contains("audio")
            || intent_lower.contains("louder")
        {
            info!("[CORTEX] 🎙️ Enhancing Voice Clarity & Volume");
            audio_filters.push("highpass=f=100".to_string());
            audio_filters
                .push("acompressor=threshold=-12dB:ratio=4:attack=5:release=50".to_string());
            audio_filters.push("equalizer=f=3000:t=q:w=1:g=5".to_string());
            audio_filters.push("loudnorm=I=-16:TP=-1.5:LRA=11".to_string());
        }

        info!("[CORTEX] 🚀 Executing FFmpeg Render...");

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
            .arg("-i")
            .arg(safe_arg_path(input))
            .arg("-c:v")
            .arg("libx264")
            .arg("-preset")
            .arg("medium")
            .arg("-crf")
            .arg("23")
            .arg("-pix_fmt")
            .arg("yuv420p");

        if !filters.is_empty() {
            cmd.arg("-vf").arg(filters.join(","));
        }

        if !audio_filters.is_empty() {
            cmd.arg("-af").arg(audio_filters.join(","));
            cmd.arg("-c:a").arg("aac").arg("-b:a").arg("192k");
        } else {
            cmd.arg("-c:a").arg("copy");
        }

        cmd.arg(safe_arg_path(output));

        let status = cmd.status().await?;

        if status.success() {
            info!("[CORTEX] ✅ Render Complete: {:?}", output);
            Ok(format!("Rendered: {:?}", output))
        } else {
            error!("[CORTEX] ❌ Render Failed");
            Err("FFmpeg execution failed".into())
        }
    }
}
