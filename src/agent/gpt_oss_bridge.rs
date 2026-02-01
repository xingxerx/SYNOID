// SYNOID™ GPT-OSS Bridge
// Copyright (c) 2026 Xing_The_Creator | SYNOID™

use serde::{Deserialize, Serialize};
use reqwest::Client;
use tracing::info;

#[derive(Serialize, Deserialize, Debug)]
struct CompletionRequest {
    model: String,
    prompt: String,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct CompletionResponse {
    choices: Vec<CompletionChoice>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CompletionChoice {
    text: String,
}

#[derive(Clone)]
pub struct SynoidAgent {
    client: Client,
    api_url: String,
    model: String,
}

impl SynoidAgent {
    pub fn new(api_url: &str) -> Self {
        Self {
            client: Client::new(),
            api_url: api_url.to_string(),
            model: std::env::var("SYNOID_MODEL").unwrap_or("gpt-oss:20b".to_string()),
        }
    }

    pub async fn reason(&self, prompt: &str) -> Result<String, String> {
        info!("[CORTEX] Reasoning on: '{}'...", prompt.chars().take(50).collect::<String>());
        
        // This is a simplified implementation assuming an OpenAI-compatible /completions endpoint
        // or a similar local inference server (e.g. llama.cpp, vllm)
        let req = CompletionRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            max_tokens: 512,
            temperature: 0.7,
        };

        let res = self.client.post(&format!("{}/completions", self.api_url))
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
            
        if !res.status().is_success() {
            return Err(format!("API Error: {}", res.status()));
        }

        let body: CompletionResponse = res.json().await
            .map_err(|e| format!("Parse failed: {}", e))?;

        if let Some(choice) = body.choices.first() {
            Ok(choice.text.trim().to_string())
        } else {
            Err("No completion choices returned".to_string())
        }
    }
}
