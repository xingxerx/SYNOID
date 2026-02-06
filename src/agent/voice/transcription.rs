// SYNOID Transcription Bridge
// Wraps generic Python Whisper script for robust local transcription.

use std::path::Path;
use tokio::process::Command;
use serde::{Deserialize, Serialize};
use tracing::info;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

pub struct TranscriptionEngine {
    script_path: String,
}

impl TranscriptionEngine {
    pub fn new() -> Self {
        Self {
            script_path: "d:/SYNOID/tools/transcribe.py".to_string(),
        }
    }

    pub async fn transcribe(&self, audio_path: &Path) -> Result<Vec<TranscriptSegment>, Box<dyn std::error::Error>> {
        info!("[TRANSCRIBE] Audio: {:?}", audio_path);

        let work_dir = audio_path.parent().unwrap_or(Path::new("."));
        let output_json = work_dir.join("transcript.json");

        // Ensure python is available
        // We assume 'python' is in PATH or use generic 'python' command
        let status = Command::new("python")
            .arg(&self.script_path)
            .arg("--audio")
            .arg(audio_path.to_str().unwrap())
            .arg("--model")
            .arg("tiny") // Default to fast model
            .arg("--output")
            .arg(output_json.to_str().unwrap())
            .status()
            .await?;

        if !status.success() {
            return Err("Transcription script failed - is openai-whisper installed?".into());
        }

        // Read result
        let data = fs::read_to_string(&output_json)?;
        let segments: Vec<TranscriptSegment> = serde_json::from_str(&data)?;

        info!("[TRANSCRIBE] Success! {} segments generated.", segments.len());

        // Cleanup JSON
        // fs::remove_file(output_json)?;
        // Keeping it might be useful for debug for now

        Ok(segments)
    }
}
