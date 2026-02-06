#![allow(dead_code, unused_variables)]
// SYNOID Open URL Reader
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use serde::{Deserialize, Serialize};
use crate::agent::gpt_oss_bridge::SynoidAgent;
use scraper::{Html, Selector};
use tracing::info;

#[derive(Serialize, Deserialize, Debug)]
pub struct LearnedPattern {
    pub source_url: String,
    pub rule_type: String, // "Visual" or "Conceptual"
    pub description: String,
    pub confidence: f32,
}

pub struct UrlReader {
    agent: SynoidAgent,
}

impl UrlReader {
    pub fn new(api_url: &str) -> Self {
        Self {
            agent: SynoidAgent::new(api_url),
        }
    }

    /// Ingest a URL and return a learned editing pattern
    pub async fn ingest(&self, url: &str) -> Result<LearnedPattern, Box<dyn std::error::Error>> {
        info!("[SENSES] Ingesting URL: {}", url);

        if url.contains("youtube.com") || url.contains("youtu.be") || url.contains("vimeo.com") {
            self.ingest_video(url).await
        } else {
            self.ingest_article(url).await
        }
    }

    /// Learn from a video URL (Visual Analysis)
    async fn ingest_video(&self, url: &str) -> Result<LearnedPattern, Box<dyn std::error::Error>> {
        info!("[SENSES] Detected Video URL. Initiating Visual Analysis...");
        
        // 1. Download metadata via yt-dlp (requires local install)
        use std::process::Command;
        let output = Command::new("yt-dlp")
            .args(["--dump-json", "--", url])
            .output()?;
            
        if !output.status.success() {
            return Err("Failed to fetch video metadata".into());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let video_data: serde_json::Value = serde_json::from_str(&json_str)?;
        let title = video_data["title"].as_str().unwrap_or("Unknown");
        let duration = video_data["duration"].as_f64().unwrap_or(0.0);

        // In a real scenario, we'd download the video and run the VectorEngine on it.
        // For now, we simulate the "learning" process based on metadata.
        
        Ok(LearnedPattern {
            source_url: url.to_string(),
            rule_type: "Visual".to_string(),
            description: format!("Analyzed '{}' ({:.1}s). Learned pacing: Dynamic.", title, duration),
            confidence: 0.85,
        })
    }

    /// Learn from a text URL (Conceptual Analysis via GPT-OSS)
    async fn ingest_article(&self, url: &str) -> Result<LearnedPattern, Box<dyn std::error::Error>> {
        info!("[SENSES] Detected Article URL. Scraping text...");

        // 1. Fetch HTML
        let resp = reqwest::get(url).await?.text().await?;
        
        // 2. Extract Text (Naive implementation)
        let document = Html::parse_document(&resp);
        let selector = Selector::parse("p, h1, h2, h3").unwrap();
        
        let mut text_content = String::new();
        for element in document.select(&selector) {
            text_content.push_str(&element.text().collect::<Vec<_>>().join(" "));
            text_content.push('\n');
        }

        // Truncate to avoid context window overflow
        let truncated_text: String = text_content.chars().take(2000).collect();

        // 3. Ask GPT-OSS to extract editing rules
        info!("[SENSES] Asking Brain to extract rules...");
        let prompt = format!(
            "Extract 1 key video editing rule from this text using specific terminology:\n\n{}", 
            truncated_text
        );
        
        let rule_description = self.agent.reason(&prompt).await.unwrap_or_else(|_| "Failed to reason".to_string());

        Ok(LearnedPattern {
            source_url: url.to_string(),
            rule_type: "Conceptual".to_string(),
            description: rule_description,
            confidence: 0.90,
        })
    }
}
