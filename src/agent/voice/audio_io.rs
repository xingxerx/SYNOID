// SYNOID Audio I/O Module
// Microphone Recording & Speaker Playback

use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use tracing::info;

/// Audio Input/Output Handler
pub struct AudioIO {
    sample_rate: u32,
}

impl AudioIO {
    pub fn new() -> Self {
        Self { sample_rate: 16000 } // 16kHz for voice
    }

    /// Record audio from microphone to WAV file.
    /// cpal::Stream is !Send, so the entire capture (including sleep) runs in spawn_blocking
    /// using std::thread::sleep to avoid holding a !Send type across an .await point.
    pub async fn record_to_file(
        &self,
        output_path: &Path,
        duration_secs: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Security check: Prevent directory traversal
        if output_path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err("Security Violation: Path traversal detected".into());
        }

        info!(
            "[VOICE] Recording {} seconds to {:?}...",
            duration_secs, output_path
        );

        let sample_rate = self.sample_rate;
        let output_path_buf = output_path.to_path_buf();

        tokio::task::spawn_blocking(move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

            let host = cpal::default_host();
            let device = host
                .default_input_device()
                .ok_or("No input device available")?;

            let config = cpal::StreamConfig {
                channels: 1,
                sample_rate: cpal::SampleRate(sample_rate),
                buffer_size: cpal::BufferSize::Default,
            };

            let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
            let samples_clone = samples.clone();

            let stream = device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut lock) = samples_clone.lock() {
                        lock.extend_from_slice(data);
                    }
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            )?;

            stream.play()?;
            std::thread::sleep(std::time::Duration::from_secs(duration_secs as u64));
            drop(stream);

            let samples_final = samples.lock().unwrap().clone();

            let spec = hound::WavSpec {
                channels: 1,
                sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let mut writer = hound::WavWriter::create(&output_path_buf, spec)?;
            for sample in samples_final {
                let amplitude = (sample * i16::MAX as f32) as i16;
                writer.write_sample(amplitude)?;
            }
            writer.finalize()?;
            Ok(())
        }).await?.map_err(|e| e.to_string())?;

        info!("[VOICE] Recording saved.");
        Ok(())
    }

    /// Play audio file through speakers
    pub async fn play_file(&self, audio_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use rodio::{Decoder, OutputStream, Sink};

        info!("[VOICE] Playing {:?}...", audio_path);
        let audio_path = audio_path.to_path_buf();

        // Offload blocking playback to thread
        tokio::task::spawn_blocking(move || {
            let (_stream, stream_handle) = OutputStream::try_default()?;
            let sink = Sink::try_new(&stream_handle)?;

            let file = File::open(&audio_path)?;
            let source = Decoder::new(std::io::BufReader::new(file))?;

            sink.append(source);
            sink.sleep_until_end();
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }).await?.map_err(|e| e.to_string())?;

        Ok(())
    }
}
