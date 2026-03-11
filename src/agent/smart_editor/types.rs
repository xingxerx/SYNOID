use serde::{Deserialize, Serialize};
use regex::Captures;
use std::fs;
use tracing::{info, warn};
// SYNOID Smart Editor Refactoring

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EditDensity {
    Highlights, // Aggressive pruning (Original ruthless behavior)
    Balanced,   // Moderate pruning (Keep most meaningful content)
    Full,       // Minimal pruning (Only remove true silence/dead air)
}

impl Default for EditDensity {
    fn default() -> Self {
        Self::Balanced
    }
}

/// Configuration for the editing strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditingStrategy {
    pub scene_threshold: f64,
    pub min_scene_score: f64,
    pub boring_penalty_threshold: f64,
    pub speech_boost: f64,
    pub silence_penalty: f64,
    pub continuity_boost: f64,
    pub speech_ratio_threshold: f64,
    pub action_duration_threshold: f64,
    /// Maximum allowed gap (seconds) between two consecutive kept scenes.
    /// If a gap exceeds this, the best scene within the gap is inserted to
    /// prevent jarring narrative jumps. Default: 45.0 s.
    #[serde(default = "default_max_jump_gap_secs")]
    pub max_jump_gap_secs: f64,
}

fn default_max_jump_gap_secs() -> f64 {
    45.0
}

impl Default for EditingStrategy {
    fn default() -> Self {
        Self {
            scene_threshold: 0.25,
            min_scene_score: 0.30, // Raised from 0.20 — prevents micro-cuts
            boring_penalty_threshold: 25.0, // Tighter: cut long boring blocks sooner
            speech_boost: 0.5,     // Raised from 0.4 — speech is story, keep more of it
            silence_penalty: -0.4,
            continuity_boost: 0.6,
            speech_ratio_threshold: 0.1,
            action_duration_threshold: 3.0,
            max_jump_gap_secs: 45.0,
        }
    }
}

impl EditingStrategy {
    pub fn load() -> Self {
        // First try the learned, cortex-cached strategy (compounding learning)
        let suffix = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_default();
        let cache_dir = format!("cortex_cache{}", suffix);
        let cached_path = format!("{}/editing_strategy.json", cache_dir);
        if let Ok(content) = fs::read_to_string(&cached_path) {
            if let Ok(config) = serde_json::from_str(&content) {
                info!("[SMART] Loaded editing strategy from {}", cached_path);
                return config;
            }
        }

        // Fallback to static JSON
        if let Ok(content) = fs::read_to_string("editing_strategy.json") {
            if let Ok(config) = serde_json::from_str(&content) {
                info!("[SMART] Loaded editing strategy from editing_strategy.json");
                return config;
            }
        }

        info!("[SMART] Using default editing strategy");
        Self::default()
    }

    pub fn save_to_cortex(&self) {
        let suffix = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_default();
        let cache_dir = format!("cortex_cache{}", suffix);
        let _ = fs::create_dir_all(&cache_dir);
        let path = format!("{}/editing_strategy.json", cache_dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            match fs::write(&path, json) {
                Ok(_) => info!("[SMART] 💾 Saved tuned EditingStrategy to {}", path),
                Err(e) => warn!("[SMART] Failed to save strategy to cortex: {}", e),
            }
        }
    }
}

/// Represents an intent extracted from user input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditIntent {
    pub remove_boring: bool,
    pub keep_action: bool,
    pub remove_silence: bool,
    pub keep_speech: bool,
    pub ruthless: bool,
    pub density: EditDensity,
    pub custom_keywords: Vec<String>,
    pub target_duration: Option<(f64, f64)>,
    #[serde(default = "default_censor_profanity")]
    pub censor_profanity: bool,
    #[serde(default)]
    pub profanity_replacement: Option<String>,
    /// Show a brief [CUT] flash at every point where content was removed.
    /// Defaults to true; suppressed automatically when density == Full.
    #[serde(default = "default_show_cut_markers")]
    pub show_cut_markers: bool,
}

fn default_show_cut_markers() -> bool {
    false
}
fn default_censor_profanity() -> bool {
    true
}

