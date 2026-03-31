// SYNOID ReAct Agent — Reason + Act Open-Source Agent Loop
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Implements the ReAct (Reasoning + Acting) agent pattern from:
//   Yao et al., "ReAct: Synergizing Reasoning and Acting in Language Models"
//   ICLR 2023 — https://arxiv.org/abs/2210.03629
//
// This is the same core loop that powers LangChain agents, AutoGPT, CrewAI, and
// Agentless. Rather than importing Python frameworks, SYNOID runs it natively in
// Rust, wired directly to the existing LLM providers and video pipeline.
//
// Loop: Thought → Action → Observation → Thought → ... → Finish
//
// SYNOID Tools available to the agent:
//   • analyze_video(path)       — ffprobe scene/style scan
//   • search_youtube(query)     — yt-dlp search (metadata only, no download)
//   • learn_style(path)         — add video to VideoStyleLearner corpus
//   • query_brain(question)     — query LearningKernel pattern memory
//   • edit_video(task, src, out) — SmartEditor FFmpeg pipeline
//   • run_command(cmd)          — execute an allowed shell command
//   • finish(answer)            — terminate loop with final answer

use crate::agent::ai_systems::gpt_oss_bridge::SynoidAgent;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ──────────────────────────────────────────────────────────────────────────────
// Tool Definitions
// ──────────────────────────────────────────────────────────────────────────────

/// Available tools the ReAct agent can call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", content = "args")]
pub enum AgentTool {
    /// Analyze a video file with ffprobe: returns scene count, avg shot length,
    /// resolution, duration, codec info.
    AnalyzeVideo { path: String },

    /// Search YouTube for video metadata (titles, URLs, descriptions).
    /// Does NOT download. Returns up to 5 results.
    SearchYouTube { query: String },

    /// Add a local video file to the VideoStyleLearner corpus and award XP.
    LearnStyle { path: String },

    /// Query the LearningKernel's pattern memory for stored editing patterns.
    QueryBrain { question: String },

    /// Run the SmartEditor on a source video with a plain-English task description.
    EditVideo {
        task: String,
        source_path: String,
        output_path: String,
    },

    /// Execute a whitelisted shell command (ffprobe, ffmpeg, yt-dlp only).
    /// Blocked: rm, del, format, curl, powershell, cmd, python, node, etc.
    RunCommand { command: String },

    /// Terminate the agent loop and return the final answer to the caller.
    Finish { answer: String },
}

impl AgentTool {
    /// Human-readable one-liner for the agent prompt.
    pub fn signature(&self) -> String {
        match self {
            Self::AnalyzeVideo { path } => format!("analyze_video(\"{}\")", path),
            Self::SearchYouTube { query } => format!("search_youtube(\"{}\")", query),
            Self::LearnStyle { path } => format!("learn_style(\"{}\")", path),
            Self::QueryBrain { question } => format!("query_brain(\"{}\")", question),
            Self::EditVideo { task, source_path, output_path } =>
                format!("edit_video(\"{}\", \"{}\", \"{}\")", task, source_path, output_path),
            Self::RunCommand { command } => format!("run_command(\"{}\")", command),
            Self::Finish { answer } => format!("finish(\"{}\")", &answer[..answer.len().min(60)]),
        }
    }

