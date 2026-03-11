// SYNOID GptOssBridge - High-level Agent Interface
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Bridges the SYNOID Agent's high-level reasoning with the MultiProviderLlm.
// Handles reasoning, vision, and intent-based routing.

use crate::agent::llm_provider::{MultiProviderLlm, ProviderConfig};
use crate::agent::token_optimizer::create_default_optimizer;
use std::sync::Arc;
use tracing::info;

pub struct SynoidAgent {
    pub provider: Arc<MultiProviderLlm>,
    pub model: String,
}

impl SynoidAgent {
    /// Create a new SynoidAgent. If Groq/Google keys are missing, it falls back to Ollama.
    pub fn new(_api_url: &str, model: &str) -> Self {
        let optimizer = Arc::new(create_default_optimizer());
        let config = ProviderConfig::default();
        let provider = Arc::new(MultiProviderLlm::new(config, optimizer));

        Self {
            provider,
            model: model.to_string(),
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
