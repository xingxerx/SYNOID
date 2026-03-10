// SYNOID MCP Server Bridge
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Now supports multi-provider routing:
//   - Groq Cloud (primary, via GROQ_API_KEY)
//   - Google AI Studio (vision, via GOOGLE_AI_KEY)
//   - Ollama (local fallback)
//
// Token usage tracked by the MCP Token Optimizer.

use crate::agent::llm_provider::{MultiProviderLlm, ProviderConfig};
use crate::agent::token_optimizer;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, warn};

pub struct SynoidAgent {
    client: reqwest::Client,
    api_url: String,
    pub model: String,
    /// Multi-provider LLM (lazy-initialized on first cloud call).
    multi_llm: Option<Arc<MultiProviderLlm>>,
}

impl SynoidAgent {
    pub fn new(api_url: &str, model: &str) -> Self {
        let config = ProviderConfig::default();
        let has_cloud = config.groq_api_key.is_some() || config.google_api_key.is_some();

        let multi_llm = if has_cloud {
            let optimizer = Arc::new(token_optimizer::create_default_optimizer());
            Some(Arc::new(MultiProviderLlm::new(config, optimizer)))
        } else {
            None
        };

        Self {
            client: reqwest::Client::new(),
            api_url: api_url.to_string(),
            model: model.to_string(),
            multi_llm,
        }
    }

    /// Create a SynoidAgent with a shared MultiProviderLlm instance.
    pub fn with_provider(api_url: &str, model: &str, provider: Arc<MultiProviderLlm>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url: api_url.to_string(),
            model: model.to_string(),
            multi_llm: Some(provider),
        }
    }

    /// Get a reference to the multi-provider LLM (if cloud is configured).
    pub fn multi_provider(&self) -> Option<&Arc<MultiProviderLlm>> {
        self.multi_llm.as_ref()
    }

    pub async fn reason(&self, request: &str) -> Result<String, String> {
        info!("[AGENT] Reasoning with {}: {}", self.model, request);

        // Route through multi-provider if available
        if let Some(llm) = &self.multi_llm {
            return llm.reason(request).await;
        }

        // Legacy: Direct Ollama call
        self.call_ollama_direct(request).await
    }

    /// Fast request for simple tasks (JSON parsing, classification).
    pub async fn fast_reason(&self, request: &str) -> Result<String, String> {
        if let Some(llm) = &self.multi_llm {
            return llm.fast_request(request).await;
        }
        self.call_ollama_direct(request).await
    }

    /// Vision request for frame analysis.
    pub async fn vision_reason(&self, prompt: &str, image_b64: &str) -> Result<String, String> {
        if let Some(llm) = &self.multi_llm {
            return llm.vision_request(prompt, image_b64).await;
        }
        // Fallback: no vision available in legacy mode
        Err("No vision provider configured. Set GOOGLE_AI_KEY for Google AI Studio.".to_string())
    }

    /// Get token usage status across all providers.
    pub fn token_status(&self) -> String {
        if let Some(llm) = &self.multi_llm {
            llm.token_status()
        } else {
            "Local Ollama (no token tracking)".to_string()
        }
    }

    /// Legacy direct Ollama call (preserved for backward compatibility).
    async fn call_ollama_direct(&self, request: &str) -> Result<String, String> {
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

        let base = self.api_url.trim_end_matches('/');
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
                    Err(format!("API Error: {}", resp.status()))
                }
            }
            Err(e) => {
                warn!("[AGENT] LLM Connection Failed ({}), falling back to Offline Mode.", e);
                Ok(format!("(Offline Mode) Mock response for: {}", request))
            }
        }
    }
}
