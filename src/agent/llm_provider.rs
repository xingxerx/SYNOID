// SYNOID Multi-Provider LLM Bridge
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Routes LLM requests to the best available provider:
//   - Groq (primary): Reasoning + Fast tasks via OpenAI-compatible API
//   - Google AI Studio: Vision/multimodal tasks via Gemini API
//   - Ollama (fallback): Local models when cloud providers are unavailable
//
// Integrated with TokenOptimizer to respect free-tier rate limits.

use crate::agent::token_optimizer::TokenOptimizer;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, warn};

/// Which provider to route a request to.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmProvider {
    /// Groq Cloud (OpenAI-compatible, fast inference)
    Groq,
    /// Groq Cloud with a smaller/faster model
    GroqFast,
    /// Google AI Studio (Gemini, vision-capable)
    GoogleVision,
    /// Local Ollama server (fallback)
    Ollama,
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Groq => write!(f, "Groq"),
            Self::GroqFast => write!(f, "Groq-Fast"),
            Self::GoogleVision => write!(f, "Google-Vision"),
            Self::Ollama => write!(f, "Ollama"),
        }
    }
}

/// Configuration for all LLM providers.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Groq API key (from GROQ_API_KEY env var)
    pub groq_api_key: Option<String>,
    /// Groq reasoning model (default: llama-3.3-70b-versatile)
    pub groq_reasoning_model: String,
    /// Groq fast model (default: llama-3.1-8b-instant)
    pub groq_fast_model: String,
    /// Google AI Studio API key (from GOOGLE_AI_KEY env var)
    pub google_ai_key: Option<String>,
    /// Google vision model (default: gemini-1.5-flash for best free tier quota)
    pub google_vision_model: String,
    /// Ollama API URL (fallback)
    pub ollama_url: String,
    /// Ollama model
    pub ollama_model: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            groq_api_key: std::env::var("GROQ_API_KEY").ok(),
            groq_reasoning_model: std::env::var("GROQ_REASONING_MODEL")
                .unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string()),
            groq_fast_model: std::env::var("GROQ_FAST_MODEL")
                .unwrap_or_else(|_| "llama-3.1-8b-instant".to_string()),
            google_ai_key: std::env::var("GOOGLE_AI_KEY").ok(),
            google_vision_model: std::env::var("GOOGLE_VISION_MODEL")
                .unwrap_or_else(|_| "gemini-1.5-flash".to_string()),
            ollama_url: std::env::var("SYNOID_API_URL")
                .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
            ollama_model: "llama3:latest".to_string(),
        }
    }
}

impl ProviderConfig {
    /// Check which cloud providers are configured with API keys.
    pub fn available_providers(&self) -> Vec<LlmProvider> {
        let mut providers = Vec::new();
        if self.groq_api_key.is_some() {
            providers.push(LlmProvider::Groq);
            providers.push(LlmProvider::GroqFast);
        }
        if self.google_ai_key.is_some() {
            providers.push(LlmProvider::GoogleVision);
        }
        providers.push(LlmProvider::Ollama); // Always available as fallback
        providers
    }
}

/// Multi-provider LLM client with automatic routing and token optimization.
pub struct MultiProviderLlm {
    client: reqwest::Client,
    pub config: ProviderConfig,
    pub optimizer: Arc<TokenOptimizer>,
}