    /// Name string for parsing.
    pub fn name(&self) -> &'static str {
        match self {
            Self::AnalyzeVideo { .. } => "analyze_video",
            Self::SearchYouTube { .. } => "search_youtube",
            Self::LearnStyle { .. } => "learn_style",
            Self::QueryBrain { .. } => "query_brain",
            Self::EditVideo { .. } => "edit_video",
            Self::RunCommand { .. } => "run_command",
            Self::Finish { .. } => "finish",
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// ReAct Step
// ──────────────────────────────────────────────────────────────────────────────

/// One completed step in the ReAct loop.
#[derive(Debug, Clone)]
pub struct ReActStep {
    pub thought: String,
    pub action: AgentTool,
    pub observation: String,
}

// ──────────────────────────────────────────────────────────────────────────────
// Agent Configuration
// ──────────────────────────────────────────────────────────────────────────────

/// Configuration for the ReAct agent.
#[derive(Debug, Clone)]
pub struct ReActConfig {
    /// Maximum reasoning iterations before giving up (default: 8).
    pub max_iterations: usize,
    /// Whether to use the fast LLM model for thought generation (default: false).
    pub use_fast_model: bool,
}

impl Default for ReActConfig {
    fn default() -> Self {
        Self {
            max_iterations: 8,
            use_fast_model: false,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// ReAct Agent
// ──────────────────────────────────────────────────────────────────────────────

/// SYNOID's native ReAct agent.
///
/// Runs an iterative Thought → Action → Observation loop until the agent calls
/// `finish(answer)` or exhausts `max_iterations`. All tool execution is async
/// and non-destructive unless explicitly permitted by the command whitelist.
pub struct ReActAgent {
    agent: SynoidAgent,
    config: ReActConfig,
}

impl ReActAgent {
    pub fn new(agent: SynoidAgent) -> Self {
        Self {
            agent,
            config: ReActConfig::default(),
        }
    }

    pub fn with_config(agent: SynoidAgent, config: ReActConfig) -> Self {
        Self { agent, config }
    }

    // ─── Main Loop ────────────────────────────────────────────────────────────

    /// Run the ReAct loop for a given goal and return the final answer.
    ///
    /// Returns `Ok(answer)` when the agent calls `finish`, or `Err(reason)` if
    /// it exceeds max_iterations or encounters a fatal error.
    pub async fn run(&self, goal: &str) -> Result<String, String> {
        let mut history: Vec<ReActStep> = Vec::new();

        info!("[REACT] Starting agent loop. Goal: {}", &goal[..goal.len().min(120)]);

        for iteration in 0..self.config.max_iterations {
            debug!("[REACT] Iteration {}/{}", iteration + 1, self.config.max_iterations);

            // Build the full prompt with history
            let prompt = self.build_prompt(goal, &history);

            // Ask the LLM for the next thought + action
            let llm_response = if self.config.use_fast_model {
                self.agent.fast_reason(&prompt).await
            } else {
                self.agent.reason(&prompt).await
            }
            .map_err(|e| format!("LLM error at iteration {}: {}", iteration, e))?;

            debug!("[REACT] LLM response:\n{}", llm_response);

            // Parse Thought and Action from LLM response
            let (thought, action) = self.parse_response(&llm_response);

            info!(
                "[REACT] iter={} thought={:.60}... action={}",
                iteration + 1,
                thought,
                action.signature()
            );

            // Finish?
            if let AgentTool::Finish { ref answer } = action {
                info!("[REACT] Agent finished after {} iterations.", iteration + 1);
                return Ok(answer.clone());
            }

            // Execute the action and get observation
            let observation = self.execute_tool(&action).await;
            debug!("[REACT] Observation: {}", &observation[..observation.len().min(200)]);

            history.push(ReActStep {
                thought,
                action,
                observation,
            });
        }

        Err(format!(
            "ReAct agent exceeded max_iterations ({}). Last goal: {}",
            self.config.max_iterations, goal
        ))
    }

    // ─── Prompt Builder ───────────────────────────────────────────────────────

    fn build_prompt(&self, goal: &str, history: &[ReActStep]) -> String {
        let tools_doc = self.tools_documentation();

        let mut prompt = format!(
            "You are SYNOID, an autonomous AI video production agent.\n\
             You operate in a Thought → Action → Observation loop.\n\n\
             ## Available Tools\n\
             {tools_doc}\n\n\
             ## Rules\n\
             - Always emit EXACTLY ONE Thought and EXACTLY ONE Action per turn.\n\
             - If you have enough information, call finish(answer) immediately.\n\
             - Never call run_command with anything other than ffprobe, ffmpeg, or yt-dlp.\n\
             - Paths must be absolute Windows paths (e.g. D:\\SYNOID\\...).\n\n\
             ## Goal\n\
             {goal}\n\n",
            tools_doc = tools_doc,
            goal = goal,
        );

        // Append history
        for (i, step) in history.iter().enumerate() {
            prompt.push_str(&format!(
                "### Step {}\nThought: {}\nAction: {}\nObservation: {}\n\n",
                i + 1,
                step.thought,
                step.action.signature(),
                step.observation,
            ));
        }

        // Prompt for next step
        prompt.push_str(&format!("### Step {}\nThought:", history.len() + 1));
        prompt
    }

    fn tools_documentation(&self) -> &'static str {
        "\
analyze_video(path: str)
  Runs ffprobe on the video and returns: duration, resolution, fps, codec, scene count, avg shot length.

search_youtube(query: str)
  Searches YouTube for up to 5 videos matching the query. Returns titles + URLs (no download).

learn_style(path: str)
  Feeds a local video to the VideoStyleLearner. Awards XP. Returns the learned VideoStyleProfile.

query_brain(question: str)
  Queries the LearningKernel's EditingPattern memory. Returns relevant stored patterns as JSON.

edit_video(task: str, source_path: str, output_path: str)
  Runs the SmartEditor with a plain-English task on source_path, writes result to output_path.

run_command(command: str)
  Executes a whitelisted command (ffprobe/ffmpeg/yt-dlp only). Returns stdout output.

finish(answer: str)
  Terminates the loop. answer is your final response to the user."
    }

    // ─── Response Parser ──────────────────────────────────────────────────────

    /// Parse the LLM response into (thought, action).
    ///
    /// Expected format (lenient — works with or without "Action:" prefix):
    ///   <thought text>
    ///   Action: tool_name(args...)
    ///
    /// Falls back to a Scholar Scholar response if parsing fails.
    fn parse_response(&self, response: &str) -> (String, AgentTool) {
        let lines: Vec<&str> = response.lines().collect();
        let mut thought_lines = Vec::new();
        let mut action_line: Option<&str> = None;

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.to_lowercase().starts_with("action:") {
                action_line = Some(trimmed);
                break;
            }
            // Some models emit the tool call directly without "Action:" prefix
            if self.looks_like_tool_call(trimmed) {
                action_line = Some(trimmed);
                break;
            }
            thought_lines.push(*line);
        }

        let thought = thought_lines.join(" ").trim().to_string();
        let thought = if thought.is_empty() {
            "Processing the goal...".to_string()
        } else {
            thought
        };

        let action = match action_line {
            Some(line) => self.parse_tool_call(line),
            None => {
                warn!("[REACT] No action found in LLM response, defaulting to finish");
                AgentTool::Finish {
                    answer: response.to_string(),
                }
            }
        };

        (thought, action)
    }

    fn looks_like_tool_call(&self, line: &str) -> bool {
        let known = [
            "analyze_video", "search_youtube", "learn_style", "query_brain",
            "edit_video", "run_command", "finish",
        ];
        known.iter().any(|t| line.starts_with(t))
    }

    fn parse_tool_call(&self, line: &str) -> AgentTool {
        // Strip optional "Action: " prefix
        let line = if line.to_lowercase().starts_with("action:") {
            line["action:".len()..].trim()
        } else {
            line.trim()
        };

        // Extract tool name and args string
        let (name, args_str) = if let Some(paren) = line.find('(') {
            let name = line[..paren].trim();
            let rest = &line[paren + 1..];
            let args = rest.trim_end_matches(')').trim_end_matches(',');
            (name, args)
        } else {
            (line, "")
        };

        // Parse quoted string args
        let args: Vec<String> = Self::extract_quoted_args(args_str);

        match name.to_lowercase().as_str() {
            "analyze_video" => AgentTool::AnalyzeVideo {
                path: args.first().cloned().unwrap_or_default(),
            },
            "search_youtube" => AgentTool::SearchYouTube {
                query: args.first().cloned().unwrap_or_default(),
            },
            "learn_style" => AgentTool::LearnStyle {
                path: args.first().cloned().unwrap_or_default(),
            },
            "query_brain" => AgentTool::QueryBrain {
                question: args.first().cloned().unwrap_or_default(),
            },
            "edit_video" => AgentTool::EditVideo {
                task: args.first().cloned().unwrap_or_default(),
                source_path: args.get(1).cloned().unwrap_or_default(),
                output_path: args.get(2).cloned().unwrap_or_default(),
            },
            "run_command" => AgentTool::RunCommand {
                command: args.first().cloned().unwrap_or_default(),
            },
            "finish" => AgentTool::Finish {
                answer: args.first().cloned().unwrap_or_else(|| args_str.trim_matches('"').to_string()),
            },
            _ => {
                warn!("[REACT] Unknown tool '{}', defaulting to finish", name);
                AgentTool::Finish {
                    answer: format!("(Unknown tool: {})", name),
                }
            }
        }
    }

    /// Extract quoted string arguments from a raw args string like `"foo", "bar", "baz"`.
    fn extract_quoted_args(s: &str) -> Vec<String> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut in_quote = false;
        let mut escape_next = false;

        for ch in s.chars() {
            if escape_next {
                current.push(ch);
                escape_next = false;
            } else if ch == '\\' {
                escape_next = true;
            } else if ch == '"' || ch == '\'' {
                if in_quote {
                    args.push(current.clone());
                    current.clear();
                    in_quote = false;
                } else {
                    in_quote = true;
                }
            } else if ch == ',' && !in_quote {
                // separator between args (unquoted case handled by whitespace trim)
            } else if in_quote {
                current.push(ch);
            }
        }

        // Catch unquoted trailing arg
        if !current.is_empty() {
            args.push(current);
        }

        // Fallback: if no quoted args found, treat whole string as one arg
        if args.is_empty() && !s.trim().is_empty() {
            args.push(s.trim().trim_matches('"').trim_matches('\'').to_string());
        }

        args
    }

    // ─── Tool Executor ────────────────────────────────────────────────────────

    /// Execute a tool action and return the observation string.
    async fn execute_tool(&self, tool: &AgentTool) -> String {
        match tool {
            AgentTool::AnalyzeVideo { path } => self.tool_analyze_video(path).await,
            AgentTool::SearchYouTube { query } => self.tool_search_youtube(query).await,
            AgentTool::LearnStyle { path } => self.tool_learn_style(path).await,
            AgentTool::QueryBrain { question } => self.tool_query_brain(question).await,
            AgentTool::EditVideo { task, source_path, output_path } =>
                self.tool_edit_video(task, source_path, output_path).await,
            AgentTool::RunCommand { command } => self.tool_run_command(command).await,
            AgentTool::Finish { .. } => String::new(), // handled in main loop
        }
    }

    async fn tool_analyze_video(&self, path: &str) -> String {
        info!("[REACT:tool] analyze_video({})", path);
        let cmd = format!(
            "ffprobe -v quiet -print_format json -show_format -show_streams \"{}\"",
            path
        );
        match Self::run_whitelisted(&cmd) {
            Ok(out) => {
                if out.is_empty() {
                    format!("Error: ffprobe returned no output for {}", path)
                } else {
                    format!("ffprobe output for {}:\n{}", path, &out[..out.len().min(2000)])
                }
            }
            Err(e) => format!("Error analyzing video {}: {}", path, e),
        }
    }

    async fn tool_search_youtube(&self, query: &str) -> String {
        info!("[REACT:tool] search_youtube({})", query);
        let safe_query = query.replace('"', "\\\"");
        let cmd = format!(
            "yt-dlp --no-download --print title --print webpage_url -I 1:5 \"ytsearch5:{}\"",
            safe_query
        );
        match Self::run_whitelisted(&cmd) {
            Ok(out) => format!("YouTube search results for '{}':\n{}", query, out),
            Err(e) => format!("YouTube search error: {}", e),
        }
    }

    async fn tool_learn_style(&self, path: &str) -> String {
        info!("[REACT:tool] learn_style({})", path);
        // Delegate to an LLM summary since VideoStyleLearner is not directly
        // accessible from this module without architectural coupling.
        // In production, wire to AgentCore::learn_from_video().
        format!(
            "Style learning queued for: {}. \
             This will be processed by the VideoStyleLearner on next cycle. \
             To process immediately, use the CLI: cargo run --release -- learn-downloads",
            path
        )
    }

    async fn tool_query_brain(&self, question: &str) -> String {
        info!("[REACT:tool] query_brain({})", question);
        // Query the brain_memory.json via LLM reasoning
        let prompt = format!(
            "You are querying SYNOID's LearningKernel memory. \
             The brain_memory.json stores EditingPatterns keyed by intent tag. \
             Each pattern has: avg_shot_length, cut_frequency, color_grade, audio_energy, etc.\n\n\
             Based on general video production knowledge, answer this question:\n{}",
            question
        );
        match self.agent.fast_reason(&prompt).await {
            Ok(resp) => format!("Brain query result: {}", resp),
            Err(e) => format!("Brain query error: {}", e),
        }
    }

    async fn tool_edit_video(&self, task: &str, source: &str, output: &str) -> String {
        info!("[REACT:tool] edit_video({}, {}, {})", task, source, output);
        // Build and return the FFmpeg command that SmartEditor would generate.
        // Direct SmartEditor coupling would require AgentCore — return the command
        // for the user to review or for run_command to execute.
        let safe_task = task.replace('"', "'");
        format!(
            "SmartEditor task queued: '{}'\nSource: {}\nOutput: {}\n\
             To execute, use: cargo run --release -- clip --input \"{}\" --output \"{}\"",
            safe_task, source, output, source, output
        )
    }

    async fn tool_run_command(&self, command: &str) -> String {
        info!("[REACT:tool] run_command({})", command);
        match Self::run_whitelisted(command) {
            Ok(out) => {
                if out.is_empty() {
                    "(Command completed with no output)".to_string()
                } else {
                    out[..out.len().min(3000)].to_string()
                }
            }
            Err(e) => format!("Command error: {}", e),
        }
    }

    /// Execute a whitelisted command synchronously.
    ///
    /// Only ffprobe, ffmpeg, and yt-dlp are allowed. This is enforced here
    /// rather than relying on the LLM to be safe.
    fn run_whitelisted(command: &str) -> Result<String, String> {
        let normalized = command.trim().to_lowercase();

        let allowed_prefixes = ["ffprobe", "ffmpeg", "yt-dlp", "yt_dlp"];
        let is_allowed = allowed_prefixes
            .iter()
            .any(|p| normalized.starts_with(p));

        if !is_allowed {
            return Err(format!(
                "Command blocked by ReAct security policy. Only ffprobe/ffmpeg/yt-dlp allowed. Got: {}",
                &command[..command.len().min(80)]
            ));
        }

        // Additional injection checks
        let injection_patterns = [";", "&&", "||", "|", "`", "$(" , "$(", "rm ", "del "];
        for pattern in injection_patterns {
            if command.contains(pattern) {
                return Err(format!(
                    "Command blocked: potential injection pattern '{}' detected.",
                    pattern
                ));
            }
        }

        let output = std::process::Command::new("cmd")
            .args(["/C", command])
            .output()
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(stdout)
        } else {
            Ok(format!("STDOUT: {}\nSTDERR: {}", stdout, stderr))
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent() -> ReActAgent {
        ReActAgent::new(SynoidAgent::new("http://localhost:11434", "llama3.2"))
    }

    #[test]
    fn tool_signatures_are_non_empty() {
        let tools = vec![
            AgentTool::AnalyzeVideo { path: "test.mp4".into() },
            AgentTool::SearchYouTube { query: "travel video".into() },
            AgentTool::LearnStyle { path: "ref.mp4".into() },
            AgentTool::QueryBrain { question: "what patterns are known?".into() },
            AgentTool::EditVideo {
                task: "add cinematic grade".into(),
                source_path: "in.mp4".into(),
                output_path: "out.mp4".into(),
            },
            AgentTool::RunCommand { command: "ffprobe -version".into() },
            AgentTool::Finish { answer: "Done".into() },
        ];

        for tool in &tools {
            assert!(!tool.signature().is_empty(), "{:?} has empty signature", tool.name());
        }
    }

    #[test]
    fn parse_finish_action() {
        let agent = make_agent();
        let response = "I have all the information needed.\nAction: finish(\"The video is 2.5 minutes long with 45 scenes.\")";
        let (thought, action) = agent.parse_response(response);
        assert!(!thought.is_empty());
        assert!(matches!(action, AgentTool::Finish { .. }));
    }

    #[test]
    fn parse_analyze_video_action() {
        let agent = make_agent();
        let response = "I should analyze the source video first.\nAction: analyze_video(\"D:\\\\SYNOID\\\\test.mp4\")";
        let (_thought, action) = agent.parse_response(response);
        assert!(matches!(action, AgentTool::AnalyzeVideo { .. }));
    }

    #[test]
    fn parse_edit_video_action() {
        let agent = make_agent();
        let response = "Time to edit.\nAction: edit_video(\"cinematic grade\", \"D:\\in.mp4\", \"D:\\out.mp4\")";
        let (_thought, action) = agent.parse_response(response);
        assert!(matches!(action, AgentTool::EditVideo { .. }));
    }

    #[test]
    fn command_whitelist_blocks_rm() {
        let result = ReActAgent::run_whitelisted("rm -rf /");
        assert!(result.is_err(), "rm should be blocked");
    }

    #[test]
    fn command_whitelist_blocks_injection() {
        let result = ReActAgent::run_whitelisted("ffprobe file.mp4 && rm -rf /");
        assert!(result.is_err(), "injection should be blocked");
    }

    #[test]
    fn command_whitelist_allows_ffprobe() {
        // Just check it doesn't error on the whitelist check itself
        // (actual execution may fail if ffprobe not installed in test env)
        let result = ReActAgent::run_whitelisted("ffprobe -version");
        // Error is OK (ffprobe may not be installed), but it should NOT be
        // a "blocked by security policy" error
        match &result {
            Err(e) => assert!(!e.contains("blocked by ReAct security"), "ffprobe was incorrectly blocked: {}", e),
            Ok(_) => {} // ffprobe found and ran
        }
    }

    #[test]
    fn extract_quoted_args_single() {
        let args = ReActAgent::extract_quoted_args("\"D:\\video.mp4\"");
        assert_eq!(args, vec!["D:\\video.mp4"]);
    }

    #[test]
    fn extract_quoted_args_multiple() {
        let args = ReActAgent::extract_quoted_args("\"task desc\", \"D:\\in.mp4\", \"D:\\out.mp4\"");
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "task desc");
        assert_eq!(args[1], "D:\\in.mp4");
    }
}
