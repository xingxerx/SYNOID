// SYNOID AutoImprove — Self-Recursing Editing Strategy Optimizer
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Inspired by karpathy/autoresearch: instead of running overnight ML experiments,
// SYNOID autonomously mutates EditingStrategy parameters, evaluates them on a
// benchmark video (dry-run: scene detect + score, no FFmpeg render), and keeps
// improvements — compounding better edits over time.
//
// Analogies to autoresearch:
//   train.py         →  EditingStrategy (the thing being mutated)
//   program.md       →  cortex_cache/improve_program.md (human guidance)
//   val_bpb metric   →  composite quality score (balance + discrimination)
//   5-min experiment →  dry-run scene scoring (seconds, no encode)
//   overnight run    →  runs in background, wakes up better each loop

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

use crate::agent::specialized::smart_editor::{
    detect_scenes, score_scenes, EditDensity, EditIntent, EditingStrategy,
};
use crate::agent::core_systems::neuroplasticity::Neuroplasticity;

// ─── AI Strategy Advisor ──────────────────────────────────────────────────────

use crate::agent::ai_systems::gpt_oss_bridge::SynoidAgent;

pub struct LlmStrategyAdvisor {
    agent: SynoidAgent,
}

impl LlmStrategyAdvisor {
    pub fn new() -> Self {
        let api_url = std::env::var("OLLAMA_API_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        // Use a reasoning model for strategy generation
        Self {
            agent: SynoidAgent::new(&api_url, "llama-3.3-70b-versatile"), // Groq default if available
        }
    }

    /// Ask the LLM to generate 4 improved strategies based on history and hints.
    async fn generate_candidates(
        &self,
        baseline: &EditingStrategy,
        history: &[ExperimentResult],
        hints: &MutationHints,
    ) -> Result<Vec<EditingStrategy>, String> {
        let history_json = serde_json::to_string_pretty(&history.iter().take(5).collect::<Vec<_>>()).unwrap_or_default();
        let baseline_json = serde_json::to_string_pretty(baseline).unwrap_or_default();

        // 🧠 Compounding Learning: List existing skills for the LLM to consider
        let skills_dir = ".agent/skills";
        let mut skill_list = Vec::new();
        if let Ok(entries) = std::fs::read_dir(skills_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".json") {
                        skill_list.push(name.replace(".json", ""));
                    }
                }
            }
        }
        let available_skills = if skill_list.is_empty() {
            "None yet.".to_string()
        } else {
            skill_list.join(", ")
        };

        let prompt = format!(
            r#"You are the SYNOID Strategy Optimizer. Your goal is to improve video editing quality scores.

### PERFORMANCE HISTORY (Last 5 Experiments)
{}

### CURRENT PROJECT BASELINE
{}

### SKILLS LIBRARY (Already discovered high-performing strategies)
{}

### HUMAN GUIDANCE / HINTS
- Increase: {:?}
- Decrease: {:?}
- Preserve: {:?}
- Notes: {:?}

### YOUR TASK
Analyze the history. You may either:
1. Mutate the current BASELINE into a better version.
2. Adapt a known SKILL to this project's requirements.
3. Combine a SKILL with the BASELINE.

Propose 4 NEW and DISTINCT EditingStrategy variations.
- Variation 1: Optimization of the baseline.
- Variation 2: Aggression based on human hints.
- Variation 3: Deep adaptation of a SKILL (if any seem relevant).
- Variation 4: "World Class" - total synthesis of best practices.

Respond ONLY with a JSON array of 4 EditingStrategy objects. No prose.
Example:
[
  {{ "scene_threshold": 0.3, ... }},
  ...
]"#,
            history_json,
            baseline_json,
            available_skills,
            hints.increase,
            hints.decrease,
            hints.preserve,
            hints.notes
        );

        match self.agent.reason(&prompt).await {
            Ok(response) => {
                let extracted = if let Some(mat) = regex::Regex::new(r"(?s)\[.*\]")
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

                match serde_json::from_str::<Vec<EditingStrategy>>(clean_json) {
                    Ok(variants) => {
                        if variants.len() >= 4 {
                            Ok(variants.into_iter().take(4).collect())
                        } else {
                            Err("LLM returned fewer than 4 variants".into())
                        }
                    }
                    Err(e) => Err(format!("Failed to parse LLM variants: {}. Raw: {}", e, clean_json)),
                }
            }
            Err(e) => Err(format!("LLM reasoning failed: {}", e)),
        }
    }

    /// Archive a high-performing strategy as a persistent "Skill".
    async fn archive_skill(
        &self,
        strategy: &EditingStrategy,
        result: &ExperimentResult,
    ) -> Result<String, String> {
        let strategy_json = serde_json::to_string_pretty(strategy).unwrap_or_default();
        let prompt = format!(
            r#"You just discovered a HIGH-PERFORMING video editing strategy.
            
### QUALITY SCORE
{:.4}

### STRATEGY PARAMS
{}

### YOUR TASK
Generate a descriptive, short name (2-4 words) and a 1-sentence summary of why this works.
Respond ONLY in JSON.
Example:
{{ "name": "Dynamic High-Speech Cut", "summary": "Aggressively minimizes silence while boosting overlapping dialogue." }}"#,
            result.quality_score,
            strategy_json
        );

        match self.agent.reason(&prompt).await {
            Ok(response) => {
                let clean_json = response.trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();

                let metadata: serde_json::Value = serde_json::from_str(clean_json).unwrap_or_default();
                let skill_name = metadata["name"].as_str().unwrap_or("Unnamed Skill").replace(" ", "_").to_lowercase();
                let summary = metadata["summary"].as_str().unwrap_or("No summary provided.");

                let skill_content = serde_json::json!({
                    "name": skill_name,
                    "summary": summary,
                    "quality_score": result.quality_score,
                    "strategy": strategy,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                let skills_dir = ".agent/skills";
                std::fs::create_dir_all(skills_dir).ok();
                let path = format!("{}/{}.json", skills_dir, skill_name);

                match std::fs::write(&path, serde_json::to_string_pretty(&skill_content).unwrap()) {
                    Ok(_) => Ok(path),
                    Err(e) => Err(format!("IO Error saving skill: {}", e)),
                }
            }
            Err(e) => Err(format!("LLM skill naming failed: {}", e)),
        }
    }
}

// ─── Mutation guidance parsed from improve_program.md ───────────────────────

#[derive(Debug, Clone, Default)]
pub struct MutationHints {
    /// Params the program wants pushed higher (positive bias on mutations).
    pub increase: Vec<String>,
    /// Params the program wants pushed lower (negative bias on mutations).
    pub decrease: Vec<String>,
    /// Params to leave mostly alone (reduce mutation magnitude).
    pub preserve: Vec<String>,
    /// Free-text notes (logged but not acted on automatically).
    pub notes: Vec<String>,
}

impl MutationHints {
    fn bias_for(&self, param: &str) -> f64 {
        if self.increase.iter().any(|p| p == param) {
            0.08
        } else if self.decrease.iter().any(|p| p == param) {
            -0.08
        } else if self.preserve.iter().any(|p| p == param) {
            0.0
        } else {
            0.0 // neutral — pure noise
        }
    }

    fn magnitude_for(&self, param: &str, base: f64) -> f64 {
        if self.preserve.iter().any(|p| p == param) {
            base * 0.3 // very small perturbation for preserved params
        } else {
            base
        }
    }
}

fn parse_improve_program(path: &Path) -> MutationHints {
    let mut hints = MutationHints::default();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return hints,
    };

    for line in content.lines() {
        let line = line.trim().to_lowercase();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        // "increase <param>" / "raise <param>" / "boost <param>"
        for prefix in &["increase ", "raise ", "boost ", "+ "] {
            if let Some(rest) = line.strip_prefix(prefix) {
                let param = rest.split_whitespace().next().unwrap_or("").to_string();
                if !param.is_empty() && !hints.increase.contains(&param) {
                    hints.increase.push(param);
                }
            }
        }

        // "decrease <param>" / "reduce <param>" / "lower <param>"
        for prefix in &["decrease ", "reduce ", "lower ", "- "] {
            if let Some(rest) = line.strip_prefix(prefix) {
                let param = rest.split_whitespace().next().unwrap_or("").to_string();
                if !param.is_empty() && !hints.decrease.contains(&param) {
                    hints.decrease.push(param);
                }
            }
        }

        // "preserve <param>" / "keep <param>" / "lock <param>"
        for prefix in &["preserve ", "keep ", "lock ", "= "] {
            if let Some(rest) = line.strip_prefix(prefix) {
                let param = rest.split_whitespace().next().unwrap_or("").to_string();
                if !param.is_empty() && !hints.preserve.contains(&param) {
                    hints.preserve.push(param);
                }
            }
        }

        // Everything else is a note
        if !line.starts_with("increase ")
            && !line.starts_with("raise ")
            && !line.starts_with("boost ")
            && !line.starts_with("+ ")
            && !line.starts_with("decrease ")
            && !line.starts_with("reduce ")
            && !line.starts_with("lower ")
            && !line.starts_with("- ")
            && !line.starts_with("preserve ")
            && !line.starts_with("keep ")
            && !line.starts_with("lock ")
            && !line.starts_with("= ")
        {
            hints.notes.push(line.to_string());
        }
    }

    if !hints.increase.is_empty() || !hints.decrease.is_empty() {
        info!(
            "[IMPROVE] 📋 Program hints — increase: {:?} | decrease: {:?} | preserve: {:?}",
            hints.increase, hints.decrease, hints.preserve
        );
    }

    hints
}

