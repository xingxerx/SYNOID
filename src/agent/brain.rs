// SYNOID Brain - Intent Classification & Heuristics
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Connected to: Neuroplasticity (adaptive speed) + GPU/CUDA backend
// The Brain accelerates with experience and uses GPU when available.

use crate::agent::body::Body;
use crate::agent::consciousness::Consciousness;
use crate::agent::gpt_oss_bridge::SynoidAgent;
use crate::gpu_backend::GpuContext;
use tracing::info;

/// Intents that the Brain can classify
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum Intent {
    DownloadYoutube {
        url: String,
    },
    ScanVideo {
        path: String,
    },
    LearnStyle {
        input: String,
        name: String,
    },
    CreateEdit {
        input: String,
        instruction: String,
    },
    Research {
        topic: String,
    },
    Vectorize {
        input: String,
        preset: String,
    },
    Upscale {
        input: String,
        scale: f64,
    },
    VoiceClone {
        input: String,
        name: String,
    },
    Speak {
        text: String,
        profile: String,
    },
    /// Complex creative request requiring MoE orchestration
    Orchestrate {
        goal: String,
        input_path: Option<String>,
    },
    Unknown {
        request: String,
    },
}

use crate::agent::learning::LearningKernel;

/// The Central Brain of SYNOID
///
/// Connected to:
/// - **Neuroplasticity**: Adaptive speed system that doubles processing
///   speed at experience thresholds (1×→16×).
/// - **GpuContext**: CUDA/NVENC backend for hardware-accelerated encoding.
///   The neuroplasticity multiplier tunes GPU batch sizes and FFmpeg presets.
pub struct Brain {
    agent: Option<SynoidAgent>,
    api_url: String,
    model: String,
    pub learning_kernel: LearningKernel,
    pub neuroplasticity: crate::agent::neuroplasticity::Neuroplasticity,
    /// GPU/CUDA backend reference (late-bound after async detection).
    /// Note: Uses 'static lifetime because GpuContext is a global singleton (OnceLock).
    gpu: Option<&'static GpuContext>,
    // Integrated components (silences unused warnings)
    _consciousness: Consciousness,
    _body: Body,
}

impl Brain {
    pub fn new(api_url: &str, model: &str) -> Self {
        Self {
            agent: None,
            api_url: api_url.to_string(),
            model: model.to_string(),
            learning_kernel: LearningKernel::new(),
            neuroplasticity: crate::agent::neuroplasticity::Neuroplasticity::new(),
            gpu: None,
            _consciousness: Consciousness::new(),
            _body: Body::new(),
        }
    }

    /// Late-bind the GPU context after async detection completes.
    pub fn connect_gpu(&mut self, gpu: &'static GpuContext) {
        self.gpu = Some(gpu);
        let accel = gpu.cuda_accel_config(self.neuroplasticity.current_speed());
        info!(
            "[BRAIN] 🔗 GPU Connected: {} | Neural CUDA config: batch={}, streams={}, preset={}",
            gpu.backend, accel.batch_size, accel.parallel_streams, accel.ffmpeg_preset
        );
    }

    /// Combined acceleration status: neuroplasticity + GPU.
    pub fn acceleration_status(&self) -> String {
        let neuro = &self.neuroplasticity;
        let gpu_str = match &self.gpu {
            Some(g) => format!("{}", g.backend),
            None => "Not connected".to_string(),
        };
        format!(
            "Brain {:.1}× [{}] | GPU: {} | Batch: {}",
            neuro.current_speed(),
            neuro.adaptation_level(),
            gpu_str,
            neuro.gpu_batch_multiplier(),
        )
    }

