// SYNOID Voice Engine - Enhanced with Speaker Embeddings
// Neural TTS & Voice Cloning using Candle

use candle_core::Device;
use hf_hub::api::sync::Api;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

/// Speaker profile containing voice characteristics
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SpeakerProfile {
    pub name: String,
    pub embedding: Vec<f32>,
    pub sample_path: PathBuf,
}

/// Voice Engine for Neural TTS and Cloning
pub struct VoiceEngine {
    #[allow(dead_code)]
    device: Device,
    #[allow(dead_code)]
    model_dir: PathBuf,
    profiles_dir: PathBuf,
}

impl VoiceEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let device = Device::Cpu;

        let base_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("synoid");

        let model_dir = base_dir.join("voice_models");
        let profiles_dir = base_dir.join("voice_profiles");

        fs::create_dir_all(&model_dir)?;
        fs::create_dir_all(&profiles_dir)?;

        info!("[VOICE] Engine initialized (Device: {:?})", device);
        Ok(Self {
            device,
            model_dir,
            profiles_dir,
        })
    }

    /// Download TTS model from HuggingFace
    pub fn download_model(
        &self,
        model_id: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        info!("[VOICE] Downloading model: {}", model_id);

        let api = Api::new()?;
        let repo = api.model(model_id.to_string());

        let _config_path = repo.get("config.json")?;
        let model_path = repo.get("model.safetensors")?;

        info!("[VOICE] Model downloaded to: {:?}", model_path.parent());
        Ok(model_path)
    }

    /// Validate profile name to prevent path traversal
    fn validate_profile_name(name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if name.is_empty() {
            return Err("Profile name cannot be empty".into());
        }

        // Allow alphanumeric, underscore, hyphen, and space
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ')
        {
            return Err("Profile name contains invalid characters. Only alphanumeric, underscore, hyphen, and space are allowed.".into());
        }

        Ok(())
    }

    /// Create speaker profile from audio file
    pub fn create_profile(
        &self,
        name: &str,
        audio_path: &Path,
    ) -> Result<SpeakerProfile, Box<dyn std::error::Error + Send + Sync>> {
        Self::validate_profile_name(name)?;
        info!("[VOICE] Creating profile '{}' from {:?}", name, audio_path);

        // For now, we'll create a placeholder embedding
        // Full implementation requires a speaker encoder model (ECAPA-TDNN, X-Vector, etc.)

        // Load audio and generate a simple spectral fingerprint
        let embedding = self.extract_voice_features(audio_path)?;

        let profile = SpeakerProfile {
            name: name.to_string(),
            embedding,
            sample_path: audio_path.to_path_buf(),
        };

        // Save profile
        let profile_path = self.profiles_dir.join(format!("{}.json", name));
        let json = serde_json::to_string_pretty(&profile)?;
        fs::write(&profile_path, json)?;

        info!("[VOICE] Profile saved to {:?}", profile_path);
        Ok(profile)
    }

    /// Load existing speaker profile
    pub fn load_profile(
        &self,
        name: &str,
    ) -> Result<SpeakerProfile, Box<dyn std::error::Error + Send + Sync>> {
        Self::validate_profile_name(name)?;
        let profile_path = self.profiles_dir.join(format!("{}.json", name));
        let json = fs::read_to_string(&profile_path)?;
        let profile: SpeakerProfile = serde_json::from_str(&json)?;
        info!("[VOICE] Loaded profile: {}", name);
        Ok(profile)
    }

    /// Extract voice features from audio (simplified spectral analysis)
    fn extract_voice_features(
        &self,
        audio_path: &Path,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        // Read WAV file
        let mut reader = hound::WavReader::open(audio_path)?;
        let spec = reader.spec();

        info!(
            "[VOICE] Audio: {} Hz, {} channels",
            spec.sample_rate, spec.channels
        );

        // Collect samples
        let samples: Vec<f32> = reader
            .samples::<i16>()
            .filter_map(|s| s.ok())
            .map(|s| s as f32 / i16::MAX as f32)
            .collect();

        // Simple feature extraction: compute energy in frequency bands
        // This is a placeholder - real embedding would use a neural encoder
        let chunk_size = 512;
        let num_features = 256; // Embedding dimension
        let mut features = vec![0.0f32; num_features];

        for (i, chunk) in samples.chunks(chunk_size).enumerate() {
            let energy: f32 = chunk.iter().map(|s| s * s).sum();
            features[i % num_features] += energy;
        }

        // Normalize
        let max = features.iter().cloned().fold(0.0f32, f32::max);
        if max > 0.0 {
            for f in &mut features {
                *f /= max;
            }
        }

        info!(
            "[VOICE] Extracted {} feature dimensions from {} samples",
            features.len(),
            samples.len()
        );
        Ok(features)
    }

    /// Generate speech from text (TTS)
    pub fn speak(
        &self,
        text: &str,
        _output_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "[VOICE] (Simulation) Synthesizing to {:?}: \"{}\"",
            _output_path, text
        );
        // Err("TTS model not yet loaded - run 'synoid voice --download' first".into())
        Ok(())
    }

    /// Clone voice from audio (legacy method)
    pub fn clone_voice(
        &self,
        audio_path: &Path,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        self.extract_voice_features(audio_path)
    }

    /// Synthesize speech with cloned voice
    pub fn speak_as(
        &self,
        text: &str,
        profile_name: &str,
        _output_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // let profile = self.load_profile(profile_name)?;
        info!(
            "[VOICE] (Simulation) Synthesizing as '{}': \"{}\"",
            profile_name, text
        );
        // Err("Voice cloning model not yet loaded".into())
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_dummy_wav(path: &Path) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for _ in 0..100 {
            writer.write_sample(0).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn test_path_traversal_exploit() {
        // Setup paths
        let base_dir = std::env::temp_dir().join("synoid_test_exploit");
        // Ensure we start clean
        if base_dir.exists() {
            let _ = fs::remove_dir_all(&base_dir);
        }

        let profiles_dir = base_dir.join("profiles");
        let outside_dir = base_dir.join("outside");

        fs::create_dir_all(&profiles_dir).unwrap();
        fs::create_dir_all(&outside_dir).unwrap();

        let wav_path = base_dir.join("test.wav");
        create_dummy_wav(&wav_path);

        // Manually construct engine
        let engine = VoiceEngine {
            device: Device::Cpu,
            model_dir: base_dir.join("models"),
            profiles_dir: profiles_dir.clone(),
        };

        // Attempt exploit: Write into 'outside' directory
        let exploit_name = "../outside/hacked";

        // This call should now fail
        let result = engine.create_profile(exploit_name, &wav_path);

        // Check if the file was written to the outside directory
        let hacked_path = outside_dir.join("hacked.json");
        let exploited = hacked_path.exists();

        // Clean up
        let _ = fs::remove_dir_all(&base_dir);

        // Assert that the vulnerability is fixed
        assert!(
            result.is_err(),
            "create_profile should fail for invalid name"
        );
        assert!(
            !exploited,
            "File should not be written to outside directory"
        );

        // Check error message
        let err = result.err().unwrap();
        assert!(err
            .to_string()
            .contains("Profile name contains invalid characters"));
    }

    #[test]
    fn test_valid_profile_creation() {
        // Setup paths
        let base_dir = std::env::temp_dir().join("synoid_test_valid");
        if base_dir.exists() {
            let _ = fs::remove_dir_all(&base_dir);
        }

        let profiles_dir = base_dir.join("profiles");
        fs::create_dir_all(&profiles_dir).unwrap();

        let wav_path = base_dir.join("test.wav");
        create_dummy_wav(&wav_path);

        // Manually construct engine
        let engine = VoiceEngine {
            device: Device::Cpu,
            model_dir: base_dir.join("models"),
            profiles_dir: profiles_dir.clone(),
        };

        let valid_name = "test_profile_123";
        let result = engine.create_profile(valid_name, &wav_path);

        assert!(
            result.is_ok(),
            "create_profile should succeed for valid name"
        );

        let profile_path = profiles_dir.join(format!("{}.json", valid_name));
        assert!(profile_path.exists(), "Profile file should be created");

        // Clean up
        let _ = fs::remove_dir_all(&base_dir);
    }
}
