use crate::agent::academy::StyleLibrary;
use crate::agent::audio_tools::AudioAnalysis;
use crate::agent::vision_tools::VisualScene;
use std::path::Path;
use tracing::info;

#[allow(dead_code)]
pub struct MotorCortex {
    api_url: String,
}

#[allow(dead_code)]
pub struct EditGraph {
    pub commands: Vec<String>,
}

impl MotorCortex {
    pub fn new(api_url: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
        }
    }

    pub async fn execute_one_shot_render(
        &mut self,
        intent: &str,
        input: &Path,
        output: &Path,
        _visual_data: &[VisualScene],
        _audio_data: &AudioAnalysis,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let library = StyleLibrary::new();
        let profile = library.get_profile(intent);

        info!("[CORTEX] Applying Style Profile: {}", profile.name);

        // 1. Rhythmic Assembly
        // Divide video into segments based on avg_shot_length and snap to nearest audio beat
        let _current_pos = 0.0;
        let mut filters = Vec::new();

        if profile.anamorphic {
            filters.push("crop=in_w:in_w/2.39".to_string()); // 2.39:1 Cinematic Mask
        }

        if let Some(lut) = &profile.color_lut {
            filters.push(format!("lut3d={}", lut));
        }

        // 2. Build FFmpeg Filtergraph (Video)
        let filter_str = if filters.is_empty() {
            String::new()
        } else {
            format!("-vf \"{}\"", filters.join(","))
        };

        // 3. Build Audio Filtergraph (Enhanced Voice)
        let mut audio_filters = Vec::new();
        // Check intent for "enhance voice" or similar variants
        let intent_lower = intent.to_lowercase();
        if (intent_lower.contains("enhance") || intent_lower.contains("fix")) && intent_lower.contains("voice") {
             info!("[CORTEX] Detected Voice Enhancement Intent. Applying Audio Clean-up.");
             // Standard Broadcast Spec: Denoise -> EQ Bandpass -> Loudness Normalization
             audio_filters.push("afftdn=nf=-25".to_string());
             audio_filters.push("highpass=f=200".to_string());
             audio_filters.push("lowpass=f=3000".to_string());
             audio_filters.push("loudnorm=I=-16:TP=-1.5:LRA=11".to_string());
        }

        let audio_filter_str = if audio_filters.is_empty() {
            String::new()
        } else {
            format!("-af \"{}\"", audio_filters.join(","))
        };

        // Construct Final Command
        // Note: Using -c:a aac for audio encoding when filters are present
        let audio_codec = if audio_filters.is_empty() { "-c:a copy" } else { "-c:a aac -b:a 192k" };

        let cmd = format!(
            "ffmpeg -i {} {} {} -c:v libx264 -preset slow -crf 18 {} -y {}",
            input.to_str().unwrap(),
            filter_str,
            audio_filter_str,
            audio_codec,
            output.to_str().unwrap()
        );

        Ok(cmd)
    }
}
