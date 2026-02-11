use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

pub struct TTSEngine {
    script_path: PathBuf,
}

impl TTSEngine {
    pub fn new() -> Result<Self> {
        // Locate synoid_tts.py similar to how transcription.rs locates transcribe.py
        let mut script_path = PathBuf::from("tools/synoid_tts.py");
        if !script_path.exists() {
             // Try absolute path if CWD is wrong (e.g. running from target/debug)
             if let Ok(exe_path) = std::env::current_exe() {
                 script_path = exe_path.parent().unwrap().join("../../../tools/synoid_tts.py");
             }
        }
        
        if !script_path.exists() {
             // Fallback to simpler relative check
             script_path = PathBuf::from("D:/SYNOID/tools/synoid_tts.py");
        }

        if !script_path.exists() {
            warn!("[TTS] Warning: synoid_tts.py not found at {:?}. TTS will fail.", script_path);
        }

        Ok(Self { script_path })
    }

    pub async fn speak(&self, text: &str, output_path: &Path, voice: Option<&str>) -> Result<()> {
        let voice = voice.unwrap_or("en-US-ChristopherNeural");
        
        info!("[TTS] Generating audio: \"{}\" -> {:?}", text, output_path);

        let status = Command::new("python")
            .arg(&self.script_path)
            .arg("--text")
            .arg(text)
            .arg("--output")
            .arg(output_path)
            .arg("--voice")
            .arg(voice)
            .status()
            .await
            .context("Failed to execute TTS script")?;

        if !status.success() {
            anyhow::bail!("TTS script failed");
        }

        Ok(())
    }
}
