// SYNOID Hive Mind - Collaborative Intelligence Network
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelRole {
    /// Heavy Lifter: Complex reasoning, planning, coding (e.g. Llama 3 70B, GPT-4)
    Reasoning,
    /// Grunt Worker: Fast, simple tasks, summarization (e.g. Llama 3 8B, Mistral)
    FastResponder,
    /// Specialist: Tuned for specific tasks (e.g. codellama, llava)
    Specialist(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub role: ModelRole,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelBackend {
    /// Local Ollama server
    Ollama,
}

impl std::fmt::Display for ModelBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ollama => write!(f, "Ollama"),
        }
    }
}

pub struct HiveMind {
    pub client: Client,
    pub api_url: String,
    pub models: HashMap<String, OllamaModel>,
    pub active_reasoner: Option<String>,
    pub active_fast_responder: Option<String>,
    /// Which backend the active reasoner lives on.
    pub reasoner_backend: ModelBackend,
    /// Which backend the active fast responder lives on.
    pub fast_backend: ModelBackend,
    /// Cloud providers are disabled.
    pub cloud_enabled: bool,
}

impl HiveMind {
    pub fn new(api_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
            api_url: api_url.to_string(),
            models: HashMap::new(),
            active_reasoner: None,
            active_fast_responder: None,
            reasoner_backend: ModelBackend::Ollama,
            fast_backend: ModelBackend::Ollama,
            cloud_enabled: false,
        }
    }

    /// Connect to Ollama and discover all available intelligence.
    pub async fn refresh_models(&mut self) -> Result<(), String> {
        self.cloud_enabled = false;

        // Then try to discover local Ollama models
        let base_url = self.api_url.trim_end_matches('/').trim_end_matches("/v1");
        let url = format!("{}/api/tags", base_url);
        tracing::debug!("[HIVE_MIND] Scanning neural network at {}...", url);

        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                    if let Some(models) = json["models"].as_array() {
                        for m in models {
                            let name = m["name"].as_str().unwrap_or("unknown").to_string();
                            let size = m["size"].as_u64().unwrap_or(0);
                            let details = m.get("details").cloned();

                            let role = self.assign_role(&name, size);

                            // Only override cloud models if local model is clearly better
                            match role {
                                ModelRole::Reasoning => {
                                    if !self.cloud_enabled {
                                        if self.active_reasoner.is_none()
                                            || size
                                                > self
                                                    .models
                                                    .get(self.active_reasoner.as_ref().unwrap())
                                                    .map(|m| m.size)
                                                    .unwrap_or(0)
                                        {
                                            self.active_reasoner = Some(name.clone());
                                            self.reasoner_backend = ModelBackend::Ollama;
                                        }
                                    }
                                }
                                ModelRole::FastResponder => {
                                    if !self.cloud_enabled && self.active_fast_responder.is_none() {
                                        self.active_fast_responder = Some(name.clone());
                                        self.fast_backend = ModelBackend::Ollama;
                                    }
                                }
                                _ => {}
                            }

                            self.models.insert(
                                name.clone(),
                                OllamaModel {
                                    name,
                                    size,
                                    role,
                                    details,
                                },
                            );
                        }
                    }
                    info!(
                        "[HIVE_MIND] Connected. Found {} total neural nodes (cloud + local).",
                        self.models.len()
                    );
                } else {
                    if !self.cloud_enabled {
                        return Err(format!("Ollama API Error: {}", resp.status()));
                    }
                    info!("[HIVE_MIND] Ollama unavailable but cloud providers active.");
                }
            }
            Err(e) => {
                if !self.cloud_enabled {
                    tracing::debug!(
                        "[HIVE_MIND] Ollama not detected at {}. Continuing with local defaults.",
                        self.api_url
                    );
                    return Err(e.to_string());
                }
                info!("[HIVE_MIND] Ollama offline, using cloud providers (Groq/Google).");
            }
        }

        // Log active configuration
        if let Some(r) = &self.active_reasoner {
            info!(
                "[HIVE_MIND] Prime Reasoner: {} ({})",
                r, self.reasoner_backend
            );
        }
        if let Some(f) = &self.active_fast_responder {
            info!("[HIVE_MIND] Fast Responder: {} ({})", f, self.fast_backend);
        }

        Ok(())
    }

    /// (Cloud identification removed - using Ollama discovery only)

    /// Heuristics to assign roles based on model metadata.
    fn assign_role(&self, name: &str, size: u64) -> ModelRole {
        let lower = name.to_lowercase();
        let size_gb = size as f64 / 1_000_000_000.0;

        // 1. Vision Capability
        if lower.contains("llava") || lower.contains("moondream") || lower.contains("vision") {
            return ModelRole::Specialist("vision".to_string());
        }

        // 2. Heavy Reasoning (Prime Reasoner) — Gemma 4 is the primary sovereign model
        if lower.contains("gemma4")
            || lower.contains("gpt-oss")
            || lower.contains("deepseek-r1")
            || size_gb > 14.0
        {
            return ModelRole::Reasoning;
        }

        // 3. Fast/Efficient Models
        if lower.contains("llama3.2") || lower.contains("gemma2") || lower.contains("gemma3") || size_gb < 10.0 {
            return ModelRole::FastResponder;
        }

        // Default
        ModelRole::FastResponder
    }

    pub fn get_reasoning_model(&self) -> String {
        self.active_reasoner
            .clone()
            .unwrap_or_else(|| "gemma4:26b".to_string())
    }

    pub fn get_fast_model(&self) -> String {
        self.active_fast_responder
            .clone()
            .unwrap_or_else(|| "gemma4:26b".to_string())
    }

    pub fn get_vision_model(&self) -> String {
        "Ollama VLM (Llava/Moondream)".to_string()
    }

    /// Get the active backend for reasoning tasks.
    pub fn get_reasoner_backend(&self) -> &ModelBackend {
        &self.reasoner_backend
    }
}
