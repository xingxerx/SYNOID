// SYNOID Gemma 4 Self-Improvement Harness
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Gives Gemma 4 (via Ollama) a tool loop to read/write SYNOID source code,
// run cargo check/test, and iteratively build and improve the codebase.
//
// CLI: synoid-core gemma4 --task "improve smart_editor scene detection"

use reqwest::Client;
use serde_json::json;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

const SYSTEM_PROMPT: &str = r#"You are Gemma 4, an expert Rust engineer embedded inside SYNOID — an autonomous video editing AI.

Your mission: analyze, build, and improve the SYNOID Rust codebase. You have access to these tools:

--- TOOLS ---

ACTION: read_file
PATH: <relative path from project root, e.g. src/agent/brain.rs>

ACTION: list_files
DIR: <relative directory, e.g. src/agent/ai_systems>

ACTION: write_file
PATH: <relative path>
CONTENT:
<full file content here>
END_CONTENT

ACTION: search_code
PATTERN: <text or regex to search for in .rs files>

ACTION: cargo_check

ACTION: cargo_test
FILTER: <test name or "all">

ACTION: finish
SUMMARY: <what you accomplished>

--- RULES ---
- One ACTION per response — no skipping ahead
- Always READ a file before writing it (understand before changing)
- After every write_file, immediately run cargo_check
- Fix every compiler error before moving on
- Only write to files inside src/ (safety boundary)
- Be precise Rust — no pseudocode, no placeholder comments
- Think step by step before each action

Start by understanding the task fully, then execute."#;

#[derive(Debug)]
enum Action {
    ReadFile { path: String },
    ListFiles { dir: String },
    WriteFile { path: String, content: String },
    SearchCode { pattern: String },
    CargoCheck,
    CargoTest { filter: String },
    Finish { summary: String },
}

#[derive(Debug, Clone, serde::Serialize)]
struct Message {
    role: String,
    content: String,
}

pub struct Gemma4Harness {
    client: Client,
    ollama_url: String,
    model: String,
    work_dir: PathBuf,
    pub dry_run: bool,
}

impl Gemma4Harness {
    pub fn new(work_dir: &Path, dry_run: bool) -> Self {
        let ollama_url = std::env::var("SYNOID_API_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("SYNOID_MODEL").unwrap_or_else(|_| "gemma4:26b".to_string());

        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_default(),
            ollama_url,
            model,
            work_dir: work_dir.to_path_buf(),
            dry_run,
        }
    }

    /// Run a task — Gemma 4 iterates Thought → Action → Observation until finish.
    pub async fn run_task(&self, task: &str, max_steps: usize) -> Result<String, String> {
        info!("[GEMMA4] Starting task: {}", task);
        info!("[GEMMA4] Model: {} | Dry-run: {}", self.model, self.dry_run);

        let mut messages: Vec<Message> = vec![
            Message {
                role: "system".into(),
                content: SYSTEM_PROMPT.into(),
            },
            Message {
                role: "user".into(),
                content: format!("TASK: {}", task),
            },
        ];

        for step in 0..max_steps {
            info!("[GEMMA4] Step {}/{}", step + 1, max_steps);

            let response = self.call_gemma4(&messages).await?;
            println!("\n[Gemma4 Step {}]\n{}", step + 1, response);

            // Add assistant response to history
            messages.push(Message {
                role: "assistant".into(),
                content: response.clone(),
            });

            // Parse and execute the action
            match self.parse_action(&response) {
                Some(Action::Finish { summary }) => {
                    info!("[GEMMA4] Task complete: {}", summary);
                    return Ok(summary);
                }
                Some(action) => {
                    let observation = self.execute_action(action).await;
                    info!("[GEMMA4] Observation: {:.120}...", observation);
                    messages.push(Message {
                        role: "user".into(),
                        content: format!("OBSERVATION:\n{}", observation),
                    });
                }
                None => {
                    warn!("[GEMMA4] No valid ACTION found in response — prompting retry");
                    messages.push(Message {
                        role: "user".into(),
                        content: "No ACTION detected. Please respond with exactly one ACTION block as shown in the tool list.".into(),
                    });
                }
            }
        }

        Err(format!("Max steps ({}) reached without finishing", max_steps))
    }

