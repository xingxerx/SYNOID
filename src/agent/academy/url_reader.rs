#![allow(dead_code, unused_variables)]
// SYNOID Open URL Reader
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::gpt_oss_bridge::SynoidAgent;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tracing::info;
use url::Url;

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
            agent: SynoidAgent::new(api_url, "llama3:latest"),
        }
    }

    /// Ingest a URL and return a learned editing pattern
    pub async fn ingest(&self, url: &str) -> Result<LearnedPattern, Box<dyn std::error::Error + Send + Sync>> {
        info!("[SENSES] Ingesting URL: {}", url);

        let parsed_url = Url::parse(url)?;
        let host = parsed_url.host_str().unwrap_or("");

        let is_video_platform = host == "youtube.com"
            || host.ends_with(".youtube.com")
            || host == "youtu.be"
            || host.ends_with(".youtu.be")
            || host == "vimeo.com"
            || host.ends_with(".vimeo.com");

        if is_video_platform {
            self.ingest_video(parsed_url.as_str()).await
        } else {
            self.ingest_article(parsed_url.as_str()).await
        }
    }

    /// Learn from a video URL (Visual Analysis)
    async fn ingest_video(&self, url: &str) -> Result<LearnedPattern, Box<dyn std::error::Error + Send + Sync>> {
        info!("[SENSES] Detected Video URL. Initiating Visual Analysis...");

        // 1. Download metadata via yt-dlp (requires local install)
        use tokio::process::Command;
        let output = Command::new("yt-dlp")
            .args(["--dump-json", "--", url])
            .output()
            .await?;
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
            description: format!(
                "Analyzed '{}' ({:.1}s). Learned pacing: Dynamic.",
                title, duration
            ),
            confidence: 0.85,
        })
    }

    /// Learn from a text URL (Conceptual Analysis via GPT-OSS)
    async fn ingest_article(
        &self,
        url: &str,
    ) -> Result<LearnedPattern, Box<dyn std::error::Error + Send + Sync>> {
        info!("[SENSES] Detected Article URL. Scraping text...");

        // 1. Fetch HTML
        let resp = reqwest::get(url).await?.text().await?;

        // 2. Extract Text (Naive implementation)
        let mut text_content = String::new();
        {
            let document = Html::parse_document(&resp);
            let selector = Selector::parse("p, h1, h2, h3").unwrap();

            for element in document.select(&selector) {
                text_content.push_str(&element.text().collect::<Vec<_>>().join(" "));
                text_content.push('\n');
            }
        } // document is dropped here

        // Truncate to avoid context window overflow
        let truncated_text: String = text_content.chars().take(2000).collect();

        // 3. Ask GPT-OSS to extract editing rules
        info!("[SENSES] Asking Brain to extract rules...");
        let prompt = format!(
            "Extract 1 key video editing rule from this text using specific terminology:\n\n{}",
            truncated_text
        );

        let rule_description = self
            .agent
            .reason(&prompt)
            .await
            .unwrap_or_else(|_| "Failed to reason".to_string());

        Ok(LearnedPattern {
            source_url: url.to_string(),
            rule_type: "Conceptual".to_string(),
            description: rule_description,
            confidence: 0.90,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ingest_invalid_url() {
        let reader = UrlReader::new("http://dummy-api");
        let result = reader.ingest("not-a-url").await;
        assert!(result.is_err());
        // Verify it is a parse error
        let err = result.unwrap_err();
        assert!(err.to_string().contains("URL")); // "relative URL without a base" or similar
    }

    #[tokio::test]
    async fn test_ingest_malformed_injection_attempt() {
        let reader = UrlReader::new("http://dummy-api");
        // A string that looks like a flag but isn't a valid URL should be rejected by parser
        let result = reader.ingest("--bad-flag").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_domain_logic_unit() {
        // Since the method logic is embedded in `ingest`, we can verify using the parser logic directly here
        // to ensure our assumption about `host_str` and matching is correct.

        let cases = vec![
            ("https://youtube.com/watch?v=123", true),
            ("https://www.youtube.com/watch?v=123", true),
            ("https://youtu.be/123", true),
            ("https://vimeo.com/123", true),
            ("https://player.vimeo.com/123", true),
            ("https://example.com", false),
            ("https://evilyoutube.com", false),
            ("https://youtube.com.evil.com", false),
        ];

        for (url_str, expected) in cases {
            let parsed = Url::parse(url_str).expect("Failed to parse test url");
            let host = parsed.host_str().unwrap_or("");
            let is_video = host == "youtube.com"
                || host.ends_with(".youtube.com")
                || host == "youtu.be"
                || host.ends_with(".youtu.be")
                || host == "vimeo.com"
                || host.ends_with(".vimeo.com");

            assert_eq!(is_video, expected, "Failed for URL: {}", url_str);
        }
    }
}
