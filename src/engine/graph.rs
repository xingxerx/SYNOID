#![allow(dead_code)]
// SYNOID™ Edit Graph - DAG Representatione - The "Synoid-Link" DAG Implementation
// Copyright (c) 2026 Xing_The_Creator | SYNOID™
//
// This module implements the ComfyUI-style node graph for video editing.
// The AI agent manipulates this graph to define the editing pipeline.

use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::Directed;
use serde::{Deserialize, Serialize};

/// Represents a single node action in the SYNOID edit graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeAction {
    /// Source node: Load video from path
    Source(String),
    
    /// Cut node: Trim video between start and end timestamps
    Cut { start: f64, end: f64 },
    
    /// Filter node: Apply an FFmpeg filter string
    Filter(String),
    
    /// Speed node: Change playback speed
    Speed { factor: f64 },
    
    /// Color node: Apply color grading with intensity
    Color { intensity: f32 },
    
    /// Scale node: Resize video
    Scale { width: u32, height: u32 },
    
    /// Crop node: Extract region from video
    Crop { x: u32, y: u32, w: u32, h: u32 },
    
    /// Concat node: Join multiple inputs
    Concat,
    
    /// Agent Review node: Hook for vision agent analysis
    AgentReview { prompt: String },
    
    /// Output node: Export to file
    Output(String),

    /// Overlay node: Overlay an asset on top
    Overlay { asset_idx: usize, x: i32, y: i32, start: f64, duration: f64 },
}

/// A connection between two nodes in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConnection {
    pub from_pin: String,
    pub to_pin: String,
}

/// The main SYNOID editing graph
pub struct EditorGraph {
    pub dag: StableGraph<NodeAction, NodeConnection, Directed>,
    pub additional_inputs: Vec<String>,
}

impl EditorGraph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self {
            dag: StableGraph::new(),
            additional_inputs: Vec::new(),
        }
    }
    
    /// Register an external asset and return its index (1-based, since 0 is main input)
    pub fn add_asset(&mut self, path: String) -> usize {
        self.additional_inputs.push(path);
        self.additional_inputs.len() // 1 means first additional input (which is stream 1)
    }
    
    /// Add a node to the graph
    pub fn add_node(&mut self, action: NodeAction) -> NodeIndex {
        self.dag.add_node(action)
    }
    
    /// Connect two nodes
    pub fn connect(&mut self, from: NodeIndex, to: NodeIndex, connection: NodeConnection) {
        self.dag.add_edge(from, to, connection);
    }
    
    /// Build an FFmpeg filter complex string from the graph
    pub fn build_ffmpeg_filter(&self) -> String {
        let mut filters = Vec::new();
        let mut stream_idx = 0;
        
        for node_idx in self.dag.node_indices() {
            if let Some(action) = self.dag.node_weight(node_idx) {
                match action {
                    NodeAction::Cut { start, end } => {
                        filters.push(format!(
                            "[{}:v]trim={}:{},setpts=PTS-STARTPTS[v{}]",
                            stream_idx, start, end, stream_idx
                        ));
                    }
                    NodeAction::Scale { width, height } => {
                        filters.push(format!(
                            "[v{}]scale={}:{}[v{}]",
                            stream_idx, width, height, stream_idx + 1
                        ));
                        stream_idx += 1;
                    }
                    NodeAction::Speed { factor } => {
                        let pts_factor = 1.0 / factor;
                        filters.push(format!(
                            "[v{}]setpts={}*PTS[v{}]",
                            stream_idx, pts_factor, stream_idx + 1
                        ));
                        stream_idx += 1;
                    }
                    NodeAction::Filter(f) => {
                        filters.push(format!(
                            "[v{}]{}[v{}]",
                            stream_idx, f, stream_idx + 1
                        ));
                        stream_idx += 1;
                    }
                    NodeAction::Color { intensity } => {
                        filters.push(format!(
                            "[v{}]eq=brightness={}[v{}]",
                            stream_idx, intensity, stream_idx + 1
                        ));
                        stream_idx += 1;
                    }
                    NodeAction::Overlay { asset_idx, x, y, start, duration } => {
                        filters.push(format!(
                            "[v{}][{}:v]overlay={}:{}:enable='between(t,{},{})'[v{}]",
                            stream_idx, asset_idx, x, y, start, start + duration, stream_idx + 1
                        ));
                        stream_idx += 1;
                    }
                    _ => {}
                }
            }
        }
        
        if filters.is_empty() {
            "null".to_string()
        } else {
            filters.join("; ")
        }
    }
    
    /// Build a complete FFmpeg command from the graph
    pub fn to_ffmpeg_command(&self, input: &str, output: &str) -> String {
        let filter = self.build_ffmpeg_filter();
        let mut cmd = format!("ffmpeg -i \"{}\"", input);
        for asset in &self.additional_inputs {
             cmd.push_str(&format!(" -i \"{}\"", asset));
        }
        cmd.push_str(&format!(" -filter_complex \"{}\" -c:v libx264 -preset fast -y \"{}\"", filter, output));
        cmd
    }
    
    /// Serialize the graph to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let nodes: Vec<_> = self.dag.node_indices()
            .filter_map(|idx| self.dag.node_weight(idx).cloned())
            .collect();
        serde_json::to_string_pretty(&nodes)
    }
    
    /// Create a simple cut-and-scale pipeline
    pub fn create_simple_pipeline(input: &str, output: &str, cuts: Vec<(f64, f64)>) -> Self {
        let mut graph = Self::new();
        
        // Add source
        let source = graph.add_node(NodeAction::Source(input.to_string()));
        
        // Add cuts
        let mut prev = source;
        for (start, end) in cuts {
            let cut = graph.add_node(NodeAction::Cut { start, end });
            graph.connect(prev, cut, NodeConnection {
                from_pin: "video".to_string(),
                to_pin: "input".to_string(),
            });
            prev = cut;
        }
        
        // Add output
        let out = graph.add_node(NodeAction::Output(output.to_string()));
        graph.connect(prev, out, NodeConnection {
            from_pin: "video".to_string(),
            to_pin: "input".to_string(),
        });
        
        graph
    }
}