    /// Parse the first ACTION block from Gemma 4's response.
    fn parse_action(&self, response: &str) -> Option<Action> {
        let lines: Vec<&str> = response.lines().collect();
        let action_idx = lines.iter().position(|l| l.trim().starts_with("ACTION:"))?;
        let action_type = lines[action_idx]
            .trim()
            .trim_start_matches("ACTION:")
            .trim()
            .to_lowercase();

        match action_type.as_str() {
            "read_file" => {
                let path = Self::find_field(&lines[action_idx..], "PATH:")?;
                Some(Action::ReadFile { path })
            }
            "list_files" => {
                let dir = Self::find_field(&lines[action_idx..], "DIR:")
                    .unwrap_or_else(|| "src".into());
                Some(Action::ListFiles { dir })
            }
            "write_file" => {
                let path = Self::find_field(&lines[action_idx..], "PATH:")?;
                let content = Self::extract_content_block(&lines[action_idx..])?;
                Some(Action::WriteFile { path, content })
            }
            "search_code" => {
                let pattern = Self::find_field(&lines[action_idx..], "PATTERN:")?;
                Some(Action::SearchCode { pattern })
            }
            "cargo_check" => Some(Action::CargoCheck),
            "cargo_test" => {
                let filter = Self::find_field(&lines[action_idx..], "FILTER:")
                    .unwrap_or_else(|| "all".into());
                Some(Action::CargoTest { filter })
            }
            "finish" => {
                let summary = Self::find_field(&lines[action_idx..], "SUMMARY:")
                    .unwrap_or_else(|| "Task complete".into());
                Some(Action::Finish { summary })
            }
            _ => None,
        }
    }

    fn find_field(lines: &[&str], prefix: &str) -> Option<String> {
        lines
            .iter()
            .find(|l| l.trim().starts_with(prefix))
            .map(|l| l.trim().trim_start_matches(prefix).trim().to_string())
    }

    fn extract_content_block(lines: &[&str]) -> Option<String> {
        let start = lines
            .iter()
            .position(|l| l.trim().starts_with("CONTENT:"))?;
        let end = lines
            .iter()
            .position(|l| l.trim() == "END_CONTENT")
            .unwrap_or(lines.len());
        // Skip the CONTENT: line itself and any opening code fence
        let body: Vec<&str> = lines[start + 1..end]
            .iter()
            .copied()
            .skip_while(|l| l.trim().starts_with("```"))
            .collect();
        // Strip trailing code fence
        let trimmed: Vec<&str> = if body.last().map(|l| l.trim().starts_with("```")).unwrap_or(false) {
            body[..body.len() - 1].to_vec()
        } else {
            body
        };
        Some(trimmed.join("\n"))
    }

    /// Execute an action and return the observation string.
    async fn execute_action(&self, action: Action) -> String {
        match action {
            Action::ReadFile { path } => self.tool_read_file(&path),
            Action::ListFiles { dir } => self.tool_list_files(&dir),
            Action::WriteFile { path, content } => self.tool_write_file(&path, &content).await,
            Action::SearchCode { pattern } => self.tool_search_code(&pattern),
            Action::CargoCheck => self.tool_cargo_check().await,
            Action::CargoTest { filter } => self.tool_cargo_test(&filter).await,
            Action::Finish { summary } => summary,
        }
    }

    // ─── Tool Implementations ───────────────────────────────────────────────

    fn tool_read_file(&self, path: &str) -> String {
        let full = self.safe_path(path);
        match full {
            None => format!("ERROR: Path '{}' is outside the project boundary.", path),
            Some(p) => match std::fs::read_to_string(&p) {
                Ok(content) => {
                    let lines: Vec<String> = content
                        .lines()
                        .enumerate()
                        .map(|(i, l)| format!("{:4}: {}", i + 1, l))
                        .collect();
                    // Cap at 400 lines to stay within context
                    let capped = lines[..lines.len().min(400)].join("\n");
                    if lines.len() > 400 {
                        format!("{}\n... ({} more lines)", capped, lines.len() - 400)
                    } else {
                        capped
                    }
                }
                Err(e) => format!("ERROR reading {}: {}", path, e),
            },
        }
    }

