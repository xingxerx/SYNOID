// SYNOID MCP Server Bridge
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Now supports multi-provider routing:
//   - Groq Cloud (primary, via GROQ_API_KEY)
//   - Google AI Studio (vision, via GOOGLE_AI_KEY)
//   - Ollama (local fallback)
//
// Token usage tracked by the MCP Token Optimizer.

<<<<<<< HEAD
use crate::agent::llm_provider::MultiProviderLlm;
use std::sync::Arc;
use tracing::info;
=======
use crate::agent::llm_provider::{MultiProviderLlm, ProviderConfig};
use crate::agent::token_optimizer;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, warn};
>>>>>>> c55b0d9e6ebf2105e2d2c161f2b2839c68f38981

pub struct SynoidAgent {
    llm: Arc<MultiProviderLlm>,
    pub model: String,
    /// Multi-provider LLM (lazy-initialized on first cloud call).
    multi_llm: Option<Arc<MultiProviderLlm>>,
}

impl SynoidAgent {
<<<<<<< HEAD
    pub fn new(_api_url: &str, model: &str) -> Self {
=======
    pub fn new(api_url: &str, model: &str) -> Self {
        let config = ProviderConfig::default();
        let has_cloud = config.groq_api_key.is_some() || config.google_api_key.is_some();

        let multi_llm = if has_cloud {
            let optimizer = Arc::new(token_optimizer::create_default_optimizer());
            Some(Arc::new(MultiProviderLlm::new(config, optimizer)))
        } else {
            None
        };

>>>>>>> c55b0d9e6ebf2105e2d2c161f2b2839c68f38981
        Self {
            llm: Arc::new(MultiProviderLlm::new()),
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
        self.llm.reason(request).await.or_else(|e| {
            Ok(format!(
                "(Offline Mode) Mock response for: {} (Error: {})",
                request, e
            ))
        })
    }

<<<<<<< HEAD
    pub async fn fast_reason(&self, request: &str) -> Result<String, String> {
        info!("[AGENT] Fast Reasoning with {}: {}", self.model, request);
        self.llm.fast_request(request).await.or_else(|e| {
            Ok(format!(
                "(Offline Mode) Mock fast response for: {} (Error: {})",
                request, e
            ))
        })
    }
=======
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
>>>>>>> c55b0d9e6ebf2105e2d2c161f2b2839c68f38981

    pub async fn vision_reason(&self, prompt: &str, image_b64: &str) -> Result<String, String> {
        info!("[AGENT] Vision Reasoning for {}", self.model);
        self.llm
            .vision_request(prompt, image_b64)
            .await
            .or_else(|e| {
                Ok(format!(
                    "(Offline Mode) Mock vision response (Error: {})",
                    e
                ))
            })
    }

<<<<<<< HEAD
    pub async fn transcribe_audio(&self, audio_path: &std::path::Path) -> Result<String, String> {
        info!("[AGENT] Transcribing audio via LLM Bridge");
        self.llm
            .audio_transcription(audio_path)
            .await
            .or_else(|e| Err(format!("Failed to transcribe: {}", e)))
=======
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
>>>>>>> c55b0d9e6ebf2105e2d2c161f2b2839c68f38981
    }
}
