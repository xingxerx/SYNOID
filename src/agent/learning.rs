#![allow(dead_code, unused_variables)]
// SYNOID Learning Kernel
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This module provides a "Memory" for the agent, allowing it to:
// 1. Store successful edit parameters (pacing, cut frequency)
// 2. Retrieve "best practices" for specific intents
// 3. Adapt over time based on feedback

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::info;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EditingPattern {
    pub intent_tag: String,
    pub avg_scene_duration: f64,
    pub transition_speed: f64,
    pub music_sync_strictness: f64, // 0.0 to 1.0
    pub color_grade_style: String,
    pub success_rating: u32, // 1-5 stars
    pub source_video: Option<String>,
}

impl Default for EditingPattern {
    fn default() -> Self {
        Self {
            intent_tag: "general".to_string(),
            avg_scene_duration: 3.5,
            transition_speed: 1.0,
            music_sync_strictness: 0.5,
            color_grade_style: "neutral".to_string(),
            success_rating: 3,
            source_video: None,
        }
    }
}

pub struct LearningKernel {
    memory_path: PathBuf,
    patterns: HashMap<String, EditingPattern>,
}

impl LearningKernel {
    pub fn new() -> Self {
        let path = PathBuf::from("brain_memory.json");
        let patterns = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
                Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        };

        Self {
            memory_path: path,
            patterns,
        }
    }

    /// Retrieve the best known editing pattern for a user intent
    pub fn recall_pattern(&self, intent: &str) -> EditingPattern {
        let intent_lower = intent.to_lowercase();
        
        // 1. Direct Match
        if let Some(pattern) = self.patterns.get(intent) {
            info!("[KERNEL] 🧠 Exact match pattern for '{}'", intent);
            return pattern.clone();
        }

        // 2. Keyword Match (Iterate over all keys)
        // If the user asks for "gaming video", and we have "gaming_montage", match it.
        for (key, pattern) in &self.patterns {
            if intent_lower.contains(key) || key.contains(&intent_lower) {
                info!("[KERNEL] 🧠 Fuzzy match: '{}' -> '{}'", intent, key);
                return pattern.clone();
            }
        }
        
        // 3. Fallback Heuristics (if no learned data matches)
        let key = if intent_lower.contains("hype") || intent_lower.contains("fast") {
            "fast_paced" 
        } else if intent_lower.contains("cinematic") || intent_lower.contains("slow") {
            "cinematic"
        } else {
            "general"
        };

        if let Some(pattern) = self.patterns.get(key) {
            info!("[KERNEL] 🧠 Fallback heuristic pattern for '{}': {:?}", key, pattern);
            return pattern.clone();
        } 
        
        // 4. Ultimate Fallback: The highest rated learned pattern
        // This ensures the agent "applies what it learns to any video we provide it with"
        if let Some((best_key, best_pattern)) = self.patterns.iter()
            .filter(|(_, p)| p.intent_tag != "general" && p.success_rating >= 4)
            .max_by_key(|(_, p)| p.success_rating) {
            
            info!("[KERNEL] 🧠 Applying generalized learned knowledge from '{}' to this video.", best_key);
            return best_pattern.clone();
        }

        info!("[KERNEL] New context encountered. Using default heuristics.");
        EditingPattern::default()
    }

    /// Store a successful editing decision to long-term memory
    pub fn memorize(&mut self, intent: &str, pattern: EditingPattern) {
        // Store under the specific intent tag provided by the learning process
        // e.g. "cinematic_travel_video"
        let key = intent.to_lowercase().replace(" ", "_");
        
        info!("[KERNEL] 💾 Memorizing pattern for '{}'", key);
        self.patterns.insert(key.clone(), pattern.clone());
        self.save();
        self.log_learned_style_to_markdown(&key, &pattern);
    }

    fn log_learned_style_to_markdown(&self, key: &str, pattern: &EditingPattern) {
        let md_path = PathBuf::from("cortex_cache/learned_styles.md");
        let _ = fs::create_dir_all("cortex_cache");
        let source_str = pattern.source_video.clone().unwrap_or_else(|| "Unknown/Generated".to_string());
        let entry = format!(
            "### {}\n- **Source Video**: {}\n- **Avg Scene Duration**: {:.2}s\n- **Transition Speed**: {:.2}\n- **Music Sync Strictness**: {:.2}\n- **Color Grade Style**: {}\n- **Success Rating**: {}\n\n",
            key, source_str, pattern.avg_scene_duration, pattern.transition_speed, pattern.music_sync_strictness, pattern.color_grade_style, pattern.success_rating
        );
        
        // Append to file
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&md_path) {
            let _ = file.write_all(entry.as_bytes());
        }
    }

    fn save(&self) {
        if let Ok(data) = serde_json::to_string_pretty(&self.patterns) {
            let _ = fs::write(&self.memory_path, data);
        }
    }
}