/// Fallback mutation logic if LLM fails or is offline.
fn mutate_strategy_fallback(baseline: &EditingStrategy, _hints: &MutationHints) -> EditingStrategy {
    // Basic ±10% jitter as safety fallback
    let mut rng = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as f64;
    let mut jitter = |val: f64| -> f64 {
        let n = (rng % 1000.0) / 1000.0;
        rng += 1.0;
        val * (0.9 + n * 0.2)
    };

    EditingStrategy {
        scene_threshold: (jitter(baseline.scene_threshold)).clamp(0.05, 0.9),
        min_scene_score: (jitter(baseline.min_scene_score)).clamp(0.05, 0.85),
        boring_penalty_threshold: (jitter(baseline.boring_penalty_threshold)).clamp(5.0, 120.0),
        speech_boost: (jitter(baseline.speech_boost)).clamp(0.0, 2.0),
        silence_penalty: (jitter(baseline.silence_penalty)).clamp(-2.0, 0.0),
        continuity_boost: (jitter(baseline.continuity_boost)).clamp(0.0, 2.0),
        speech_ratio_threshold: (jitter(baseline.speech_ratio_threshold)).clamp(0.01, 0.8),
        action_duration_threshold: (jitter(baseline.action_duration_threshold)).clamp(0.5, 15.0),
        max_jump_gap_secs: (jitter(baseline.max_jump_gap_secs)).clamp(10.0, 180.0),
    }
}

