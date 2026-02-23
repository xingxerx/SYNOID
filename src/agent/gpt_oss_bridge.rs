// SYNOID MCP Server Bridge
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::multi_agent::NativeTimelineEngine;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, warn};

pub struct SynoidAgent {
    client: reqwest::Client,
    api_url: String,
    pub model: String,
}

impl SynoidAgent {
    pub fn new(api_url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url: api_url.to_string(),
            model: model.to_string(),
        }
    }

    pub async fn reason(&self, request: &str) -> Result<String, String> {
        info!("[AGENT] Reasoning with {}: {}", self.model, request);

        // Construct standard OpenAI-compatible Chat Completion request
        let payload = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are Synoid, an autonomous video production AI. Respond with concise JSON or direct commands."
                },
                {
                    "role": "user",
                    "content": request
                }
            ],
            "temperature": 0.7
        });

        let endpoint = format!("{}/chat/completions", self.api_url.trim_end_matches('/'));

        match self.client.post(&endpoint).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                    // Extract content from: choices[0].message.content
                    let content = json["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or("Error: Empty response")
                        .to_string();
                    Ok(content)
                } else {
                    Err(format!("API Error: {}", resp.status()))
                }
            }
            Err(e) => {
                warn!("[AGENT] 📡 LLM Connection Failed ({}), falling back to Offline Mode.", e);
                // Fallback for offline testing
                Ok(format!("(Offline Mode) Mock response for: {}", request))
            }
        }
    }
}


