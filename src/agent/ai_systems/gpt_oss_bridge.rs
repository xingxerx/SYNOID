// SYNOID GptOssBridge - High-level Agent Interface
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Bridges the SYNOID Agent's high-level reasoning with the MultiProviderLlm.
// Handles reasoning, vision, and intent-based routing.

use crate::agent::ai_systems::llm_provider::{MultiProviderLlm, ProviderConfig};
use crate::agent::ai_systems::token_optimizer::create_default_optimizer;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone)]
pub struct SynoidAgent {
    pub provider: Arc<MultiProviderLlm>,
    pub model: String,
}

impl SynoidAgent {
    /// Create a new SynoidAgent backed by Gemma 4 (Ollama primary, cloud fallback).
    pub fn new(_api_url: &str, model: &str) -> Self {
        let optimizer = Arc::new(create_default_optimizer());
        let mut config = ProviderConfig::from_env();
        // If caller didn't specify a model, use Gemma 4 as sovereign default
        let effective_model = if model.is_empty() || model == "default" {
            config.ollama_model.clone()
        } else {
            model.to_string()
        };
        config.ollama_model = effective_model.clone();
        let provider = Arc::new(MultiProviderLlm::new(config, optimizer));
        info!("[AGENT] Initialized with primary model: {}", effective_model);

        Self {
            provider,
            model: effective_model,
        }
    }

    /// High-level reasoning (text-only).
    pub async fn reason(&self, request: &str) -> Result<String, String> {
        info!("[AGENT] Reasoning with {}: {}", self.model, request);
        self.provider.reason(request).await
    }

    /// Fast reasoning for classification/JSON parsing.
    pub async fn fast_reason(&self, request: &str) -> Result<String, String> {
        self.provider.fast_request(request).await
    }

    /// Vision reasoning (frame analysis).
    pub async fn vision_reason(&self, prompt: &str, image_b64: &str) -> Result<String, String> {
        self.provider.vision_request(prompt, image_b64).await
    }

    /// Audio Transcription Proxy.
    pub async fn transcribe_audio(&self, audio_path: &std::path::Path) -> Result<String, String> {
        self.provider.audio_transcription(audio_path).await
    }
}