// ─── Quality metric (the "val_bpb" of SYNOID) ───────────────────────────────

/// Compute a composite quality score in [0, 1] from a scored scene list.
///
/// Two components:
/// - **balance_score** (65%): peaks when ~45% of scenes are kept.
///   Penalises extremes (cutting everything or keeping everything).
/// - **discrimination_score** (35%): rewards a wide spread of scene scores
///   (0.25 std-dev → full credit).  A strategy that gives everything 0.5 is
///   useless even if the kept ratio happens to look good.
pub fn compute_quality(scenes: &[crate::agent::specialized::smart_editor::Scene], min_score: f64) -> f64 {
    if scenes.is_empty() {
        return 0.0;
    }

    let total = scenes.len() as f64;
    let kept = scenes.iter().filter(|s| s.score > min_score).count() as f64;
    let kept_ratio = kept / total;

    // Balance: Gaussian around 45% kept (σ ≈ 0.25 → reasonable width)
    let balance_score = (-8.0 * (kept_ratio - 0.45).powi(2)).exp();

    // Discrimination: std-dev of all scene scores, normalised so 0.25 → 1.0
    let mean = scenes.iter().map(|s| s.score).sum::<f64>() / total;
    let variance = scenes.iter().map(|s| (s.score - mean).powi(2)).sum::<f64>() / total;
    let std_dev = variance.sqrt();
    let discrimination_score = (std_dev * 4.0).min(1.0);

    0.65 * balance_score + 0.35 * discrimination_score
}

