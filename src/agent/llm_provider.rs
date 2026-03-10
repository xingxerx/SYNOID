// SYNOID Multi-Provider LLM Bridge
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::token_optimizer::TokenOptimizer;
use serde_json::json;
use std::env;
use std::sync::Arc;
use tracing::warn;

pub struct MultiProviderLlm {
    client: reqwest::Client,
    optimizer: Arc<TokenOptimizer>,
    groq_api_key: Option<String>,
    google_api_key: Option<String>,
    ollama_url: String,
}

impl MultiProviderLlm {
    pub fn new() -> Self {
        // Load from env, or use defaults
        let groq_api_key = env::var("GROQ_API_KEY").ok();
        let google_api_key = env::var("GOOGLE_AI_KEY").ok();
        let ollama_url =
            env::var("SYNOID_API_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());

        Self {
            client: reqwest::Client::new(),
            optimizer: Arc::new(TokenOptimizer::new()),
            groq_api_key,
            google_api_key,
            ollama_url,
        }
    }

    // Reasoning capabilities (llama-3.3-70b-versatile)
    pub async fn reason(&self, request: &str) -> Result<String, String> {
        match self.call_groq("llama-3.3-70b-versatile", request).await {
            Ok(res) => Ok(res),
            Err(e) => {
                warn!(
                    "[LLM_BRIDGE] Groq reason failed: {}. Falling back to Ollama.",
                    e
                );
                self.call_ollama("llama3", request).await
            }
        }
    }

    // Fast Request parser (llama-3.1-8b-instant)
    pub async fn fast_request(&self, request: &str) -> Result<String, String> {
        match self.call_groq("llama-3.1-8b-instant", request).await {
            Ok(res) => Ok(res),
            Err(e) => {
                warn!(
                    "[LLM_BRIDGE] Groq fast_request failed: {}. Falling back to Ollama.",
                    e
                );
                self.call_ollama("llama3", request).await
            }
        }
    }

    // Vision processing with Google Gemini
    pub async fn vision_request(&self, prompt: &str, image_b64: &str) -> Result<String, String> {
        match self
            .call_google("gemini-2.5-flash", prompt, image_b64)
            .await
        {
            Ok(res) => Ok(res),
            Err(e) => {
                warn!(
                    "[LLM_BRIDGE] Gemini vision failed: {}. Falling back to Ollama.",
                    e
                );
                self.call_ollama_vision("llava", prompt, image_b64).await
            }
        }
    }

    // Audio Transcription (Whisper API via Groq)
    pub async fn audio_transcription(
        &self,
        audio_path: &std::path::Path,
    ) -> Result<String, String> {
        let api_key = self.groq_api_key.as_ref().ok_or("GROQ_API_KEY not set")?;

        if !self.optimizer.can_make_request("groq") {
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
            // We return the raw string so the caller can parse the segments JSON
            self.optimizer.record_usage("groq", 1000); // flat token cost for audio
            Ok(json_text)
        } else {
            Err(format!("Groq Whisper API error: {}", response.status()))
        }
    }

    async fn call_groq(&self, model: &str, prompt: &str) -> Result<String, String> {
        let api_key = self.groq_api_key.as_ref().ok_or("GROQ_API_KEY not set")?;

        if !self.optimizer.can_make_request("groq") {
            return Err("Groq rate limits exceeded".into());
        }

        let payload = json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are Synoid, an autonomous video production AI. Respond with concise JSON or direct commands."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.7
        });

        // roughly guess tokens (characters / 4 * 2 for input+output)
        let estimated_tokens = (prompt.len() as u64) / 2;

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
            let content = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();

            self.optimizer
                .record_usage("groq", estimated_tokens + (content.len() as u64 / 4));
            Ok(content)
        } else {
            Err(format!("Groq API error: {}", response.status()))
        }
    }

    async fn call_google(
        &self,
        _model: &str,
        prompt: &str,
        image_b64: &str,
    ) -> Result<String, String> {
        let api_key = self
            .google_api_key
            .as_ref()
            .ok_or("GOOGLE_AI_KEY not set")?;

        if !self.optimizer.can_make_request("google") {
            return Err("Google rate limits exceeded".into());
        }

        // Format for Gemini 2.0 Flash REST API
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
            }]
        });

        let estimated_tokens = 500; // rough guess for image + text

        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}", api_key);
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

            // Navigate Gemini's response structure
            let content = json["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string();

            self.optimizer
                .record_usage("google", estimated_tokens + (content.len() as u64 / 4));
            Ok(content)
        } else {
            Err(format!("Google API error: {}", response.status()))
        }
    }

    async fn call_ollama(&self, model: &str, prompt: &str) -> Result<String, String> {
        let payload = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "stream": false
        });

        let endpoint = format!("{}/api/chat", self.ollama_url.trim_end_matches("/v1"));
        let response = self
            .client
            .post(&endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
            Ok(json["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string())
        } else {
            Err(format!("Ollama API error: {}", response.status()))
        }
    }

    async fn call_ollama_vision(
        &self,
        model: &str,
        prompt: &str,
        image_b64: &str,
    ) -> Result<String, String> {
        let payload = json!({
            "model": model,
            "prompt": prompt,
            "images": [image_b64],
            "stream": false
        });

        let endpoint = format!("{}/api/generate", self.ollama_url.trim_end_matches("/v1"));
        let response = self
            .client
            .post(&endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
            Ok(json["response"].as_str().unwrap_or("").to_string())
        } else {
            Err(format!("Ollama Vision API error: {}", response.status()))
        }
    }
}

impl Default for MultiProviderLlm {
    fn default() -> Self {
        Self::new()
    }
}
