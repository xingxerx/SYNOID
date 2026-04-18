// SYNOID LLM Bridge — Ollama-only (sovereign, fully in-house)
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// All reasoning, fast-request, and vision tasks route through the local
// Ollama server.  No cloud providers (Groq / Google) are used.

use crate::agent::ai_systems::token_optimizer::TokenOptimizer;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, warn};

/// Single provider — always Ollama.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmProvider {
    Ollama,
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ollama")
    }
}

/// Configuration for the local Ollama server.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Ollama API URL (default: http://localhost:11434)
    pub ollama_url: String,
    /// Primary reasoning/text model (default: gemma4:26b)
    pub ollama_model: String,
    /// Vision/multimodal model (default: llava:latest)
    pub ollama_vision_model: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            ollama_url: std::env::var("SYNOID_API_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            ollama_model: std::env::var("SYNOID_MODEL")
                .unwrap_or_else(|_| "gemma4:26b".to_string()),
            ollama_vision_model: std::env::var("SYNOID_VISION_MODEL")
                .unwrap_or_else(|_| "llava:latest".to_string()),
        }
    }
}

impl ProviderConfig {
    pub fn from_env() -> Self {
        Self::default()
    }

    pub fn available_providers(&self) -> Vec<LlmProvider> {
        vec![LlmProvider::Ollama]
    }
}

/// LLM client routing everything through the local Ollama server.
#[derive(Debug)]
pub struct MultiProviderLlm {
    client: reqwest::Client,
    pub config: ProviderConfig,
    pub optimizer: Arc<TokenOptimizer>,
}

impl MultiProviderLlm {
    pub fn new(config: ProviderConfig, optimizer: Arc<TokenOptimizer>) -> Self {
        info!("[LLM] Initialized — Ollama-only mode ({})", config.ollama_url);
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
            config,
            optimizer,
        }
    }

    /// Send a reasoning request through Ollama.
    pub async fn reason(&self, request: &str) -> Result<String, String> {
        self.call_ollama(request).await.and_then(|r| {
            if r.starts_with("(Offline Mode)") {
                Err("Ollama unavailable".into())
            } else {
                Ok(r)
            }
        })
    }

    /// Send a fast/simple request through Ollama.
    pub async fn fast_request(&self, request: &str) -> Result<String, String> {
        self.reason(request).await
    }

    /// Send a vision request through the local Ollama VLM.
    pub async fn vision_request(&self, prompt: &str, image_b64: &str) -> Result<String, String> {
        self.call_ollama_vision(prompt, image_b64).await
    }

    /// Audio transcription — use local Whisper (not yet wired) or Faster-Whisper CLI.
    pub async fn audio_transcription(
        &self,
        _audio_path: &std::path::Path,
    ) -> Result<String, String> {
        Err("Use local Whisper/Faster-Whisper for transcription (set WHISPER_BIN env var)".into())
    }

    pub fn token_status(&self) -> String {
        self.optimizer.display_status()
    }

    // ─── Ollama Implementations ───────────────────────────────────────────────

    async fn call_ollama(&self, request: &str) -> Result<String, String> {
        info!("[LLM] Ollama → {} @ {}", self.config.ollama_model, self.config.ollama_url);

        let base = self
            .config
            .ollama_url
            .trim_end_matches('/')
            .trim_end_matches("/v1");

        let full_prompt = format!(
            "You are Synoid, an autonomous video production AI. Respond with concise JSON or direct commands.\n\n{}",
            request
        );

        let payload = json!({
            "model": self.config.ollama_model,
            "prompt": full_prompt,
            "stream": false,
            "options": { "temperature": 0.7 }
        });

        match self
            .client
            .post(format!("{}/api/generate", base))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                Ok(json["response"]
                    .as_str()
                    .unwrap_or("Error: Empty response")
                    .to_string())
            }
            Ok(resp) => Err(format!("Ollama API error: {}", resp.status())),
            Err(e) => {
                warn!("[LLM] Ollama unreachable ({}), entering offline mode", e);
                Ok(format!("(Offline Mode) Mock response for: {}", request))
            }
        }
    }

    async fn call_ollama_vision(&self, prompt: &str, image_b64: &str) -> Result<String, String> {
        let base = self
            .config
            .ollama_url
            .trim_end_matches('/')
            .trim_end_matches("/v1");

        let body = json!({
            "model": self.config.ollama_vision_model,
            "prompt": prompt,
            "images": [image_b64],
            "stream": false
        });

        match self
            .client
            .post(format!("{}/api/generate", base))
            .json(&body)
            .timeout(std::time::Duration::from_secs(90))
            .send()
            .await
        {
            Ok(resp) => {
                let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                Ok(json["response"].as_str().unwrap_or("").to_string())
            }
            Err(e) => {
                warn!("[LLM] Ollama VLM unavailable: {}", e);
                Ok(String::new())
            }
        }
    }
}

/// Rough token estimate: ~4 chars per token for English text.
#[allow(dead_code)]
fn estimate_tokens(text: &str) -> u64 {
    (text.len() as u64) / 4
}