// ─── Single experiment ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    pub iteration: u64,
    pub candidate_id: usize,
    pub quality_score: f64,
    pub kept_ratio: f64,
    pub scene_count: usize,
    pub improved: bool,
    pub timestamp: u64,
    pub strategy: EditingStrategy,
}

async fn run_dry_eval(
    video: &Path,
    strategy: &EditingStrategy,
    iteration: u64,
    candidate_id: usize,
) -> Option<ExperimentResult> {
    // Detect scenes (ffprobe pass — fast, no render)
    let mut scenes = match detect_scenes(video, strategy.scene_threshold).await {
        Ok(s) => s,
        Err(e) => {
            warn!("[IMPROVE] Scene detection failed for candidate {}: {}", candidate_id, e);
            return None;
        }
    };

    if scenes.is_empty() {
        return None;
    }

    // Build a neutral intent for scoring (no transcript needed)
    let intent = EditIntent {
        remove_boring: true,
        keep_action: true,
        remove_silence: true,
        keep_speech: true,
        ruthless: false,
        density: EditDensity::Balanced,
        custom_keywords: vec![],
        target_duration: None,
        censor_profanity: false,
        profanity_replacement: None,
        show_cut_markers: false,
        use_remotion: false,
        remotion_template: None,
        enable_subtitles: false,
    };

    let total_duration = scenes.last().map(|s| s.end_time).unwrap_or(0.0);
    score_scenes(&mut scenes, &intent, None, strategy, total_duration);

    let total = scenes.len();
    let kept = scenes.iter().filter(|s| s.score > strategy.min_scene_score).count();
    let kept_ratio = if total > 0 { kept as f64 / total as f64 } else { 0.0 };

    let quality_score = compute_quality(&scenes, strategy.min_scene_score);

    Some(ExperimentResult {
        iteration,
        candidate_id,
        quality_score,
        kept_ratio,
        scene_count: total,
        improved: false, // filled in by the loop
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        strategy: strategy.clone(),
    })
}

// ─── Persistent log ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImproveLog {
    pub iterations_run: u64,
    pub experiments_run: u64,
    pub improvements: u64,
    pub best_quality: f64,
    pub baseline_quality: f64,
    pub recent: Vec<ExperimentResult>, // Rolling window, last 200
}

impl ImproveLog {
    fn log_path() -> PathBuf {
        let suffix = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_default();
        PathBuf::from(format!("cortex_cache{}", suffix)).join("improve_log.json")
    }

    pub fn load() -> Self {
        if let Ok(data) = fs::read_to_string(Self::log_path()) {
            if let Ok(log) = serde_json::from_str(&data) {
                return log;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::log_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, data);
        }
    }

    fn push(&mut self, result: ExperimentResult) {
        self.recent.push(result);
        if self.recent.len() > 200 {
            self.recent.drain(..self.recent.len() - 200);
        }
    }
}

// ─── AutoImprove public API ───────────────────────────────────────────────────