    /// Fast heuristic classification (energy efficient)
    /// Returns an Intent enum without calling the heavy LLM if possible.
    pub fn fast_classify(&self, request: &str) -> Intent {
        let req_lower = request.to_lowercase();

        // 1. YouTube Download Heuristics
        if (req_lower.contains("download") || req_lower.contains("get"))
            && (req_lower.contains("youtube") || req_lower.contains("http"))
        {
            // Extract URL (simple extraction)
            if let Some(start) = request.find("http") {
                let rest = &request[start..];
                let end = rest.find(' ').unwrap_or(rest.len());
                return Intent::DownloadYoutube {
                    url: rest[0..end].to_string(),
                };
            }
        }

        // 2. Visual Scan Heuristics
        if req_lower.contains("scan") || req_lower.contains("analyze") {
            return Intent::ScanVideo {
                path: Self::extract_path(request).unwrap_or_else(|| "input.mp4".to_string()),
            };
        }

        // 3. Learning Heuristics
        if req_lower.contains("learn") {
            let name = Self::extract_quoted_value(request, "style")
                .unwrap_or_else(|| "new_style".to_string());
            return Intent::LearnStyle {
                input: Self::extract_path(request).unwrap_or_else(|| "input.mp4".to_string()),
                name,
            };
        }

        // 4. Research Heuristics
        if req_lower.contains("find")
            || req_lower.contains("search")
            || req_lower.contains("tutorial")
        {
            // Simple topic extraction: everything after key verb
            let keys = ["find", "search for", "tutorial on", "about"];
            for key in keys {
                if let Some(idx) = req_lower.find(key) {
                    let topic = request[idx + key.len()..].trim().to_string();
                    if !topic.is_empty() {
                        return Intent::Research { topic };
                    }
                }
            }
        }

        // 5. Vector Engine Heuristics
        if req_lower.contains("vector") || req_lower.contains("svg") {
            return Intent::Vectorize {
                input: Self::extract_path(request).unwrap_or_else(|| "input.mp4".to_string()),
                preset: "default".to_string(),
            };
        }

        if req_lower.contains("upscale") || req_lower.contains("enhance") {
            let scale = if req_lower.contains("4x") { 4.0 } else { 2.0 };
            return Intent::Upscale {
                input: Self::extract_path(request).unwrap_or_else(|| "input.mp4".to_string()),
                scale,
            };
        }

        // 6. Voice Engine Heuristics
        if req_lower.contains("clone voice")
            || (req_lower.contains("voice") && req_lower.contains("learn"))
        {
            return Intent::VoiceClone {
                input: "sample.wav".to_string(),
                name: "cloned_voice".to_string(),
            };
        }

        if req_lower.contains("say") || req_lower.contains("speak") {
            let text = if let Some(idx) = req_lower.find("say") {
                request[idx + 3..].trim().to_string()
            } else if let Some(idx) = req_lower.find("speak") {
                request[idx + 5..].trim().to_string()
            } else {
                "Hello".to_string()
            };

            return Intent::Speak {
                text,
                profile: "default".to_string(),
            };
        }

        // 7. Orchestrate Heuristics (MoE Dispatcher)
        // Complex creative requests requiring multi-expert coordination
        let orchestrate_verbs = [
            "create",
            "make",
            "produce",
            "build",
            "generate",
            "edit",
            "transform",
            "cut",
            "trim",
        ];
        let creative_nouns = [
            "video",
            "movie",
            "trailer",
            "montage",
            "highlight",
            "reel",
            "content",
            "clip",
        ];

        let has_verb = orchestrate_verbs.iter().any(|v| req_lower.contains(v));
        let has_noun = creative_nouns.iter().any(|n| req_lower.contains(n));

        if has_verb && has_noun {
            return Intent::Orchestrate {
                goal: request.to_string(),
                input_path: Self::extract_path(request),
            };
        }

        Intent::Unknown {
            request: request.to_string(),
        }
    }

    /// Extract a file path from a request string.
    /// Looks for quoted paths first, then common file extensions.
    fn extract_path(request: &str) -> Option<String> {
        // 1. Try to find a quoted path (e.g. "cortex_cache\video.mp4")
        for quote in ['"', '\''] {
            let mut chars = request.char_indices().peekable();
            while let Some((start_idx, ch)) = chars.next() {
                if ch == quote {
                    let content_start = start_idx + 1;
                    while let Some((end_idx, ch2)) = chars.next() {
                        if ch2 == quote {
                            let candidate = &request[content_start..end_idx];
                            if Self::looks_like_path(candidate) {
                                return Some(candidate.to_string());
                            }
                            break;
                        }
                    }
                }
            }
        }

        // 2. Try to find an unquoted path by looking for file extensions
        for word in request.split_whitespace() {
            // Strip surrounding punctuation
            let clean = word.trim_matches(|c: char| c == ',' || c == ';' || c == ')' || c == '(');
            if Self::looks_like_path(clean) {
                return Some(clean.to_string());
            }
        }

        None
    }

    /// Check if a string looks like a file path.
    fn looks_like_path(s: &str) -> bool {
        let extensions = [
            ".mp4", ".mkv", ".mov", ".avi", ".webm", ".wav", ".mp3", ".flac", ".svg",
        ];
        let s_lower = s.to_lowercase();
        extensions.iter().any(|ext| s_lower.ends_with(ext))
            || s.contains(std::path::MAIN_SEPARATOR)
            || s.contains('/')
    }

