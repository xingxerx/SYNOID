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

/// Which backend a model lives on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelBackend {
    /// Local Ollama server
    Ollama,
    /// Groq Cloud API
    Groq,
    /// Google AI Studio (Gemini)
    Google,
}

impl std::fmt::Display for ModelBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ollama => write!(f, "Ollama"),
            Self::Groq => write!(f, "Groq"),
            Self::Google => write!(f, "Google"),
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
    /// Whether cloud providers (Groq/Google) are configured.
    pub cloud_enabled: bool,
}

impl HiveMind {
    pub fn new(api_url: &str) -> Self {
        let has_groq = std::env::var("GROQ_API_KEY").is_ok();
        let has_google = std::env::var("GOOGLE_AI_KEY").is_ok();
        let cloud_enabled = has_groq || has_google;

        // If Groq is configured, use cloud models as defaults
        let (reasoner, reasoner_backend, fast, fast_backend) = if has_groq {
            (
                Some("llama-3.3-70b-versatile".to_string()),
                ModelBackend::Groq,
                Some("llama-3.1-8b-instant".to_string()),
                ModelBackend::Groq,
            )
        } else {
            (None, ModelBackend::Ollama, None, ModelBackend::Ollama)
        };

        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
            api_url: api_url.to_string(),
            models: HashMap::new(),
            active_reasoner: reasoner,
            active_fast_responder: fast,
            reasoner_backend,
            fast_backend,
            cloud_enabled,
        }
    }

    /// Connect to Ollama and discover all available intelligence.
    /// Cloud models (Groq/Google) are registered statically from env vars.
    pub async fn refresh_models(&mut self) -> Result<(), String> {
        // [HOT RELOAD] Re-read .env to pull any newly added API keys dynamically
        if let Ok(content) = std::fs::read_to_string(".env") {
            for line in content.lines() {
                let s = line.trim();
                if s.starts_with('#') || s.is_empty() { continue; }
                if let Some((k, v)) = s.split_once('=') {
                    let key = k.trim();
                    let val = v.trim().trim_matches('"').trim_matches('\'');
                    std::env::set_var(key, val);
                }
            }
        }
        self.cloud_enabled = std::env::var("GROQ_API_KEY").is_ok() || std::env::var("GOOGLE_AI_KEY").is_ok();

        // Register cloud models first
        self.register_cloud_models();

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
        if std::env::var("GOOGLE_AI_KEY").is_ok() {
            info!("[HIVE_MIND] Vision Provider: Google AI Studio (Gemini)");
        }

        Ok(())
    }

    /// Register cloud-hosted models from environment variables.
    fn register_cloud_models(&mut self) {
        if std::env::var("GROQ_API_KEY").is_ok() {
            // Groq reasoning model
            let reasoning_model = std::env::var("GROQ_REASONING_MODEL")
                .unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string());
            self.models.insert(
                reasoning_model.clone(),
                OllamaModel {
                    name: reasoning_model.clone(),
                    size: 70_000_000_000, // 70B params
                    role: ModelRole::Reasoning,
                    details: Some(serde_json::json!({"provider": "groq"})),
                },
            );

            // Groq fast model
            let fast_model = std::env::var("GROQ_FAST_MODEL")
                .unwrap_or_else(|_| "llama-3.1-8b-instant".to_string());
            self.models.insert(
                fast_model.clone(),
                OllamaModel {
                    name: fast_model.clone(),
                    size: 8_000_000_000, // 8B params
                    role: ModelRole::FastResponder,
                    details: Some(serde_json::json!({"provider": "groq"})),
                },
            );

            self.active_reasoner = Some(reasoning_model);
            self.reasoner_backend = ModelBackend::Groq;
            self.active_fast_responder = Some(fast_model);
            self.fast_backend = ModelBackend::Groq;
        }

        if std::env::var("GOOGLE_AI_KEY").is_ok() {
            let vision_model = std::env::var("GOOGLE_VISION_MODEL")
                .unwrap_or_else(|_| "gemini-2.0-flash".to_string());
            self.models.insert(
                vision_model.clone(),
                OllamaModel {
                    name: vision_model,
                    size: 0,
                    role: ModelRole::Specialist("vision".to_string()),
                    details: Some(serde_json::json!({"provider": "google"})),
                },
            );
        }
    }

    /// Heuristics to assign roles based on model metadata.
    fn assign_role(&self, name: &str, size: u64) -> ModelRole {
        let lower = name.to_lowercase();
        let size_gb = size as f64 / 1_000_000_000.0;

        // 1. Specialist Detection
        if lower.contains("code") || lower.contains("deepseek-coder") {
            return ModelRole::Specialist("coding".to_string());
        }
        if lower.contains("llava") || lower.contains("vision") || lower.contains("gemini") {
            return ModelRole::Specialist("vision".to_string());
        }
        if lower.contains("dolphin") || lower.contains("uncensored") {
            return ModelRole::Specialist("creative".to_string());
        }

        // 2. Reasoning vs Grunt Isolation (Size-based)
        if size_gb > 14.0
            || lower.contains("70b")
            || lower.contains("mixtral")
            || lower.contains("deepseek-r1")
            || lower.contains("gpt-oss")
        {
            return ModelRole::Reasoning;
        }

        // Default to fast responder for smaller models (7B, 8B)
        ModelRole::FastResponder
    }

    pub fn get_reasoning_model(&self) -> String {
        self.active_reasoner
            .clone()
            .unwrap_or_else(|| "llama3:latest".to_string())
    }

    pub fn get_fast_model(&self) -> String {
        self.active_fast_responder
            .clone()
            .unwrap_or_else(|| "llama3:latest".to_string())
    }

    /// Check if vision is available via Google AI Studio.
    pub fn has_cloud_vision(&self) -> bool {
        std::env::var("GOOGLE_AI_KEY").is_ok()
    }

    pub fn get_vision_model(&self) -> String {
        if self.has_cloud_vision() {
            std::env::var("GOOGLE_VISION_MODEL").unwrap_or_else(|_| "gemini-2.0-flash (Google)".to_string())
        } else {
            "None".to_string()
        }
    }

    /// Get the active backend for reasoning tasks.
    pub fn get_reasoner_backend(&self) -> &ModelBackend {
        &self.reasoner_backend
    }
}
