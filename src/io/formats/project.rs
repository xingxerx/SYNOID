// SYNOID™ Project Format (.synoid)
// JSON manifest for project state

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use crate::agents::multi_agent::Timeline; // Using the internal timeline representation

#[derive(Debug, Serialize, Deserialize)]
pub struct SynoidProject {
    pub version: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub timeline: Timeline,
    pub assets: Vec<AssetEntry>,
    pub metadata: ProjectMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetEntry {
    pub id: String,
    pub path: String, // Relative path in project bundle
    pub kind: String, // "video", "audio", "image"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub author: String,
    pub intent: String,
    pub style_profile: Option<String>,
}

impl SynoidProject {
    pub fn new(name: &str) -> Self {
        Self {
            version: "0.1.0".to_string(),
            name: name.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            timeline: Timeline::new(name),
            assets: Vec::new(),
            metadata: ProjectMetadata {
                author: "Unknown".to_string(),
                intent: "".to_string(),
                style_profile: None,
            },
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let json = fs::read_to_string(path)?;
        let project: Self = serde_json::from_str(&json)?;
        Ok(project)
    }
}