fn get_videos_in_dir(path: &Path) -> Vec<PathBuf> {
    let mut videos = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(ext) = p.extension() {
                    let ext_str = ext.to_string_lossy();
                    if ext_str.eq_ignore_ascii_case("mp4") || ext_str.eq_ignore_ascii_case("mkv") || ext_str.eq_ignore_ascii_case("mov") {
                        videos.push(p);
                    }
                }
            }
        }
    }
    videos
}

async fn fetch_and_replace_video(download_dir: &Path, current_videos: &mut Vec<PathBuf>) {
    info!("[IMPROVE] 📥 Fetching a new reference video for further improvement...");
    use crate::agent::tools::source_tools;
    
    let topics = [
        "cinematic travel video",
        "gaming montage fast paced",
        "documentary style editing",
        "vlog editing tricks",
        "music video visual effects"
    ];
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let topic = topics[(seed as usize) % topics.len()];

    if let Ok(results) = source_tools::search_youtube(topic, 5).await {
        for source in results {
            if source.duration > 30.0 && source.duration < 1200.0 {
                if let Some(url) = source.original_url {
                    let browser = source_tools::detect_browser();
                    if let Ok(downloaded) = source_tools::download_youtube(&url, download_dir, browser.as_deref()).await {
                        info!("[IMPROVE] ✅ Acquired new video: {}", downloaded.title);
                        if !current_videos.contains(&downloaded.local_path) {
                            current_videos.push(downloaded.local_path.clone());
                        }
                        break;
                    }
                }
            }
        }
    }
    
    // Evict oldest videos if > 10
    if current_videos.len() > 10 {
        current_videos.sort_by(|a, b| {
            std::fs::metadata(a).and_then(|m| m.modified()).ok()
              .cmp(&std::fs::metadata(b).and_then(|m| m.modified()).ok())
        });
        
        while current_videos.len() > 10 {
            let stale = current_videos.remove(0);
            info!("[IMPROVE] 🧹 Evicting old reference video: {:?}", stale.file_name().unwrap_or_default());
            let _ = std::fs::remove_file(stale);
        }
    }
}

async fn run_dry_eval_all(
    videos: &[PathBuf],
    strategy: &EditingStrategy,
    iteration: u64,
    candidate_id: usize,
) -> Option<ExperimentResult> {
    if videos.is_empty() {
        return None;
    }

    let mut total_quality = 0.0;
    let mut total_kept = 0.0;
    let mut total_scenes = 0;
    let mut successful_evals = 0;

    for video in videos {
        if let Some(res) = run_dry_eval(video, strategy, iteration, candidate_id).await {
            total_quality += res.quality_score;
            total_kept += res.kept_ratio;
            total_scenes += res.scene_count;
            successful_evals += 1;
        }
    }

    if successful_evals == 0 {
        return None;
    }

    Some(ExperimentResult {
        iteration,
        candidate_id,
        quality_score: total_quality / successful_evals as f64,
        kept_ratio: total_kept / successful_evals as f64,
        scene_count: total_scenes, // Sum of all scenes evaluated
        improved: false,
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        strategy: strategy.clone(),
    })
}

/// Configuration for the self-improvement loop.
pub struct AutoImprove {
    /// Directory to read reference videos from.
    pub download_dir: PathBuf,
    /// How many strategy mutations to evaluate per iteration.
    pub candidates_per_iter: usize,
    /// Stop after this many iterations (None = run until Ctrl-C).
    pub max_iterations: Option<u64>,
    /// Path to the human-curated guidance document.
    pub program_path: PathBuf,
}

impl AutoImprove {
    pub fn new() -> Self {
        let suffix = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_default();
        Self {
            download_dir: crate::agent::video_style_learner::get_download_dir(),
            candidates_per_iter: 4,
            max_iterations: None,
            program_path: PathBuf::from(format!("cortex_cache{}", suffix))
                .join("improve_program.md"),
        }
    }

    /// Run the improvement loop.  Calls `shutdown_rx.has_changed()` each
    /// iteration so the caller can stop it with a tokio `watch` channel.
    pub async fn run(
        &self,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "[IMPROVE] 🚀 AutoImprove loop starting | dir: {:?} | {} candidates/iter",
            self.download_dir, self.candidates_per_iter
        );

