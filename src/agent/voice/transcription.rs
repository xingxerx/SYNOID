// SYNOID Transcription Bridge
// Wraps generic Python Whisper script for robust local transcription.

<<<<<<< HEAD
use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};
use tracing::info;
use std::fs;
use std::env;
use anyhow::{Context, Result};
=======
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use tracing::info;
>>>>>>> pr-9

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
<<<<<<< HEAD
    pub fn new() -> Result<Self> {
        let script_path = if let Ok(env_path) = env::var("SYNOID_TRANSCRIPTION_SCRIPT") {
            PathBuf::from(env_path)
        } else {
            // Fallback to relative path
            let current_dir = env::current_dir().context("Failed to get current directory")?;
            current_dir.join("tools").join("transcribe.py")
        };

        if !script_path.exists() {
             anyhow::bail!("Transcription script not found at: {:?}", script_path);
        }

        Ok(Self {
            script_path: script_path.to_string_lossy().to_string(),
        })
    }

    pub fn transcribe(&self, audio_path: &Path) -> Result<Vec<TranscriptSegment>> {
=======
    pub fn new() -> Self {
        Self {
            script_path: "d:/SYNOID/tools/transcribe.py".to_string(),
        }
    }

    pub fn transcribe(
        &self,
        audio_path: &Path,
    ) -> Result<Vec<TranscriptSegment>, Box<dyn std::error::Error>> {
>>>>>>> pr-9
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
            .status()?;

        if !status.success() {
<<<<<<< HEAD
            anyhow::bail!("Transcription script failed - is openai-whisper installed?");
=======
            return Err("Transcription script failed - is openai-whisper installed?".into());
>>>>>>> pr-9
        }

        // Read result
        let data = fs::read_to_string(&output_json)?;
        let segments: Vec<TranscriptSegment> = serde_json::from_str(&data)?;

<<<<<<< HEAD
        info!("[TRANSCRIBE] Success! {} segments generated.", segments.len());
=======
        info!(
            "[TRANSCRIBE] Success! {} segments generated.",
            segments.len()
        );
>>>>>>> pr-9

        // Cleanup JSON
        // fs::remove_file(output_json)?;
        // Keeping it might be useful for debug for now

        Ok(segments)
    }
}
<<<<<<< HEAD
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::sync::Mutex;

    // Mutex to serialize tests that modify env vars
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_transcription_engine_config() {
        let _lock = ENV_LOCK.lock().unwrap();

        // 1. Test Env Var Override
        let temp_script = "test_script.py";
        // Create a dummy file
        File::create(temp_script).unwrap();
        let abs_path = env::current_dir().unwrap().join(temp_script);

        env::set_var("SYNOID_TRANSCRIPTION_SCRIPT", &abs_path);
        let engine = TranscriptionEngine::new();
        assert!(engine.is_ok(), "Should find script via env var");
        if let Ok(e) = engine {
             assert_eq!(e.script_path, abs_path.to_string_lossy().to_string());
        }

        // Cleanup
        env::remove_var("SYNOID_TRANSCRIPTION_SCRIPT");
        let _ = fs::remove_file(temp_script);

        // 2. Test Fallback
        // This assumes tools/transcribe.py exists in the repo root as seen in `ls`.
        // If the test runner changes CWD, this might fail, but standard cargo test runs in crate root.
        let engine = TranscriptionEngine::new();
        // We expect this to pass if tools/transcribe.py exists
        if Path::new("tools/transcribe.py").exists() {
             assert!(engine.is_ok(), "Should find default tools/transcribe.py");
        } else {
            // If the file doesn't exist in the environment (e.g. CI without it), it should fail.
            // But for this environment, we know it exists.
            println!("Note: tools/transcribe.py not found in CWD, skipping default path test");
        }

        // 3. Test Failure
        env::set_var("SYNOID_TRANSCRIPTION_SCRIPT", "non_existent_file_xyz.py");
        let engine = TranscriptionEngine::new();
        assert!(engine.is_err(), "Should fail for non-existent file");

        env::remove_var("SYNOID_TRANSCRIPTION_SCRIPT");
    }
}
=======
>>>>>>> pr-9
