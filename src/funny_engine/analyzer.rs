use anyhow::{Context, Result};
use hound::WavReader;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct FunnyMoment {
    pub start_time: f64,
    pub duration: f64,
    pub intensity: f64,
    pub moment_type: MomentType,
}

#[derive(Debug, Clone)]
pub enum MomentType {
    Laughter,
    DeadSilence,
    Chaos,
}

pub struct AudioAnalyzer {}

impl AudioAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn find_funny_moments(&self, input: &Path) -> Result<Vec<FunnyMoment>> {
        let temp_wav = input.with_extension("temp_analysis.wav");

        // 1. Extract Audio
        println!("🎧 Extracting audio for analysis...");
        let status = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(input)
            .arg("-ac")
            .arg("1") // Mono
            .arg("-ar")
            .arg("16000") // Low sample rate for speed
            .arg(&temp_wav)
            .output()
            .context("Failed to run ffmpeg for audio extraction")?;

        if !status.status.success() {
            anyhow::bail!(
                "FFmpeg audio extraction failed: {:?}",
                String::from_utf8_lossy(&status.stderr)
            );
        }

        // 2. Analyze Audio
        println!("📊 Analyzing audio energy levels...");
        let mut moments = Vec::new();
        let mut reader = WavReader::open(&temp_wav).context("Failed to open temp wav file")?;
        let samples: Vec<i16> = reader.samples().map(|s| s.unwrap_or(0)).collect();
        let sample_rate = reader.spec().sample_rate as usize;

        let chunk_size = sample_rate / 2; // 0.5 seconds
        let mut chunk_index = 0;

        for chunk in samples.chunks(chunk_size) {
            let start_time = (chunk_index * chunk_size) as f64 / sample_rate as f64;
            let duration = chunk_size as f64 / sample_rate as f64;

            // Calculate RMS
            let sum_squares: f64 = chunk.iter().map(|&s| (s as f64).powi(2)).sum();
            let rms = (sum_squares / chunk.len() as f64).sqrt();

            // Normalize RMS (roughly, assuming 16-bit audio)
            let intensity = rms / 32768.0;

            // Simple Heuristics
            if intensity > 0.4 {
                println!(
                    "  Found loud moment at {:.1}s (Intensity: {:.2})",
                    start_time, intensity
                );
                moments.push(FunnyMoment {
                    start_time,
                    duration,
                    intensity,
                    moment_type: MomentType::Laughter, // Assuming loud is funny/laughter
                });
            } else if intensity < 0.001 {
                // Silence detection (maybe too sensitive)
                // moments.push(FunnyMoment { start_time, duration, intensity, moment_type: MomentType::DeadSilence });
            }

            chunk_index += 1;
        }

        // Cleanup
        let _ = std::fs::remove_file(temp_wav);

        // Deduplicate/Merge adjacent moments
        let merged_moments = self.merge_moments(moments);
        println!("✨ Found {} funny bits!", merged_moments.len());

        Ok(merged_moments)
    }

    fn merge_moments(&self, raw: Vec<FunnyMoment>) -> Vec<FunnyMoment> {
        if raw.is_empty() {
            return vec![];
        }

        let mut merged = Vec::new();
        let mut current = raw[0].clone();

        for next in raw.iter().skip(1) {
            if next.start_time - (current.start_time + current.duration) < 1.0 {
                // Merge if close enough
                current.duration = (next.start_time + next.duration) - current.start_time;
                current.intensity = current.intensity.max(next.intensity);
            } else {
                merged.push(current);
                current = next.clone();
            }
        }
        merged.push(current);
        merged
    }
}
