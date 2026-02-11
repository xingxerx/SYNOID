use crate::agent::gpt_oss_bridge::SynoidAgent;
use crate::agent::voice::TTSEngine;
use crate::agent::smart_editor::Scene;
use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{info, error};

pub struct CommentaryGenerator {
    agent: SynoidAgent,
    tts: TTSEngine,
}

impl CommentaryGenerator {
    pub fn new(api_url: &str) -> Result<Self> {
        Ok(Self {
            agent: SynoidAgent::new(api_url),
            tts: TTSEngine::new()?,
        })
    }

    /// Generate a funny comment for a specific scene context
    pub async fn generate_commentary(
        &self, 
        scene: &Scene, 
        context_text: &str,
        output_dir: &Path,
        index: usize
    ) -> Result<Option<PathBuf>> {
        // 1. Ask LLM for a funny one-liner
        let prompt = format!(
            "Watch this video segment ({:.1}s). Context: \"{}\". \
            Generate a VERY SHORT, sarcastic, or funny one-liner commentary (max 10 words) about this moment. \
            Do not describe the scene. Just react to it. \
            If nothing funny comes to mind, reply 'SKIP'.",
            scene.duration, context_text
        );

        let response = match self.agent.reason(&prompt).await {
            Ok(r) => r.trim().to_string(),
            Err(e) => {
                error!("[FUNNY] LLM failed: {}", e);
                return Ok(None);
            }
        };

        if response.to_uppercase().contains("SKIP") || response.len() < 2 {
            return Ok(None);
        }

        // Clean quotes
        let clean_text = response.replace("\"", "").replace("*", "");
        
        info!("[FUNNY] Generated command: \"{}\"", clean_text);

        // 2. Synthesize Audio
        let filename = format!("commentary_{}.mp3", index);
        let output_path = output_dir.join(&filename);

        self.tts.speak(&clean_text, &output_path, None).await?;

        Ok(Some(output_path))
    }
}
