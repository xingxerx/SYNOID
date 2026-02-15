use crate::agent::academy::StyleLibrary;
use crate::agent::audio_tools::AudioAnalysis;
use crate::agent::smart_editor;
use crate::agent::vision_tools::VisualScene;
use crate::agent::voice::transcription::TranscriptSegment;
use std::path::Path;
use tracing::{info, warn};

#[allow(dead_code)]
pub struct MotorCortex {
    api_url: String,
}

#[allow(dead_code)]
pub struct EditGraph {
    pub commands: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum TransitionType {
    Cut,
    Mix,
    WipeLeft,
    WipeRight,
    SlideLeft,
    SlideRight,
    CircleOpen,
    ZoomPan, // Custom zoom
    Glitch,  // Custom glitch logic
}

pub trait TransitionAgent {
    fn generate_filter(
        &self,
        input_idx_a: usize,
        input_idx_b: usize,
        duration: f32,
        offset: f32,
    ) -> String;
}

pub struct SmartTransition {
    pub transition_type: TransitionType,
}

impl TransitionAgent for SmartTransition {
    fn generate_filter(
        &self,
        input_idx_a: usize,
        input_idx_b: usize,
        duration: f32,
        offset: f32,
    ) -> String {
        match self.transition_type {
            TransitionType::Cut => {
                // Hard cut doesn't need xfade, but for consistency in a chain, we might use xfade with duration 0?
                // Actually xfade doesn't support 0 duration well.
                // For cut, we usually just concat.
                // But if we are in an xfade chain, we might use a very fast fade?
                format!(
                    "[{0}][{1}]xfade=transition=fade:duration=0.1:offset={2}[v{1}]",
                    input_idx_a, input_idx_b, offset
                )
            }
            TransitionType::Mix => format!(
                "[{0}][{1}]xfade=transition=fade:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::WipeLeft => format!(
                "[{0}][{1}]xfade=transition=wipeleft:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::WipeRight => format!(
                "[{0}][{1}]xfade=transition=wiperight:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::SlideLeft => format!(
                "[{0}][{1}]xfade=transition=slideleft:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::SlideRight => format!(
                "[{0}][{1}]xfade=transition=slideright:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::CircleOpen => format!(
                "[{0}][{1}]xfade=transition=circleopen:duration={2}:offset={3}[v{1}]",
                input_idx_a, input_idx_b, duration, offset
            ),
            TransitionType::ZoomPan => {
                // Custom zoompan is not xfade, but we can simulate it or return a complex string?
                // For now, mapping to circleopen as placeholder for "zoom" transition
                format!(
                    "[{0}][{1}]xfade=transition=circleopen:duration={2}:offset={3}[v{1}]",
                    input_idx_a, input_idx_b, duration, offset
                )
            }
            TransitionType::Glitch => {
                // Glitch is complex. Mapping to 'pixelize' or 'hblur' if available in xfade?
                // xfade has 'pixelize'.
                format!(
                    "[{0}][{1}]xfade=transition=pixelize:duration={2}:offset={3}[v{1}]",
                    input_idx_a, input_idx_b, duration, offset
                )
            }
        }
    }
}

impl MotorCortex {
    pub fn new(api_url: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
        }
    }