        let mut log = ImproveLog::load();
        let mut np = Neuroplasticity::new();

        // Establish baseline ────────────────────────────────────────────────
        let baseline = EditingStrategy::load();
        info!("[IMPROVE] 📐 Loaded baseline EditingStrategy");

        let mut videos = get_videos_in_dir(&self.download_dir);
        if videos.is_empty() {
            // Fetch one immediately if empty
            fetch_and_replace_video(&self.download_dir, &mut videos).await;
            if videos.is_empty() {
                return Err("No videos found in Download directory and failed to fetch new ones".into());
            }
        }

        let baseline_quality = match run_dry_eval_all(
            &videos,
            &baseline,
            0,
            0,
        )
        .await
        {
            Some(r) => {
                info!(
                    "[IMPROVE] 📊 Baseline quality: {:.4} | kept {:.1}% of {} scenes across {} videos",
                    r.quality_score,
                    r.kept_ratio * 100.0,
                    r.scene_count,
                    videos.len()
                );
                r.quality_score
            }
            None => {
                return Err(
                    "Baseline evaluation failed — check that videos are readable by ffprobe"
                        .into(),
                )
            }
        };

        if log.baseline_quality == 0.0 {
            log.baseline_quality = baseline_quality;
        }
        if log.best_quality == 0.0 {
            log.best_quality = baseline_quality;
        }

        let mut current_strategy = baseline;
        let mut current_quality = baseline_quality;
        let mut iteration = log.iterations_run;

