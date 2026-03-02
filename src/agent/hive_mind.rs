// SYNOID Hive Mind - Collaborative Intelligence Network
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use reqwest::Client;

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

pub struct HiveMind {
    pub client: Client,
    pub api_url: String,
    pub models: HashMap<String, OllamaModel>,
    pub active_reasoner: Option<String>,
    pub active_fast_responder: Option<String>,
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
        }
    }

    /// Connect to Ollama and discover all available intelligence
    pub async fn refresh_models(&mut self) -> Result<(), String> {
        // Strip /v1 suffix if present — Ollama's native API doesn't use it
        // (The /v1 prefix is only for OpenAI-compatible chat/completions endpoint)
        let base_url = self.api_url.trim_end_matches('/').trim_end_matches("/v1");
        let url = format!("{}/api/tags", base_url);
        tracing::debug!("[HIVE_MIND] 📡 Scanning neural network at {}...", url);

        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                    if let Some(models) = json["models"].as_array() {
                        self.models.clear();
                        for m in models {
                            let name = m["name"].as_str().unwrap_or("unknown").to_string();
                            let size = m["size"].as_u64().unwrap_or(0);
                            let details = m.get("details").cloned();

                            let role = self.assign_role(&name, size);
                            
                            // Auto-select best models
                            match role {
                                ModelRole::Reasoning => {
                                    if self.active_reasoner.is_none() || size > self.models.get(self.active_reasoner.as_ref().unwrap()).map(|m| m.size).unwrap_or(0) {
                                        self.active_reasoner = Some(name.clone());
                                    }
                                }
                                ModelRole::FastResponder => {
                                    // Prefer smaller but capable models for speed, but not tiny
                                    if self.active_fast_responder.is_none() {
                                        self.active_fast_responder = Some(name.clone());
                                    }
                                }
                                _ => {}
                            }

                            self.models.insert(name.clone(), OllamaModel {
                                name,
                                size,
                                role,
                                details,
                            });
                        }
                        
                        info!("[HIVE_MIND] ✅ Connected. Found {} active neual nodes.", self.models.len());
                        if let Some(r) = &self.active_reasoner {
                            info!("[HIVE_MIND] 🧠 Prime Reasoner: {}", r);
                        }
                        if let Some(f) = &self.active_fast_responder {
                            info!("[HIVE_MIND] ⚡ Fast Responder: {}", f);
                        }
                    }
                    Ok(())
                } else {
                    Err(format!("Ollama API Error: {}", resp.status()))
                }
            }
            Err(e) => {
                tracing::debug!("[HIVE_MIND] 📡 Ollama not detected at {}. Continuing with local defaults.", self.api_url);
                tracing::debug!("[HIVE_MIND] Connection error: {}", e);
                Err(e.to_string())
            }
        }
    }

    /// heuristics to assign roles based on model metadata
    fn assign_role(&self, name: &str, size: u64) -> ModelRole {
        let lower = name.to_lowercase();
        let size_gb = size as f64 / 1_000_000_000.0;

        // 1. Specialist Detection
        if lower.contains("code") || lower.contains("deepseek-coder") {
            return ModelRole::Specialist("coding".to_string());
        }
        if lower.contains("llava") || lower.contains("vision") {
            return ModelRole::Specialist("vision".to_string());
        }
        if lower.contains("dolphin") || lower.contains("uncensored") {
             return ModelRole::Specialist("creative".to_string());
        }

        // 2. Reasoning vs Grunt Isolation (Size-based)
        // > 14GB usually implies > 13B parameters (FP16/Q4), good for reasoning
        if size_gb > 14.0 || lower.contains("70b") || lower.contains("mixtral") || lower.contains("deepseek-r1") || lower.contains("gpt-oss") {
            return ModelRole::Reasoning;
        }

        // Default to fast responder for smaller models (7B, 8B)
        ModelRole::FastResponder
    }

    pub fn get_reasoning_model(&self) -> String {
        self.active_reasoner.clone().unwrap_or_else(|| "llama3:latest".to_string())
    }

    pub fn get_fast_model(&self) -> String {
        self.active_fast_responder.clone().unwrap_or_else(|| "llama3:latest".to_string())
    }
}
