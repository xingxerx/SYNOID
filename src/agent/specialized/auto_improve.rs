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

// ─── Simple LCG RNG (no external dep needed) ────────────────────────────────

struct Lcg(u64);

impl Lcg {
    fn seeded() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        // Mix seed bits to avoid clustering on low-resolution clocks
        let seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        Self(seed)
    }

    fn next_f64(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Return a value in `[low, high)`.
    fn range(&mut self, low: f64, high: f64) -> f64 {
        low + self.next_f64() * (high - low)
    }

    /// Return a signed delta in `[-magnitude, +magnitude]`.
    fn delta(&mut self, magnitude: f64) -> f64 {
        self.range(-magnitude, magnitude)
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

// ─── Mutation ─────────────────────────────────────────────────────────────

/// Apply a stochastic perturbation to every parameter, guided by program hints.
fn mutate_strategy(baseline: &EditingStrategy, hints: &MutationHints, rng: &mut Lcg) -> EditingStrategy {
    let base_mag = 0.12; // ±12% relative perturbation as base magnitude

    let perturb = |rng: &mut Lcg, value: f64, param: &str, min: f64, max: f64| -> f64 {
        let mag = hints.magnitude_for(param, base_mag);
        let bias = hints.bias_for(param);
        let noise = rng.delta(mag);
        (value * (1.0 + bias + noise)).clamp(min, max)
    };

    EditingStrategy {
        scene_threshold: perturb(rng, baseline.scene_threshold, "scene_threshold", 0.05, 0.90),
        min_scene_score: perturb(rng, baseline.min_scene_score, "min_scene_score", 0.05, 0.85),
        boring_penalty_threshold: perturb(
            rng,
            baseline.boring_penalty_threshold,
            "boring_penalty_threshold",
            5.0,
            120.0,
        ),
        speech_boost: perturb(rng, baseline.speech_boost, "speech_boost", 0.0, 1.5),
        silence_penalty: perturb(rng, baseline.silence_penalty, "silence_penalty", -1.5, 0.0),
        continuity_boost: perturb(rng, baseline.continuity_boost, "continuity_boost", 0.0, 1.5),
        speech_ratio_threshold: perturb(
            rng,
            baseline.speech_ratio_threshold,
            "speech_ratio_threshold",
            0.01,
            0.8,
        ),
        action_duration_threshold: perturb(
            rng,
            baseline.action_duration_threshold,
            "action_duration_threshold",
            0.5,
            15.0,
        ),
        max_jump_gap_secs: perturb(
            rng,
            baseline.max_jump_gap_secs,
            "max_jump_gap_secs",
            10.0,
            180.0,
        ),
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

            // Generate & evaluate candidates ─────────────────────────────
            let mut rng = Lcg::seeded();
            let mut best_candidate: Option<ExperimentResult> = None;

            for cid in 0..self.candidates_per_iter {
                let candidate = mutate_strategy(&current_strategy, &hints, &mut rng);
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
