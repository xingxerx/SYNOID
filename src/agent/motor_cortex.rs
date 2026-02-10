use crate::agent::academy::StyleLibrary;
use crate::agent::audio_tools::AudioAnalysis;
use crate::agent::production_tools::safe_arg_path;
use crate::agent::vision_tools::VisualScene;
use std::path::Path;
use tokio::process::Command;
use tracing::{error, info};

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
        let mut filters = Vec::new();

        if profile.anamorphic {
            filters.push("crop=in_w:in_w/2.39".to_string()); // 2.39:1 Cinematic Mask
        }

        if let Some(lut) = &profile.color_lut {
            filters.push(format!("lut3d={}", lut));
        }

        // 2. Build Video Filtergraph
        let filter_arg = if filters.is_empty() {
            String::new()
        } else {
            filters.join(",")
        };

        // 3. Build Audio Filtergraph (Enhanced Voice & Smart Cut)
        let mut audio_filters = Vec::new();
        let intent_lower = intent.to_lowercase();

        // Feature: Smart Cut (Silence Removal) - "Ruthless" editing
        if intent_lower.contains("ruthless") 
            || intent_lower.contains("cut") 
            || intent_lower.contains("short") 
        {
             info!("[CORTEX] ✂️ Applying Ruthless Silence Removal");
             // fail: 1s silence, threshold: -40dB 
             audio_filters.push("silenceremove=stop_periods=-1:stop_duration=1:stop_threshold=-40dB".to_string());
        }

        // Feature: Neural Audio Enhancement
        if intent_lower.contains("enhance")
            || intent_lower.contains("fix")
            || intent_lower.contains("voice")
            || intent_lower.contains("audio")
            || intent_lower.contains("louder") // User asked for louder voice
        {
             info!("[CORTEX] 🎙️ Enhancing Voice Clarity & Volume");
             // 1. Highpass to remove rumble
             audio_filters.push("highpass=f=100".to_string());
             // 2. Compressor to level out voice (makes it "louder" and consistent)
             audio_filters.push("acompressor=threshold=-12dB:ratio=4:attack=5:release=50".to_string());
             // 3. EQ Presense boost
             audio_filters.push("equalizer=f=3000:t=q:w=1:g=5".to_string());
             // 4. Loudness Normalization to standard -16 LUFS
             audio_filters.push("loudnorm=I=-16:TP=-1.5:LRA=11".to_string());
        }

        info!("[CORTEX] 🚀 Executing FFmpeg Render...");

        let mut cmd = Command::new("ffmpeg");
        // Use standard flags separate from arguments for security and correctness
        cmd.arg("-y")
            .arg("-i")
            .arg(safe_arg_path(input))
            .arg("-c:v")
            .arg("libx264")
            .arg("-preset")
            .arg("medium")
            .arg("-crf")
            .arg("23")
            // FIX: Force yuv420p for compatibility to prevent playback lag
            .arg("-pix_fmt")
            .arg("yuv420p");

        if !filters.is_empty() {
            cmd.arg("-vf").arg(&filter_arg);
        }

        if !audio_filters.is_empty() {
            cmd.arg("-af").arg(audio_filters.join(","));
            cmd.arg("-c:a").arg("aac").arg("-b:a").arg("192k");
        } else {
            // Default copy if no processing
            cmd.arg("-c:a").arg("copy");
        }

        cmd.arg(safe_arg_path(output));

        // STREAMING OUTPUT
        // We spawn the child process which inherits stdout/stderr by default in tokio::process::Command unless piped.
        // Wait, tokio Command defaults to inheriting stdio? No, it inherits if not specified?
        // Docs say: "By default, stdin, stdout and stderr are inherited from the parent."
        // So this should stream to the console automatically.

        let mut child = cmd.spawn()?;
        let status = child.wait().await?;

        if status.success() {
            info!("[CORTEX] ✅ Render Complete: {:?}", output);
            Ok(format!("Rendered: {:?}", output))
        } else {
            error!("[CORTEX] ❌ Render Failed");
            Err("FFmpeg execution failed".into())
        }
    }
}
