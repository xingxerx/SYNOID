// SYNOID™ MCP Server Bridge
// Copyright (c) 2026 Xing_The_Creator | SYNOID™

use std::sync::Arc;
use crate::agent::multi_agent::NativeTimelineEngine;
use tracing::info;

// Mock MCP SDK Structures
pub struct Tool {
    pub name: String,
    pub description: String,
    pub handler: Box<dyn Fn(&str) + Send + Sync>,
}

impl Tool {
    pub fn new<F>(name: &str, description: &str, handler: F) -> Self
    where F: Fn(&str) + Send + Sync + 'static {
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
            }
        ));

        // Resource: Exposes the current project media folder
        server.register_resource(Resource::new(
            "media://project/assets",
            "Access to local raw footage for semantic indexing"
        ));

        Self {
            project_root: path.to_string(),
            timeline_engine: engine,
            mcp_server: server,
        }
    }
}
