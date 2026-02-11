use crate::funny_engine::analyzer::FunnyMoment;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub struct ContentInjector {}

impl ContentInjector {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn inject_content(
        &self,
        input: &Path,
        output: &Path,
        moments: &[FunnyMoment],
    ) -> Result<()> {
        if moments.is_empty() {
            println!("No funny moments found. Copying input -> output.");
            if input != output {
                std::fs::copy(input, output).context("Failed to copy file")?;
            }
            return Ok(());
        }

        println!("🤡 Injecting {} funny bits...", moments.len());

        // Build the enable expression: between(t,s1,e1)+between(t,s2,e2)+...
        // Limit to 50 moments to avoid command line length issues for now
        let enable_expr: String = moments
            .iter()
            .take(50)
            .map(|m| {
                format!(
                    "between(t,{:.2},{:.2})",
                    m.start_time,
                    m.start_time + m.duration
                )
            })
            .collect::<Vec<_>>()
            .join("+");

        // Simple text overlay: "LOL" in center, flashing yellow/red?
        // fontfile usage might be tricky without a known font path on Windows.
        // Windows usually has C:\Windows\Fonts\arial.ttf
        // But drawtext might fallback to default if fontfile not specified? No, usually needs fontfile or fontconfig.
        // On Windows, specifying font path is safest.

        let font_path = "C:/Windows/Fonts/arial.ttf";
        // If file doesn't exist, we might fail. Let's assume it exists or use a safer default?
        // Actually, let's check if it exists or let ffmpeg try.

        let filter = format!(
            "drawtext=fontfile='{font}':text='LOL':fontsize=120:fontcolor=yellow:borderw=5:bordercolor=black:x=(w-text_w)/2:y=(h-text_h)/2:enable='{}'",
            enable_expr,
            font = font_path
        );

        println!("  Filter: copy audio, re-encode video with overlay...");

        let status = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(input)
            .arg("-vf")
            .arg(&filter)
            .arg("-c:a")
            .arg("copy") // Preserves audio (important!)
            .arg(output)
            .output()
            .context("Failed to execute ffmpeg for injection")?;

        if !status.status.success() {
            // Fallback: maybe font path logic failed?
            // Print error
            anyhow::bail!(
                "FFmpeg injection failed: {:?}",
                String::from_utf8_lossy(&status.stderr)
            );
        }

        Ok(())
    }
}