    fn tool_list_files(&self, dir: &str) -> String {
        let full = match self.safe_path(dir) {
            None => return format!("ERROR: Path '{}' is outside the project boundary.", dir),
            Some(p) => p,
        };
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&full) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let suffix = if entry.path().is_dir() { "/" } else { "" };
                out.push(format!("{}{}", name, suffix));
            }
        }
        out.sort();
        out.join("\n")
    }

    async fn tool_write_file(&self, path: &str, content: &str) -> String {
        if self.dry_run {
            return format!(
                "[DRY RUN] Would write {} bytes to '{}'",
                content.len(),
                path
            );
        }
        // Only allow .rs, .toml, .json, .md
        let allowed_ext = ["rs", "toml", "json", "md"];
        let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
        if !allowed_ext.contains(&ext) {
            return format!("ERROR: write_file only allows: {:?}", allowed_ext);
        }
        match self.safe_path(path) {
            None => format!("ERROR: Path '{}' is outside project src/ boundary.", path),
            Some(full) => {
                if let Some(parent) = full.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(&full, content) {
                    Ok(_) => format!("OK: Wrote {} bytes to '{}'", content.len(), path),
                    Err(e) => format!("ERROR writing '{}': {}", path, e),
                }
            }
        }
    }

    fn tool_search_code(&self, pattern: &str) -> String {
        let src = self.work_dir.join("src");
        let mut results = Vec::new();
        Self::walk_search(&src, pattern, &mut results, 0);
        if results.is_empty() {
            format!("No matches for '{}'", pattern)
        } else {
            results[..results.len().min(60)].join("\n")
        }
    }

    fn walk_search(dir: &Path, pattern: &str, results: &mut Vec<String>, depth: usize) {
        if depth > 8 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                Self::walk_search(&path, pattern, results, depth + 1);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for (i, line) in content.lines().enumerate() {
                        if line.contains(pattern) {
                            results.push(format!(
                                "{}:{}: {}",
                                path.display(),
                                i + 1,
                                line.trim()
                            ));
                        }
                    }
                }
            }
        }
    }

    async fn tool_cargo_check(&self) -> String {
        if self.dry_run {
            return "[DRY RUN] Would run: cargo check".into();
        }
        self.run_cargo(&["check", "--message-format=short"]).await
    }

    async fn tool_cargo_test(&self, filter: &str) -> String {
        if self.dry_run {
            return format!("[DRY RUN] Would run: cargo test {}", filter);
        }
        if filter == "all" || filter.is_empty() {
            self.run_cargo(&["test", "--lib", "--", "--test-output=immediate"])
                .await
        } else {
            self.run_cargo(&["test", "--lib", filter, "--", "--test-output=immediate"])
                .await
        }
    }

    async fn run_cargo(&self, args: &[&str]) -> String {
        use tokio::process::Command;
        match Command::new("cargo")
            .args(args)
            .current_dir(&self.work_dir)
            .output()
            .await
        {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let status = if out.status.success() { "OK" } else { "FAILED" };
                format!("[{}]\nstdout:\n{}\nstderr:\n{}", status, stdout, stderr)
            }
            Err(e) => format!("ERROR running cargo: {}", e),
        }
    }

    /// Resolve a relative path to an absolute path, enforcing project boundary.
    fn safe_path(&self, rel: &str) -> Option<PathBuf> {
        let rel = rel.trim_start_matches('/').trim_start_matches('\\');
        let full = self.work_dir.join(rel);
        // Canonicalize if possible; otherwise just normalize
        let canonical = full.canonicalize().unwrap_or(full);
        // Must be inside the work_dir
        if canonical.starts_with(&self.work_dir) {
            Some(canonical)
        } else {
            None
        }
    }

    // ─── Ollama Chat API ─────────────────────────────────────────────────────

    async fn call_gemma4(&self, messages: &[Message]) -> Result<String, String> {
        let base = self
            .ollama_url
            .trim_end_matches('/')
            .trim_end_matches("/v1");
        let endpoint = format!("{}/api/chat", base);

        let payload = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": 0.3,
                "num_predict": 2048
            }
        });

        let resp = self
            .client
            .post(&endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Ollama request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama error {}: {}", status, body));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let content = json["message"]["content"]
            .as_str()
            .unwrap_or("(empty response)")
            .to_string();

        Ok(content)
    }
}
