// SYNOID Engine Module
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// DAG-based edit graph and frame types for the node pipeline.

pub mod graph;

/// Represents a single video/audio frame flowing through the SYNOID node graph.
/// This is the fundamental data unit that nodes process and pass along edges.
#[derive(Debug, Clone)]
pub struct SynoidFrame {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Raw pixel data (RGBA u8)
    pub data: Vec<u8>,
    /// Presentation timestamp in seconds
    pub pts: f64,
    /// Frame index in the sequence
    pub index: u64,
}

impl SynoidFrame {
    /// Create a new empty frame with given dimensions
    pub fn new(width: u32, height: u32) -> Self {
        let data = vec![0u8; (width * height * 4) as usize];
        Self {
            width,
            height,
            data,
            pts: 0.0,
            index: 0,
        }
    }
}
