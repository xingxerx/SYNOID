// SYNOID MCP Server Bridge
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::llm_provider::MultiProviderLlm;
use std::sync::Arc;
use tracing::info;

pub struct SynoidAgent {
    llm: Arc<MultiProviderLlm>,
    pub model: String,
}

impl SynoidAgent {
    pub fn new(_api_url: &str, model: &str) -> Self {
        Self {
            llm: Arc::new(MultiProviderLlm::new()),
            model: model.to_string(),
        }
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

    pub async fn fast_reason(&self, request: &str) -> Result<String, String> {
        info!("[AGENT] Fast Reasoning with {}: {}", self.model, request);
        self.llm.fast_request(request).await.or_else(|e| {
            Ok(format!(
                "(Offline Mode) Mock fast response for: {} (Error: {})",
                request, e
            ))
        })
    }

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

    pub async fn transcribe_audio(&self, audio_path: &std::path::Path) -> Result<String, String> {
        info!("[AGENT] Transcribing audio via LLM Bridge");
        self.llm
            .audio_transcription(audio_path)
            .await
            .or_else(|e| Err(format!("Failed to transcribe: {}", e)))
    }
}