impl Default for EditorGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for nodes that can be linked into the SYNOID graph
pub trait SynoidLink: Send + Sync {
    /// Execute this node's logic
    fn execute(&self, input: &crate::nodes::ffi::SynoidFrame) -> Result<crate::nodes::ffi::SynoidFrame, String>;
    
    /// Get the node's identity for debugging
    fn identity(&self) -> String;
    
    /// Convert to FFmpeg filter segment
    fn to_ffmpeg_filter(&self) -> Option<String> {
        None
    }
}

/// A graph delta command from the AI agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum GraphDelta {
    /// Insert a new node
    #[serde(rename = "insert_node")]
    InsertNode {
        #[serde(rename = "type")]
        node_type: String,
        params: serde_json::Value,
        after: Option<u32>,
    },
    
    /// Remove a node
    #[serde(rename = "remove_node")]
    RemoveNode {
        node_id: u32,
    },
    
    /// Update node parameters
    #[serde(rename = "update_node")]
    UpdateNode {
        node_id: u32,
        params: serde_json::Value,
    },
    
    /// Connect two nodes
    #[serde(rename = "connect")]
    Connect {
        from: u32,
        to: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_simple_graph() {
        let graph = EditorGraph::create_simple_pipeline(
            "input.mp4",
            "output.mp4",
            vec![(0.0, 5.0), (10.0, 15.0)],
        );
        
        assert!(graph.dag.node_count() > 0);
    }
    
    #[test]
    fn test_ffmpeg_filter_generation() {
        let mut graph = EditorGraph::new();
        graph.add_node(NodeAction::Cut { start: 0.0, end: 5.0 });
        graph.add_node(NodeAction::Scale { width: 1920, height: 1080 });
        
        let filter = graph.build_ffmpeg_filter();
        assert!(filter.contains("trim"));
        assert!(filter.contains("scale"));
    }
}
