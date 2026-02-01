// SYNOID™ Brain - Intent Classification & Heuristics
// Copyright (c) 2026 Xing_The_Creator | SYNOID™

use crate::agent::gpt_oss_bridge::SynoidAgent;
use tracing::info;

/// Intents that the Brain can classify
#[derive(Debug, PartialEq)]
pub enum Intent {
    DownloadYoutube { url: String },
    ScanVideo { path: String },
    LearnStyle { input: String, name: String },
    CreateEdit { input: String, instruction: String },
    Research { topic: String },
    Unknown { request: String },
}

/// The Central Brain of SYNOID
pub struct Brain {
    agent: Option<SynoidAgent>,
    api_url: String,
}

impl Brain {
    pub fn new(api_url: &str) -> Self {
        Self {
            agent: None,
            api_url: api_url.to_string(),
        }
    }

    /// Fast heuristic classification (energy efficient)
    /// Returns an Intent enum without calling the heavy LLM if possible.
    pub fn fast_classify(&self, request: &str) -> Intent {
        let req_lower = request.to_lowercase();
        
        // 1. YouTube Download Heuristics
        if (req_lower.contains("download") || req_lower.contains("get")) && 
           (req_lower.contains("youtube") || req_lower.contains("http")) {
            // Extract URL (simple extraction)
            if let Some(start) = request.find("http") {
                let rest = &request[start..];
                let end = rest.find(' ').unwrap_or(rest.len());
                return Intent::DownloadYoutube { url: rest[0..end].to_string() };
            }
        }

        // 2. Visual Scan Heuristics
        if req_lower.contains("scan") || req_lower.contains("analyze") {
             // If implicit video context or explicit mention
             return Intent::ScanVideo { path: "input.mp4".to_string() };
        }
        
        // 3. Learning Heuristics
        if req_lower.contains("learn") {
            return Intent::LearnStyle { 
                input: "input.mp4".to_string(), 
                name: "new_style".to_string() 
            };
        }

        // 4. Research Heuristics
        if req_lower.contains("find") || req_lower.contains("search") || req_lower.contains("tutorial") {
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

        Intent::Unknown { request: request.to_string() }
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
            },
            Intent::ScanVideo { path } => {
                info!("[BRAIN] ⚡ Fast-path activated: Visual Scan");
                // Activate Vision Tools ONLY
                use crate::agent::vision_tools;
                let path = std::path::Path::new(&path);
                match vision_tools::scan_visual(path).await {
                    Ok(scenes) => Ok(format!("Scanned {} scenes.", scenes.len())),
                    Err(e) => Err(format!("Scan failed: {}", e)),
                }
            },
            Intent::Research { topic } => {
                info!("[BRAIN] ⚡ Fast-path activated: Research Agent");
                use crate::agent::source_tools;
                match source_tools::search_youtube(&topic, 5).await {
                    Ok(results) => {
                        let mut response = format!("Found {} resources for '{}':\n", results.len(), topic);
                        for (i, r) in results.iter().enumerate() {
                            response.push_str(&format!("{}. {} ({})\n", i+1, r.title, r.original_url.as_deref().unwrap_or("?")));
                        }
                        Ok(response)
                    },
                    Err(e) => Err(format!("Research failed: {}", e)),
                }
            },
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
            },
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