        // Main loop ─────────────────────────────────────────────────────────
        loop {
            // Shutdown check
            if *shutdown.borrow() {
                info!("[IMPROVE] 🛑 Shutdown signal received — stopping loop");
                break;
            }

            // Max iterations check
            if let Some(max) = self.max_iterations {
                if iteration >= max {
                    info!("[IMPROVE] ✅ Reached {} iterations — stopping", max);
                    break;
                }
            }

            iteration += 1;
            info!(
                "[IMPROVE] ─── Iteration {} | current quality: {:.4} | best ever: {:.4} ───",
                iteration, current_quality, log.best_quality
            );

            // Re-parse program.md each iteration so humans can steer live
            let hints = parse_improve_program(&self.program_path);
            if !hints.notes.is_empty() {
                info!("[IMPROVE] 📋 Program notes: {:?}", hints.notes);
            }

            // Generate & evaluate candidates via LLM ─────────────────────
            let advisor = LlmStrategyAdvisor::new();
            let mut best_candidate: Option<ExperimentResult> = None;

            info!("[IMPROVE] 🧠 Querying LLM for {} strategy variants...", self.candidates_per_iter);
            
            // Get history slice for the prompt
            let history_context = &log.recent;
            
            let candidates = match advisor.generate_candidates(&current_strategy, history_context, &hints).await {
                Ok(cands) => cands,
                Err(e) => {
                    warn!("[IMPROVE] LLM Advisor failed: {}. Falling back to random mutation.", e);
                    let mut fallback_cands = Vec::new();
                    for _ in 0..self.candidates_per_iter {
                        fallback_cands.push(mutate_strategy_fallback(&current_strategy, &hints));
                    }
                    fallback_cands
                }
            };

            for (cid, candidate) in candidates.into_iter().enumerate() {
                info!(
                    "[IMPROVE]   Candidate {}/{}: scene_thr={:.3} min_score={:.3} speech_boost={:.3}",
                    cid + 1,
                    self.candidates_per_iter,
                    candidate.scene_threshold,
                    candidate.min_scene_score,
                    candidate.speech_boost,
                );

                if let Some(result) =
                    run_dry_eval_all(&videos, &candidate, iteration, cid).await
                {
                    info!(
                        "[IMPROVE]   → quality {:.4} | kept {:.1}% | {} scenes",
                        result.quality_score,
                        result.kept_ratio * 100.0,
                        result.scene_count
                    );

                    log.experiments_run += 1;
                    let is_better = best_candidate
                        .as_ref()
                        .map(|b| result.quality_score > b.quality_score)
                        .unwrap_or(true);

                    if is_better {
                        best_candidate = Some(result);
                    }
                }

                // Shutdown check inside candidate loop
                if *shutdown.borrow() {
                    break;
                }
            }

            // Promote best candidate if it beats current ─────────────────
            if let Some(mut best) = best_candidate {
                // 🔥 ARCHIVE TO SKILLS IF EXTREMELY GOOD (Independent of current_quality)
                if best.quality_score > 0.65 {
                    info!("[IMPROVE] ✨ Quality {:.4} exceeds threshold (0.65). Crystallizing as persistent Skill...", best.quality_score);
                    let advisor = LlmStrategyAdvisor::new(); // Re-init or use existing
                    match advisor.archive_skill(&best.strategy, &best).await {
                        Ok(p) => info!("[IMPROVE] 💾 Skill crystallized at: {}", p),
                        Err(e) => warn!("[IMPROVE] ⚠️ Failed to crystallize skill: {}", e),
                    }
                }

                if best.quality_score > current_quality {
                    let gain = best.quality_score - current_quality;
                    info!(
                        "[IMPROVE] ✨ IMPROVEMENT: {:.4} → {:.4} (+{:.4}) | kept {:.1}%",
                        current_quality,
                        best.quality_score,
                        gain,
                        best.kept_ratio * 100.0,
                    );

                    current_quality = best.quality_score;
                    current_strategy = best.strategy.clone();
                    best.improved = true;

                    // Persist the new strategy
                    current_strategy.save_to_cortex();

                    // Award neuroplasticity XP (quality-weighted)
                    np.record_success_with_quality(best.quality_score);
                    info!("[IMPROVE] ⚡ {}", np.acceleration_report());

                    log.improvements += 1;
                    if best.quality_score > log.best_quality {
                        log.best_quality = best.quality_score;
                        info!(
                            "[IMPROVE] 🏆 New all-time best quality: {:.4}",
                            log.best_quality
                        );
                    }

                    log.push(best);

                    // Fetch new videos and replace old ones up to a limit of 10
                    fetch_and_replace_video(&self.download_dir, &mut videos).await;
                } else {
                    info!(
                        "[IMPROVE] ↔ No improvement this iteration (best candidate: {:.4} ≤ current: {:.4})",
                        best.quality_score, current_quality
                    );
                    log.push(best);
                }
            } else {
                warn!("[IMPROVE] All candidates failed evaluation this iteration");
            }

            log.iterations_run = iteration;
            log.save();

            // Adaptive sleep — faster brain = shorter wait ────────────────
            let delay_secs = np.adaptive_delay_secs(30);
            info!(
                "[IMPROVE] 💤 Sleeping {}s before next iteration ({} improvements total)",
                delay_secs, log.improvements
            );
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
        }

        info!(
            "[IMPROVE] 🏁 Loop complete | {} iterations | {} improvements | best quality {:.4}",
            log.iterations_run, log.improvements, log.best_quality
        );
        log.save();
        Ok(())
    }

    /// Print a summary of past improvement runs.
    pub fn print_status() {
        let log = ImproveLog::load();
        println!("=== AutoImprove Status ===");
        println!("  Iterations run   : {}", log.iterations_run);
        println!("  Experiments run  : {}", log.experiments_run);
        println!("  Improvements     : {}", log.improvements);
        println!("  Baseline quality : {:.4}", log.baseline_quality);
        println!("  Best quality     : {:.4}", log.best_quality);
        println!(
            "  Quality gain     : {:.4}",
            log.best_quality - log.baseline_quality
        );
        if let Some(last) = log.recent.last() {
            println!("  Last experiment  : quality={:.4} kept={:.1}%",
                last.quality_score, last.kept_ratio * 100.0);
        }
    }
}
