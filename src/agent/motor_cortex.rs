use crate::agent::academy::StyleLibrary;
use crate::agent::audio_tools::AudioAnalysis;
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

        let filter_arg = if filters.is_empty() {
            String::new()
        } else {
            filters.join(",")
        };

        info!("[CORTEX] 🚀 Executing FFmpeg Render...");

        let mut cmd = Command::new("ffmpeg");
        // Use standard flags separate from arguments for security and correctness
        cmd.arg("-y")
            .arg("-i")
            .arg(input)
            .arg("-c:v")
            .arg("libx264")
            .arg("-preset")
            .arg("medium")
            .arg("-crf")
            .arg("23");

        if !filters.is_empty() {
            cmd.arg("-vf").arg(&filter_arg);
        }

        cmd.arg(output);

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
