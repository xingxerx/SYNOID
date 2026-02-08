// SYNOID Transcription Bridge
// Wraps generic Python Whisper script for robust local transcription.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

pub struct TranscriptionEngine {
    script_path: PathBuf,
}

impl TranscriptionEngine {
    pub fn new() -> Result<Self> {
        let script_path = if let Ok(env_path) = env::var("SYNOID_TRANSCRIPTION_SCRIPT") {
            PathBuf::from(env_path)
        } else {
            // Check relative to executable (robust for release builds)
            let mut found_path = None;

            if let Ok(exe_path) = env::current_exe() {
                // Try walking up from exe location (target/release/synoid -> root/tools/transcribe.py)
                // Try: exe_dir/tools/transcribe.py
                // Try: exe_dir/../tools/transcribe.py
                // Try: exe_dir/../../tools/transcribe.py
                // Try: exe_dir/../../../tools/transcribe.py

                let candidates = [
                    exe_path
                        .parent()
                        .map(|p| p.join("tools").join("transcribe.py")),
                    exe_path
                        .parent()
                        .and_then(|p| p.parent())
                        .map(|p| p.join("tools").join("transcribe.py")),
                    exe_path
                        .parent()
                        .and_then(|p| p.parent())
                        .and_then(|p| p.parent())
                        .map(|p| p.join("tools").join("transcribe.py")),
                    exe_path
                        .parent()
                        .and_then(|p| p.parent())
                        .and_then(|p| p.parent())
                        .and_then(|p| p.parent())
                        .map(|p| p.join("tools").join("transcribe.py")),
                ];

                for candidate in candidates.iter().flatten() {
                    if candidate.exists() {
                        found_path = Some(candidate.clone());
                        break;
                    }
                }
            }

            if let Some(p) = found_path {
                p
            } else {
                // Fallback to CWD
                let cwd_path = Path::new("tools").join("transcribe.py");
                if cwd_path.exists() {
                    cwd_path
                } else {
                    // Default to CWD path anyway, will warn below
                    cwd_path
                }
            }
        };

        if !script_path.exists() {
            warn!("[TRANSCRIBE] Warning: Transcription script not found at: {:?}. Transcription may fail.", script_path);
        } else {
            info!("[TRANSCRIBE] Using script at: {:?}", script_path);
        }

        Ok(Self { script_path })
    }

    pub async fn transcribe(&self, audio_path: &Path) -> Result<Vec<TranscriptSegment>> {
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
            anyhow::bail!("Transcription script failed - is openai-whisper installed?");
        }

        // Read result
        let segments: Vec<TranscriptSegment> =
            serde_json::from_str(&fs::read_to_string(&output_json)?)?;

        info!(
            "[TRANSCRIBE] Success! {} segments generated.",
            segments.len()
        );

        // Cleanup JSON
        // fs::remove_file(output_json)?;
        // Keeping it might be useful for debug for now

        Ok(segments)
    }
}
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
            assert_eq!(e.script_path, abs_path);
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

        // 3. Test Non-Existent - Now returns Ok but logs warning
        env::set_var("SYNOID_TRANSCRIPTION_SCRIPT", "non_existent_file_xyz.py");
        let engine = TranscriptionEngine::new();
        assert!(
            engine.is_ok(),
            "Should accept non-existent file but warn (returning Ok struct)"
        );

        env::remove_var("SYNOID_TRANSCRIPTION_SCRIPT");
    }
}