impl MultiProviderLlm {
    pub fn new(config: ProviderConfig, optimizer: Arc<TokenOptimizer>) -> Self {
        let available = config.available_providers();
        info!(
            "[LLM] Multi-provider initialized: {:?}",
            available.iter().map(|p| p.to_string()).collect::<Vec<_>>()
        );

        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
            config,
            optimizer,
        }
    }

    /// Send a reasoning request (text-only) to the best available provider.
    /// Priority: Groq → Ollama
    pub async fn reason(&self, request: &str) -> Result<String, String> {
        let est_tokens = estimate_tokens(request) + 500;

        if self.config.groq_api_key.is_some() && self.optimizer.can_use("groq", est_tokens) {
            match self
                .call_groq(request, &self.config.groq_reasoning_model)
                .await
            {
                Ok((response, tokens)) => {
                    self.optimizer.record("groq", tokens);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("[LLM] Groq reasoning failed: {}, falling back", e);
                }
            }
        }

        self.call_ollama(request).await
    }

    /// Send a fast/simple request (JSON parsing, classification).
    /// Priority: Groq Fast → Groq → Ollama
    pub async fn fast_request(&self, request: &str) -> Result<String, String> {
        let est_tokens = estimate_tokens(request) + 300;

        if self.config.groq_api_key.is_some() && self.optimizer.can_use("groq_fast", est_tokens) {
            match self.call_groq(request, &self.config.groq_fast_model).await {
                Ok((response, tokens)) => {
                    self.optimizer.record("groq_fast", tokens);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("[LLM] Groq fast failed: {}, falling back", e);
                }
            }
        }

        self.reason(request).await
    }

    /// Send a vision request (frame analysis) to Google AI Studio.
    /// Priority: Google AI Studio → Ollama VLM
    pub async fn vision_request(&self, prompt: &str, image_b64: &str) -> Result<String, String> {
        let est_tokens = estimate_tokens(prompt) + 1000;

        if self.config.google_ai_key.is_some()
            && self.optimizer.can_use("google_vision", est_tokens)
        {
            match self.call_google_vision(prompt, image_b64).await {
                Ok((response, tokens)) => {
                    self.optimizer.record("google_vision", tokens);
                    return Ok(response);
                }
                Err(e) => {
                    warn!(
                        "[LLM] Google Vision failed: {}, falling back to Ollama VLM",
                        e
                    );
                }
            }
        }

        self.call_ollama_vision(prompt, image_b64).await
    }

    /// Audio Transcription (Whisper API via Groq)
    pub async fn audio_transcription(
        &self,
        audio_path: &std::path::Path,
    ) -> Result<String, String> {
        let api_key = self
            .config
            .groq_api_key
            .as_ref()
            .ok_or("GROQ_API_KEY not set")?;

        if !self.optimizer.can_use("groq", 1000) {
            return Err("Groq rate limits exceeded".into());
        }

        let file_bytes = std::fs::read(audio_path).map_err(|e| e.to_string())?;
        let file_part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| e.to_string())?;

        let form = reqwest::multipart::Form::new()
            .text("model", "whisper-large-v3-turbo")
            .text("response_format", "verbose_json")
            .part("file", file_part);

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.status().is_success() {
            let json_text = response.text().await.map_err(|e| e.to_string())?;
            self.optimizer.record("groq", 1000);
            Ok(json_text)
        } else {
            Err(format!("Groq Whisper API error: {}", response.status()))
        }
    }

    /// Get token usage status for all providers.
    pub fn token_status(&self) -> String {
        self.optimizer.display_status()
    }

    // ─── Provider Implementations ───────────────────────────────────────────

    /// Call Groq's OpenAI-compatible API.
    async fn call_groq(&self, request: &str, model: &str) -> Result<(String, u64), String> {
        let api_key = self.config.groq_api_key.as_ref().ok_or("No Groq API key")?;

        let payload = json!({
            "model": model,
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

        let resp = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Groq request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Groq API error {}: {}", status, body));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Error: Empty response")
            .to_string();

        let tokens_used = json["usage"]["total_tokens"].as_u64().unwrap_or(0);

        info!("[LLM] Groq ({}) used {} tokens", model, tokens_used);
        Ok((content, tokens_used))
    }

    /// Call Google AI Studio's Gemini API for vision tasks.
    async fn call_google_vision(
        &self,
        prompt: &str,
        image_b64: &str,
    ) -> Result<(String, u64), String> {
        let api_key = self
            .config
            .google_ai_key
            .as_ref()
            .ok_or("No Google AI key")?;

        let model = &self.config.google_vision_model;
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        let payload = json!({
            "contents": [{
                "parts": [
                    { "text": prompt },
                    {
                        "inline_data": {
                            "mime_type": "image/jpeg",
                            "data": image_b64
                        }
                    }
                ]
            }],
            "generationConfig": {
                "temperature": 0.4,
                "maxOutputTokens": 256
            }
        });

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Google Vision request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Google AI Studio error {}: {}", status, body));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        let content = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tokens_used = json["usageMetadata"]["totalTokenCount"]
            .as_u64()
            .unwrap_or(0);

        info!(
            "[LLM] Google Vision ({}) used {} tokens",
            model, tokens_used
        );
        Ok((content, tokens_used))
    }

    /// Call local Ollama (OpenAI-compatible fallback for text).
    async fn call_ollama(&self, request: &str) -> Result<String, String> {
        info!("[LLM] Falling back to Ollama at {}", self.config.ollama_url);

        let payload = json!({
            "model": self.config.ollama_model,
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

        let base = self.config.ollama_url.trim_end_matches('/');
        let endpoint = if base.ends_with("/v1") {
            format!("{}/chat/completions", base)
        } else {
            format!("{}/v1/chat/completions", base)
        };

        match self.client.post(&endpoint).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                    let content = json["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or("Error: Empty response")
                        .to_string();
                    Ok(content)
                } else {
                    Err(format!("Ollama API Error: {}", resp.status()))
                }
            }
            Err(e) => {
                warn!(
                    "[LLM] Ollama also unreachable ({}), entering offline mode",
                    e
                );
                Ok(format!("(Offline Mode) Mock response for: {}", request))
            }
        }
    }

    /// Call local Ollama with vision (VLM fallback).
    async fn call_ollama_vision(&self, prompt: &str, image_b64: &str) -> Result<String, String> {
        let base = self
            .config
            .ollama_url
            .trim_end_matches('/')
            .trim_end_matches("/v1");

        let body = json!({
            "model": "llava:latest",
            "prompt": prompt,
            "images": [image_b64],
            "stream": false
        });

        match self
            .client
            .post(format!("{}/api/generate", base))
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
        {
            Ok(resp) => {
                let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                Ok(json["response"].as_str().unwrap_or("").to_string())
            }
            Err(e) => {
                warn!("[LLM] Ollama VLM also unavailable: {}", e);
                Ok(String::new())
            }
        }
    }
}

/// Rough token estimate: ~4 chars per token for English text.
fn estimate_tokens(text: &str) -> u64 {
    (text.len() as u64) / 4
}
