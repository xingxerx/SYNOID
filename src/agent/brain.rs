// SYNOID Brain - Intent Classification & Heuristics
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::body::Body;
use crate::agent::consciousness::Consciousness;
use crate::agent::gpt_oss_bridge::SynoidAgent;
use tracing::info;

/// Intents that the Brain can classify
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum Intent {
    DownloadYoutube { url: String },
    ScanVideo { path: String },
    LearnStyle { input: String, name: String },
    CreateEdit { input: String, instruction: String },
    Research { topic: String },
    Vectorize { input: String, preset: String },
    Upscale { input: String, scale: f64 },
    VoiceClone { input: String, name: String },
    Speak { text: String, profile: String },
    Unknown { request: String },
}

use crate::agent::learning::LearningKernel;

/// The Central Brain of SYNOID
pub struct Brain {
    agent: Option<SynoidAgent>,
    api_url: String,
    learning_kernel: LearningKernel,
    // Integrated components (silences unused warnings)
    _consciousness: Consciousness,
    _body: Body,
}

impl Brain {
    pub fn new(api_url: &str) -> Self {
        Self {
            agent: None,
            api_url: api_url.to_string(),
            learning_kernel: LearningKernel::new(),
            _consciousness: Consciousness::new(),
            _body: Body::new(),
        }
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
            // If implicit video context or explicit mention
            return Intent::ScanVideo {
                path: "input.mp4".to_string(),
            };
        }

        // 3. Learning Heuristics
        if req_lower.contains("learn") {
            return Intent::LearnStyle {
                input: "input.mp4".to_string(),
                name: "new_style".to_string(),
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
            let input = "input.mp4".to_string(); 
            return Intent::Vectorize { 
                input,
                preset: "default".to_string() 
            };
        }

        if req_lower.contains("upscale") || req_lower.contains("enhance") {
            let scale = if req_lower.contains("4x") { 4.0 } else { 2.0 };
            return Intent::Upscale {
                input: "input.mp4".to_string(),
                scale,
            };
        }

        // 6. Voice Engine Heuristics
        if req_lower.contains("clone voice") || (req_lower.contains("voice") && req_lower.contains("learn")) {
            return Intent::VoiceClone {
                input: "sample.wav".to_string(),
                name: "cloned_voice".to_string(),
            };
        }
        
        if req_lower.contains("say") || req_lower.contains("speak") {
             let text = if let Some(idx) = req_lower.find("say") {
                 request[idx+3..].trim().to_string()
             } else if let Some(idx) = req_lower.find("speak") {
                 request[idx+5..].trim().to_string()
             } else {
                 "Hello".to_string()
             };
             
            return Intent::Speak {
                text,
                profile: "default".to_string(),
            };
        }

        Intent::Unknown {
            request: request.to_string(),
        }
    }

    /// Process a request through the Brain
    pub async fn process(&mut self, request: &str) -> Result<String, String> {
        let intent = self.fast_classify(request);

        match intent {
            Intent::DownloadYoutube { url } => {
                info!("[BRAIN] ⚡ Fast-path activated: YouTube Download");
                // Activate Source Tools ONLY
                use crate::agent::source_tools;
                let output_dir = std::path::Path::new("downloads");
                // Brain fast-path doesn't support auth yet
                match source_tools::download_youtube(&url, output_dir, None).await {
                    Ok(info) => Ok(format!("Downloaded: {}", info.title)),
                    Err(e) => Err(format!("Download failed: {}", e)),
                }
            }
            Intent::ScanVideo { path } => {
                info!("[BRAIN] ⚡ Fast-path activated: Visual Scan");
                // Activate Vision Tools ONLY
                use crate::agent::vision_tools;
                let path = std::path::Path::new(&path);
                match vision_tools::scan_visual(path).await {
                    Ok(scenes) => Ok(format!("Scanned {} scenes.", scenes.len())),
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
                             return Err("Video too short or no scenes detected to learn from.".to_string());
                        }
                        
                        // Calculate average scene duration
                        let total_duration = scenes.last().unwrap().timestamp - scenes.first().unwrap().timestamp;
                        let avg_duration = if scenes.len() > 1 {
                             total_duration / (scenes.len() as f64 - 1.0)
                        } else {
                            total_duration
                        };
                        
                        info!("[BRAIN] Extracted style metrics: Avg Scene Duration = {:.2}s", avg_duration);

                        // 2. Create and Store Pattern
                        let pattern = crate::agent::learning::EditingPattern {
                            intent_tag: name.clone(),
                            avg_scene_duration: avg_duration,
                            transition_speed: 1.0, // Default for now, could be inferred
                            music_sync_strictness: 0.8, // Assume high sync for learned styles
                            color_grade_style: "learned".to_string(),
                            success_rating: 5, // User explicitly asked to learn this, so we rate it high
                        };
                        
                        self.learning_kernel.memorize(&name, pattern);
                        Ok(format!("Learned new style '{}' with average scene duration of {:.2}s", name, avg_duration))
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
            Intent::Unknown { request } => {
                info!("[BRAIN] 🧠 Complex request detected. Waking up Cortex (GPT-OSS)...");
                // Lazy-load the heavy AI agent only now
                if self.agent.is_none() {
                    self.agent = Some(SynoidAgent::new(&self.api_url));
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
            _ => Ok("Intent recognized but handler not implemented yet.".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_classify_youtube() {
        let brain = Brain::new("http://localhost");
        let intent = brain.fast_classify("Download this video https://youtube.com/watch?v=123");
        match intent {
            Intent::DownloadYoutube { url } => assert!(url.contains("youtube.com")),
            _ => panic!("Failed to classify youtube download"),
        }
    }

    #[test]
    fn test_fast_classify_scan() {
        let brain = Brain::new("http://localhost");
        let intent = brain.fast_classify("scan this file");
        match intent {
            Intent::ScanVideo { .. } => assert!(true),
            _ => panic!("Failed to classify video scan"),
        }
    }
}
