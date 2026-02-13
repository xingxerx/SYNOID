// SYNOID Academy - Learning Engine
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleProfile {
    pub name: String,
    pub avg_shot_length: f64,
    pub transition_density: f64,
    pub color_lut: Option<String>,
    pub anamorphic: bool,
}

pub struct StyleLibrary {
    pub profiles: Vec<StyleProfile>,
}

impl StyleLibrary {
    pub fn new() -> Self {
        Self {
            profiles: vec![
                StyleProfile {
                    name: "cinematic".to_string(),
                    avg_shot_length: 4.0,
                    transition_density: 0.5,
                    color_lut: Some("teal_orange.cube".to_string()),
                    anamorphic: true,
                },
                StyleProfile {
                    name: "action".to_string(),
                    avg_shot_length: 1.5,
                    transition_density: 0.9,
                    color_lut: Some("high_contrast.cube".to_string()),
                    anamorphic: true,
                },
            ],
        }
    }

    pub fn get_profile(&self, intent: &str) -> StyleProfile {
        if intent.to_lowercase().contains("action") {
            self.profiles[1].clone()
        } else {
            self.profiles[0].clone() // Default to cinematic
        }
    }
}

pub struct TechniqueExtractor {}
pub mod code_scanner;
pub mod url_reader;
