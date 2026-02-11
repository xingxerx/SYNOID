#![allow(dead_code)]
// SYNOID MCP Server Bridge
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::multi_agent::NativeTimelineEngine;
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info};

pub struct SynoidAgent {
    client: reqwest::Client,
    api_url: String,
    model: String,
}

impl SynoidAgent {
    pub fn new(api_url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url: api_url.to_string(),
            model: model.to_string(),
        }
    }

    pub async fn reason(&self, request: &str) -> Result<String, String> {
        info!("[AGENT] Reasoning with {}: {}", self.model, request);

        // Construct standard OpenAI-compatible Chat Completion request
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

        let endpoint = format!("{}/chat/completions", self.api_url.trim_end_matches('/'));

        match self.client.post(&endpoint).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                    // Extract content from: choices[0].message.content
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
                error!("LLM Connection Failed: {}", e);
                // Fallback for offline testing
                Ok(format!("(Offline Mode) Mock response for: {}", request))
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
