// SYNOID Code Scanner
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use crate::agent::gpt_oss_bridge::SynoidAgent;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzedConcept {
    pub source_repo: String,
    pub concept: String,   // "Bézier Curve Interpolation"
    pub file_type: String, // "cpp", "python"
    pub logic_summary: String,
    pub confidence: f32,
}

pub struct CodeScanner {
    agent: SynoidAgent,
}

impl CodeScanner {
    pub fn new(api_url: &str) -> Self {
        Self {
            agent: SynoidAgent::new(api_url, "gpt-oss:20b"),
        }
    }

    /// Stealthily scan a repository file URL for editing logic
    /// This fetches the raw content in-memory, processes it, and discards the code.
    pub async fn scan_remote_code(
        &self,
        url: &str,
    ) -> Result<AnalyzedConcept, Box<dyn std::error::Error + Send + Sync>> {
        info!("[SCANNER] 🕵️ Stealthily accessing: {}", url);

        // 1. Fetch raw content (In-Memory Only)
        // Convert github blob URL to raw if necessary, or assume raw input
        let raw_url = if url.contains("github.com") && url.contains("/blob/") {
            url.replace("github.com", "raw.githubusercontent.com")
                .replace("/blob/", "/")
        } else {
            url.to_string()
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let resp = client.get(&raw_url).send().await?;
        if !resp.status().is_success() {
            return Err(format!(
                "Failed to fetch code from {}: Status {}",
                raw_url,
                resp.status()
            )
            .into());
        }

        let code_content = resp.text().await?;
        let code_len = code_content.len();

        // 2. Filter for relevance (Client-side heuristic)
        // If file is too huge or binary, skip
        if code_len > 100_000 || code_content.contains('\0') {
            return Err("File too large or binary".into());
        }

        // 3. Extract Conceptual Logic (LLM)
        // We do strictly extraction of *math* or *logic*, no copy-paste.
        info!("[SCANNER] 🧠 Distilling logic from {} bytes...", code_len);

        // Truncate for context window
        let snippet = if code_len > 3000 {
            &code_content[..3000]
        } else {
            &code_content
        };

        let prompt = format!(
            "Analyze this code snippet (Source: {}) to understand the underlying video editing algorithm.\n\
            Extract ONLY the mathematical concept or logic rule (e.g., 'Use Catmull-Rom splines for smooth keyframes').\n\
            DO NOT output any code. Output a single sentence summary.\n\n\
            Code:\n```\n{}\n```",
            url, snippet
        );

        let logic = self
            .agent
            .reason(&prompt)
            .await
            .map_err(|e| {
                warn!(
                    "[SCANNER] ⚠️ Reasoning failed: {}. Falling back to default concept.",
                    e
                );
                e
            })
            .unwrap_or_else(|_| "Algorithmic logic distilled from source code.".to_string());

        let file_ext = Url::parse(url)?
            .path_segments()
            .and_then(|check| check.last())
            .and_then(|name| name.split('.').last())
            .unwrap_or("unknown")
            .to_string();

        Ok(AnalyzedConcept {
            source_repo: url.to_string(),
            concept: "Algorithmic Logic".to_string(),
            file_type: file_ext,
            logic_summary: logic,
            confidence: 0.85,
        })
    }
}