    /// Extract a value after a keyword, possibly quoted.
    /// e.g. for "learn style 'cinematic' from video" with key="style" → "cinematic"
    fn extract_quoted_value(request: &str, key: &str) -> Option<String> {
        let lower = request.to_lowercase();
        if let Some(idx) = lower.find(key) {
            let after = &request[idx + key.len()..];
            let trimmed = after.trim_start();
            // Check for quoted value
            if trimmed.starts_with('\'') || trimmed.starts_with('"') {
                let quote = trimmed.chars().next().unwrap();
                if let Some(end) = trimmed[1..].find(quote) {
                    return Some(trimmed[1..1 + end].to_string());
                }
            }
            // Unquoted: take first word
            let word = trimmed.split_whitespace().next()?;
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if !clean.is_empty() {
                return Some(clean.to_string());
            }
        }
        None
    }

    /// Process a request through the Brain
    ///
    /// Uses neuroplasticity-tuned parameters and GPU acceleration when
    /// available to speed up processing.
    pub async fn process(&mut self, request: &str) -> Result<String, String> {
        let intent = self.fast_classify(request);

        // Log combined acceleration status before dispatching
        info!("[BRAIN] Acceleration: {}", self.acceleration_status());

        match intent {
            Intent::DownloadYoutube { url } => {
                info!("[BRAIN] ⚡ Fast-path activated: YouTube Download");
                // Activate Source Tools ONLY
                use crate::agent::source_tools;
                let output_dir = std::path::Path::new("downloads");
                // Brain fast-path doesn't support auth yet
                match source_tools::download_youtube(&url, output_dir, None).await {
                    Ok(info) => {
                        self.neuroplasticity.record_success();
                        Ok(format!("Downloaded: {}", info.title))
                    }
                    Err(e) => Err(format!("Download failed: {}", e)),
                }
            }
            Intent::ScanVideo { path } => {
                info!("[BRAIN] ⚡ Fast-path activated: Visual Scan");
                // Activate Vision Tools ONLY
                use crate::agent::vision_tools;
                let path = std::path::Path::new(&path);
                match vision_tools::scan_visual(path).await {
                    Ok(scenes) => {
                        self.neuroplasticity.record_success();
                        Ok(format!("Scanned {} scenes.", scenes.len()))
                    }
                    Err(e) => Err(format!("Scan failed: {}", e)),
                }
            }
            Intent::LearnStyle { input, name } => {
                info!("[BRAIN] 🧠 Learning style '{}' from video...", name);
                use crate::agent::vision_tools;
                let path = std::path::Path::new(&input);

                // 1. Analyze the video to extract style metrics
                match vision_tools::scan_visual(path).await {
                    Ok(scenes) => {
                        if scenes.len() < 2 {
                            return Err(
                                "Video too short or no scenes detected to learn from.".to_string()
                            );
                        }

                        // Calculate average scene duration
                        let total_duration =
                            scenes.last().unwrap().timestamp - scenes.first().unwrap().timestamp;
                        let avg_duration = if scenes.len() > 1 {
                            total_duration / (scenes.len() as f64 - 1.0)
                        } else {
                            total_duration
                        };

                        info!(
                            "[BRAIN] Extracted style metrics: Avg Scene Duration = {:.2}s",
                            avg_duration
                        );

                        // 2. Create and Store Pattern
                        let pattern = crate::agent::learning::EditingPattern {
                            intent_tag: name.clone(),
                            avg_scene_duration: avg_duration,
                            transition_speed: 1.0, // Default for now, could be inferred
                            music_sync_strictness: 0.8, // Assume high sync for learned styles
                            color_grade_style: "learned".to_string(),
                            success_rating: 5, // User explicitly asked to learn this, so we rate it high
                        };

                        // Removed extra closing brace

                        self.learning_kernel.memorize(&name, pattern);
                        self.neuroplasticity.record_success();
                        Ok(format!(
                            "Learned new style '{}' with average scene duration of {:.2}s",
                            name, avg_duration
                        ))
                    }
                    Err(e) => Err(format!("Failed to analyze video for learning: {}", e)),
                }
            }
            Intent::Research { topic } => {
                info!("[BRAIN] ⚡ Fast-path activated: Research Agent");
                use crate::agent::source_tools;
                match source_tools::search_youtube(&topic, 5).await {
                    Ok(results) => {
                        let mut response =
                            format!("Found {} resources for '{}':\n", results.len(), topic);
                        for (i, r) in results.iter().enumerate() {
                            response.push_str(&format!(
                                "{}. {} ({})\n",
                                i + 1,
                                r.title,
                                r.original_url.as_deref().unwrap_or("?")
                            ));
                        }
                        Ok(response)
                    }
                    Err(e) => Err(format!("Research failed: {}", e)),
                }
            }
            Intent::Vectorize { input, preset: _ } => {
                info!("[BRAIN] 🎨 Activating Vector Engine...");
                use crate::agent::vector_engine::{self, VectorConfig};
                let input_path = std::path::Path::new(&input);
                let output_path = input_path.with_file_name(format!(
                    "{}_vectorized",
                    input_path.file_stem().unwrap().to_string_lossy()
                ));

                let config = VectorConfig::default();

                match vector_engine::vectorize_video(input_path, &output_path, config).await {
                    Ok(msg) => {
                        self.neuroplasticity.record_success();
                        Ok(format!("Vectorization complete: {}", msg))
                    }
                    Err(e) => Err(format!("Vectorization failed: {}", e)),
                }
            }
            Intent::Upscale { input, scale } => {
                info!("[BRAIN] 🔎 Activating Infinite Upscale ({}x)...", scale);
                use crate::agent::vector_engine;
                let input_path = std::path::Path::new(&input);
                let stem = input_path.file_stem().unwrap().to_string_lossy();
                let output_path = input_path.with_file_name(format!("{}_upscaled.mp4", stem));

                match vector_engine::upscale_video(input_path, scale, &output_path).await {
                    Ok(msg) => {
                        self.neuroplasticity.record_success();
                        Ok(format!("Upscale complete: {}", msg))
                    }
                    Err(e) => Err(format!("Upscale failed: {}", e)),
                }
            }
            Intent::VoiceClone { .. } | Intent::Speak { .. } => Err(
                "Voice operations require access to the VoiceEngine. Please use the 'voice' CLI command.".to_string(),
            ),
            Intent::Orchestrate { goal, .. } => {
                 info!("[BRAIN] 🎼 Orchestrating creative goal: {}", goal);
                 // Use the LLM to reason about the orchestration
                 if self.agent.is_none() {
                    self.agent = Some(SynoidAgent::new(&self.api_url, &self.model));
                 }
                 if let Some(agent) = &self.agent {
                    match agent.reason(&goal).await {
                        Ok(resp) => Ok(format!("Orchestration Plan: {}", resp)),
                        Err(e) => Err(format!("Orchestration failed: {}", e)),
                    }
                 } else {
                    Err("Failed to initialize Cortex for orchestration".to_string())
                 }
            }
            Intent::CreateEdit { input, instruction } => {
                // Similar to Orchestrate but simpler
                 info!("[BRAIN] 🎬 Planning edit for {}: {}", input, instruction);
                 if self.agent.is_none() {
                    self.agent = Some(SynoidAgent::new(&self.api_url, &self.model));
                 }
                 if let Some(agent) = &self.agent {
                    match agent.reason(&instruction).await {
                        Ok(resp) => Ok(format!("Edit Plan: {}", resp)),
                        Err(e) => Err(format!("Planning failed: {}", e)),
                    }
                 } else {
                     Err("Failed to initialize Cortex".to_string())
                 }
            }
            Intent::Unknown { request } => {
                info!("[BRAIN] 🧠 Complex request detected. Waking up Cortex (GPT-OSS)...");
                // Lazy-load the heavy AI agent only now
                if self.agent.is_none() {
                    self.agent = Some(SynoidAgent::new(&self.api_url, &self.model));
                }

                if let Some(agent) = &self.agent {
                    // Simple passthrough for now - in reality this would call tool use
                    match agent.reason(&request).await {
                        Ok(resp) => Ok(format!("Cortex reasoned: {}", resp)),
                        Err(e) => Err(format!("Cortex failed: {}", e)),
                    }
                } else {
                    Err("Failed to initialize Cortex".to_string())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_classify_youtube() {
        let brain = Brain::new("http://localhost", "mock-model");
        let intent = brain.fast_classify("Download this video https://youtube.com/watch?v=123");
        match intent {
            Intent::DownloadYoutube { url } => assert!(url.contains("youtube.com")),
            _ => panic!("Failed to classify youtube download"),
        }
    }

    #[test]
    fn test_fast_classify_scan() {
        let brain = Brain::new("http://localhost", "mock-model");
        let intent = brain.fast_classify("scan this file");
        match intent {
            Intent::ScanVideo { .. } => assert!(true),
            _ => panic!("Failed to classify video scan"),
        }
    }
}
