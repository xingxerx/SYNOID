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
    pub success_rating: u32, // 1-5 stars from user
    pub source_video: Option<String>,
    /// Fraction of scenes kept during the edit (0.0 = cut everything, 1.0 = kept everything).
    /// Ideal range is 0.3–0.7 for a balanced edit.
    #[serde(default = "default_kept_ratio")]
    pub kept_ratio: f64,
    /// Quality weight derived from balance of kept_ratio and user rating.
    /// 1.0 = perfect edit, 0.1 = poor edit. Used to prefer high-quality patterns.
    #[serde(default = "default_outcome_xp")]
    pub outcome_xp: f64,
}

fn default_kept_ratio() -> f64 {
    0.5
}
fn default_outcome_xp() -> f64 {
    1.0
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
            kept_ratio: 0.5,
            outcome_xp: 1.0,
        }
    }
}

pub struct LearningKernel {
    memory_path: PathBuf,
    patterns: HashMap<String, EditingPattern>,
}

impl LearningKernel {
    pub fn new() -> Self {
        let suffix = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_default();
        let cache_dir = format!("cortex_cache{}", suffix);
        let _ = fs::create_dir_all(&cache_dir);
        let path = PathBuf::from(&cache_dir).join("brain_memory.json");
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

    /// Retrieve the best known editing pattern for a user intent.
    /// Prefers patterns with higher `outcome_xp` (quality score) when multiple match.
    pub fn recall_pattern(&self, intent: &str) -> EditingPattern {
        let intent_lower = intent.to_lowercase();

        // 1. Direct Match
        if let Some(pattern) = self.patterns.get(intent) {
            info!(
                "[KERNEL] 🧠 Exact match pattern for '{}' (XP: {:.2})",
                intent, pattern.outcome_xp
            );
            return pattern.clone();
        }

        // 2. Keyword Match — collect ALL fuzzy matches and pick the highest quality one
        let mut fuzzy_matches: Vec<&EditingPattern> = Vec::new();
        for (key, pattern) in &self.patterns {
            if intent_lower.contains(key.as_str()) || key.contains(&intent_lower) {
                fuzzy_matches.push(pattern);
            }
        }
        if !fuzzy_matches.is_empty() {
            // Sort descending by outcome_xp then by success_rating
            fuzzy_matches.sort_by(|a, b| {
                b.outcome_xp
                    .partial_cmp(&a.outcome_xp)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.success_rating.cmp(&a.success_rating))
            });
            let best = fuzzy_matches[0];
            info!(
                "[KERNEL] 🧠 Best fuzzy match for '{}': '{}' (XP: {:.2})",
                intent, best.intent_tag, best.outcome_xp
            );
            return best.clone();
        }

        // 3. Fallback Heuristics
        let key = if intent_lower.contains("hype") || intent_lower.contains("fast") {
            "fast_paced"
        } else if intent_lower.contains("cinematic") || intent_lower.contains("slow") {
            "cinematic"
        } else {
            "general"
        };

        if let Some(pattern) = self.patterns.get(key) {
            info!(
                "[KERNEL] 🧠 Fallback heuristic for '{}': '{}' (XP: {:.2})",
                intent, key, pattern.outcome_xp
            );
            return pattern.clone();
        }

        // 4. Ultimate fallback: highest quality (outcome_xp) learned pattern
        if let Some((best_key, best_pattern)) = self
            .patterns
            .iter()
            .filter(|(_, p)| p.intent_tag != "general" && p.outcome_xp >= 0.6)
            .max_by(|a, b| {
                a.1.outcome_xp
                    .partial_cmp(&b.1.outcome_xp)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            info!(
                "[KERNEL] 🧠 Applying best learned knowledge from '{}' (XP: {:.2}).",
                best_key, best_pattern.outcome_xp
            );
            return best_pattern.clone();
        }

        info!("[KERNEL] No learned patterns. Using defaults.");
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
        let suffix = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_default();
        let cache_dir = format!("cortex_cache{}", suffix);
        let md_path = PathBuf::from(&cache_dir).join("learned_styles.md");
        let _ = fs::create_dir_all(&cache_dir);
        let source_str = pattern
            .source_video
            .clone()
            .unwrap_or_else(|| "Unknown/Generated".to_string());
        let entry = format!(
            "### {}\n- **Source Video**: {}\n- **Avg Scene Duration**: {:.2}s\n- **Transition Speed**: {:.2}\n- **Music Sync Strictness**: {:.2}\n- **Color Grade Style**: {}\n- **Success Rating**: {}★\n- **Kept Ratio**: {:.1}%\n- **Outcome XP**: {:.2}\n\n",
            key, source_str, pattern.avg_scene_duration, pattern.transition_speed, pattern.music_sync_strictness,
            pattern.color_grade_style, pattern.success_rating,
            pattern.kept_ratio * 100.0, pattern.outcome_xp
        );

        // Append to file
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&md_path)
        {
            let _ = file.write_all(entry.as_bytes());
        }
    }

    fn save(&self) {
        if let Ok(data) = serde_json::to_string_pretty(&self.patterns) {
            let _ = fs::write(&self.memory_path, data);
        }
    }
}
