#![allow(dead_code)]
// SYNOID MCP Server Bridge
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::multi_agent::NativeTimelineEngine;
use std::sync::Arc;
use tracing::{info, error};
use serde_json::json;

/// Agent interface for LLM reasoning
pub struct SynoidAgent {
    api_url: String,
}

impl SynoidAgent {
    pub fn new(api_url: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
        }
    }

    /// Reason about a request using the LLM backend
    pub async fn reason(&self, request: &str) -> Result<String, String> {
        info!("[AGENT] Reasoning about: {}", request);

        let client = reqwest::Client::new();
        // Handle trailing slash just in case
        let base_url = self.api_url.trim_end_matches('/');
        // Default to chat completions endpoint
        let url = format!("{}/chat/completions", base_url);

        // Default model - usually 'llama2' or 'mistral' for local Ollama
        // Ideally this should be configurable
        let model = "llama3";

        let payload = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": "You are Synoid, an autonomous intelligent video production kernel. Your goal is to assist the user with video editing, analysis, and creative direction. Be concise and technical."},
                {"role": "user", "content": request}
            ],
            "stream": false
        });

        match client.post(&url)
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(json_body) => {
                            // Parse OpenAI-compatible format
                            if let Some(content) = json_body["choices"][0]["message"]["content"].as_str() {
                                Ok(content.to_string())
                            } else {
                                Err(format!("Unexpected API response structure: {}", json_body))
                            }
                        },
                        Err(e) => Err(format!("Failed to parse JSON response: {}", e))
                    }
                } else {
                    Err(format!("API returned error status: {}", response.status()))
                }
            },
            Err(e) => {
                error!("LLM Connection Error: {}", e);
                Err(format!("Could not connect to Brain at {}. Is the LLM server running?", url))
            }
        }
    }
}

// Mock MCP SDK Structures
pub struct Tool {
    pub name: String,
    pub description: String,
    pub handler: Box<dyn Fn(&str) + Send + Sync>,
}

impl Tool {
    pub fn new<F>(name: &str, description: &str, handler: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            handler: Box::new(handler),
        }
    }
}

pub struct Resource {
    pub uri: String,
    pub description: String,
}

impl Resource {
    pub fn new(uri: &str, description: &str) -> Self {
        Self {
            uri: uri.to_string(),
            description: description.to_string(),
        }
    }
}

pub struct Server {
    pub name: String,
    pub tools: Vec<Tool>,
    pub resources: Vec<Resource>,
}

impl Server {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tools: Vec::new(),
            resources: Vec::new(),
        }
    }

    pub fn register_tool(&mut self, tool: Tool) {
        self.tools.push(tool);
    }

    pub fn register_resource(&mut self, resource: Resource) {
        self.resources.push(resource);
    }
}

// Synoid MCP Implementation

pub struct SynoidMcpServer {
    pub project_root: String,
    pub timeline_engine: Arc<NativeTimelineEngine>,
    pub mcp_server: Server,
}

impl SynoidMcpServer {
    pub fn init(path: &str, engine: Arc<NativeTimelineEngine>) -> Self {
        let mut server = Server::new("SYNOID_Core_Bridge");

        // Tool: Allows agent to execute a trim in the native app
        server.register_tool(Tool::new(
            "trim_clip",
            "Trims a specific clip in the SYNOID timeline",
            |args| {
                info!("[MCP] Executing native trim: {:?}", args);
            },
        ));

        // Resource: Exposes the current project media folder
        server.register_resource(Resource::new(
            "media://project/assets",
            "Access to local raw footage for semantic indexing",
        ));

        Self {
            project_root: path.to_string(),
            timeline_engine: engine,
            mcp_server: server,
        }
    }
}
