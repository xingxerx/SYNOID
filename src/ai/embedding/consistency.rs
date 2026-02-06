// SYNOID™ Content Consistency System
// Character & Style Persistence

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterProfile {
    pub name: String,
    pub face_embedding: Vec<f32>, // e.g. InsightFace
    pub voice_embedding: Vec<f32>, // e.g. Speaker Encoder
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleEmbedding {
    pub name: String,
    pub latent_vector: Vec<f32>, // e.g. LoRA or TI
}

pub struct ConsistencyEngine {
    pub characters: Vec<CharacterProfile>,
    pub styles: Vec<StyleEmbedding>,
}

impl ConsistencyEngine {
    pub fn new() -> Self {
        Self {
            characters: Vec::new(),
            styles: Vec::new(),
        }
    }

    pub fn register_character(&mut self, profile: CharacterProfile) {
        self.characters.push(profile);
    }

    pub fn get_character(&self, name: &str) -> Option<&CharacterProfile> {
        self.characters.iter().find(|c| c.name == name)
    }
}