    pub async fn execute_smart_render(
        &mut self,
        intent: &str,
        input: &Path,
        output: &Path,
        visual_data: &[VisualScene],
        transcript: &[TranscriptSegment],
        _audio_data: &AudioAnalysis,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        info!("[CORTEX] 🧠 Planning Smart Render based on Visual Analysis & Sovereign Ear...");

        // 1. Convert VisualScene to SmartEditor::Scene
        // We need to estimate end times for each visual scene based on the next timestamp
        let mut editor_scenes = Vec::new();

        // We don't have total duration here easily, but we can assume the last scene
        // ends at a reasonable point or just let the smart editor's detect_scenes (if we fall back) handle it.
        // But since we have visual_data, we should use it.

        for i in 0..visual_data.len() {
            let start = visual_data[i].timestamp;
            let end = if i + 1 < visual_data.len() {
                visual_data[i + 1].timestamp
            } else {
                // Approximate last scene duration or let it be handled
                start + 5.0 // Placeholder for last shot (SmartEditor will refine it anyway)
            };

            if end > start {
                editor_scenes.push(smart_editor::Scene {
                    start_time: start,
                    end_time: end,
                    duration: end - start,
                    score: 0.5,
                });
            }
        }

        info!(
            "[CORTEX] 🛠️ Integrating {} visual scenes into Smart Editor pipeline.",
            editor_scenes.len()
        );

        // 2. Transmit planning to the Smart Editor for higher-order execution (cutting)
        // We pass the pre-scanned data to avoid redundant FFmpeg passes.
        let callback: Box<dyn Fn(&str) + Send + Sync> = Box::new(|msg: &str| {
            info!("{}", msg);
        });

        // We assume 'funny_mode' is false unless the intent carries specific markers
        let funny_mode =
            intent.to_lowercase().contains("funny") || intent.to_lowercase().contains("comedy");

        let transcript_opt = if transcript.is_empty() {
            None
        } else {
            Some(transcript.to_vec())
        };

        match smart_editor::smart_edit(
            input,
            intent,
            output,
            funny_mode,
            Some(callback),
            Some(editor_scenes),
            transcript_opt,
        )
        .await
        {
            Ok(summary) => {
                info!("[CORTEX] ✅ Smart Edit completed via high-order logic.");
                Ok(summary)
            }
            Err(e) => {
                warn!(
                    "[CORTEX] ⚠️ Smart Edit pipeline failed: {}. Falling back to one-shot filter.",
                    e
                );
                // Fallback to the old filter-only method if the complex edit fails
                self.execute_one_shot_render(intent, input, output, visual_data, _audio_data)
                    .await
                    .map(|args| args.join(" "))
            }
        }
    }

    pub async fn execute_one_shot_render(
        &mut self,
        intent: &str,
        input: &Path,
        output: &Path,
        _visual_data: &[VisualScene],
        _audio_data: &AudioAnalysis,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
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

        // 2. Build FFmpeg Filtergraph (Video)
        if filters.is_empty() {
            // No video filters
        }

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
            audio_filters.push(
                "silenceremove=stop_periods=-1:stop_duration=1:stop_threshold=-40dB".to_string(),
            );
        }

        // Feature: Neural Audio Enhancement
        if intent_lower.contains("enhance")
            || intent_lower.contains("fix")
            || intent_lower.contains("voice")
            || intent_lower.contains("audio")
            || intent_lower.contains("louder")
        // User asked for louder voice
        {
            info!("[CORTEX] 🎙️ Enhancing Voice Clarity & Volume");
            // 1. Highpass to remove rumble
            audio_filters.push("highpass=f=100".to_string());
            // 2. Compressor to level out voice (makes it "louder" and consistent)
            audio_filters
                .push("acompressor=threshold=-12dB:ratio=4:attack=5:release=50".to_string());
            // 3. EQ Presense boost
            audio_filters.push("equalizer=f=3000:t=q:w=1:g=5".to_string());
            // 4. Loudness Normalization to standard -16 LUFS
            audio_filters.push("loudnorm=I=-16:TP=-1.5:LRA=11".to_string());
        }

        // Construct Final Command using Vec<String> to avoid shell injection and space issues
        let mut args = Vec::new();
        args.push("ffmpeg".to_string());
        args.push("-y".to_string());
        args.push("-i".to_string());
        args.push(input.to_string_lossy().to_string());

        if !filters.is_empty() {
            args.push("-vf".to_string());
            args.push(filters.join(","));
        }

        if !audio_filters.is_empty() {
            args.push("-af".to_string());
            args.push(audio_filters.join(","));
            args.push("-c:a".to_string());
            args.push("aac".to_string());
            args.push("-b:a".to_string());
            args.push("192k".to_string());
        } else {
            args.push("-c:a".to_string());
            args.push("copy".to_string());
        }

        args.push("-c:v".to_string());
        args.push("libx264".to_string());
        args.push("-preset".to_string());
        args.push("medium".to_string()); // Kept 'medium' from HEAD
        args.push("-crf".to_string());
        args.push("23".to_string()); // Kept '23' from HEAD
        args.push("-pix_fmt".to_string());
        args.push("yuv420p".to_string());

        args.push(output.to_string_lossy().to_string());

        Ok(args)
    }
}