impl EditIntent {
    /// Parse natural language intent into structured intent using LLM.
    /// Routes through Groq (fast model) when available, Ollama as fallback.
    pub async fn from_llm(text: &str) -> Self {
        use crate::agent::gpt_oss_bridge::SynoidAgent;
        // Use the multi-provider bridge; standard fast JSON intent parser
        let api_url = std::env::var("OLLAMA_API_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        // Uses Groq fast model via multi-provider when GROQ_API_KEY is set
        let agent = SynoidAgent::new(&api_url, "llama-3.1-8b-instant");

        let prompt = format!(
            r#"You are a video editing AI assistant. Convert the user's natural language request into a JSON configuration for the EditIntent struct.
The JSON must strictly follow this structure and include nothing else:
{{
    "remove_boring": bool,
    "keep_action": bool,
    "remove_silence": bool,
    "keep_speech": bool,
    "ruthless": bool,
    "density": "Highlights" | "Balanced" | "Full",
    "custom_keywords": [string],
    "target_duration": null or [min_secs_float, max_secs_float],
    "censor_profanity": bool,
    "profanity_replacement": null or string (e.g. "boing.wav")
}}

User Request: "{}"
"#,
            text
        );

        match agent.fast_reason(&prompt).await {
            Ok(response) => {
                // Extract the JSON object from the LLM response.
                // Llama3 often prefixes its answer with prose like "Here is the JSON configuration:"
                // so we search for the first {...} block instead of relying on the full string being JSON.
                let extracted = if let Some(mat) = regex::Regex::new(r"(?s)\{.*\}")
                    .ok()
                    .and_then(|re| re.find(response.trim()))
                {
                    mat.as_str()
                } else {
                    response.trim()
                };
                let clean_json = extracted
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                if let Ok(intent) = serde_json::from_str::<EditIntent>(clean_json) {
                    tracing::info!("[SMART] Successfully parsed EditIntent from LLM");
                    return intent;
                } else {
                    tracing::warn!("[SMART] LLM intent JSON deserialization failed, falling back to heuristic parsing. Raw: {}", clean_json);
                }
            }
            Err(e) => tracing::warn!(
                "[SMART] LLM intent parsing failed: {}, falling back to heuristic parsing",
                e
            ),
        }

        Self::from_text(text)
    }

    /// Parse natural language intent into structured intent
    pub fn from_text(text: &str) -> Self {
        let lower = text.to_lowercase();

        // Density detection
        let mut density = EditDensity::Balanced;

        let highlights_words = [
            "short",
            "highlights",
            "ruthless",
            "aggressive",
            "fast-paced",
            "quick",
            "snappy",
        ];
        let full_words = [
            "long",
            "full",
            "whole",
            "most",
            "minutes",
            "hour",
            "hours",
            "40-60",
            "exhaustive",
            "complete",
        ];

        if highlights_words.iter().any(|&w| lower.contains(w)) {
            density = EditDensity::Highlights;
        } else if full_words.iter().any(|&w| lower.contains(w)) {
            density = EditDensity::Full;
        }

        Self {
            show_cut_markers: default_show_cut_markers(),
            remove_boring: lower.contains("boring")
                || lower.contains("lame")
                || lower.contains("dull")
                || lower.contains("slow"),
            keep_action: lower.contains("action")
                || lower.contains("exciting")
                || lower.contains("fast")
                || lower.contains("intense")
                || lower.contains("engaging")
                || lower.contains("interesting")
                || lower.contains("viral clip"),
            remove_silence: lower.contains("silence")
                || lower.contains("quiet")
                || lower.contains("dead air")
                || lower.contains("silent parts")
                || lower.contains("viral clip"),
            keep_speech: lower.contains("speech")
                || lower.contains("talking")
                || lower.contains("dialogue")
                || lower.contains("conversation")
                || lower.contains("voice")
                || lower.contains("transcript")
                || lower.contains("engaging"),
            ruthless: lower.contains("ruthless")
                || lower.contains("aggressive")
                || lower.contains("fast-paced")
                || lower.contains("no filler")
                || lower.contains("remove all silence"),
            density,
            custom_keywords: vec![],
            target_duration: Self::parse_duration_range(&lower),
            censor_profanity: true, // Always-on: safety-first, never let slurs through
            profanity_replacement: if lower.contains("boing") {
                Some("boing.wav".to_string())
            } else if lower.contains("beep") || lower.contains("viral clip") {
                Some("beep.wav".to_string())
            } else if lower.contains("funny sound") || lower.contains("sound effect") {
                Some("boing.wav".to_string())
            } else {
                Some("beep.wav".to_string()) // Default to beep for slurs
            },
        }
    }

    fn parse_duration_range(text: &str) -> Option<(f64, f64)> {
        // Look for patterns like "40-60 minutes", "30 mins", "1 hour"
        // Return (min_seconds, max_seconds)

        let mut min_secs = 0.0;
        let mut max_secs = 0.0;

        // Simple case: "X-Y minutes"
        if let Some(caps) = regex::Regex::new(r"(\d+)-(\d+)\s*(min|minute|mins)")
            .ok()?
            .captures(text)
        {
            let caps: Captures = caps;
            min_secs = caps.get(1)?.as_str().parse::<f64>().ok()? * 60.0;
            max_secs = caps.get(2)?.as_str().parse::<f64>().ok()? * 60.0;
        } else if let Some(caps) = regex::Regex::new(r"(\d+)\s*(min|minute|mins)")
            .ok()?
            .captures(text)
        {
            let caps: Captures = caps;
            let mins = caps.get(1)?.as_str().parse::<f64>().ok()?;
            min_secs = mins * 60.0 * 0.9; // 10% tolerance
            max_secs = mins * 60.0 * 1.1;
        } else if let Some(caps) = regex::Regex::new(r"(\d+)\s*(hour|hr)").ok()?.captures(text) {
            let caps: Captures = caps;
            let hrs = caps.get(1)?.as_str().parse::<f64>().ok()?;
            min_secs = hrs * 3600.0 * 0.9;
            max_secs = hrs * 3600.0 * 1.1;
        }

        if max_secs > 0.0 {
            Some((min_secs, max_secs))
        } else {
            None
        }
    }

    /// Check if any editing intent was detected
    #[allow(dead_code)]
    pub fn has_intent(&self) -> bool {
        self.remove_boring
            || self.keep_action
            || self.remove_silence
            || self.keep_speech
            || self.ruthless
    }
}

/// Represents a detected scene in the video
#[derive(Debug, Clone)]
pub struct Scene {
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
    pub score: f64, // 0.0 = definitely remove, 1.0 = definitely keep
    pub vision_tags: Vec<String>,
}
