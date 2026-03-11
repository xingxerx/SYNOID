// SYNOID Video Style Learner
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Scans D:\SYNOID\Download for up to 10 reference videos and learns editing
// style patterns from each one. Results are injected into the LearningKernel
// and Neuroplasticity systems, and a tuned EditingStrategy is written to
// cortex_cache so every future edit reflects what was observed.
//
// Style pipeline per video
//   1. Classify genre from filename keywords
//   2. Detect scenes → measure avg shot length & density
//   3. Map metrics to an EditingPattern and memorize it
//   4. Award quality-weighted XP to Neuroplasticity
//   5. After all videos are processed, synthesize and save EditingStrategy

use crate::agent::brain::Brain;
use crate::agent::learning::EditingPattern;
use crate::agent::smart_editor::EditingStrategy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Video genre classification
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum VideoGenre {
    CinematicTravel,
    GamingMontage,
    Documentary,
    Vlog,
    General,
}

impl VideoGenre {
    fn from_filename(name: &str) -> Self {
        let lower = name.to_lowercase();

        // Cinematic travel
        if lower.contains("cinematic") || lower.contains("travel") || lower.contains("bali") {
            return Self::CinematicTravel;
        }
        // Gaming / montage
        if lower.contains("gaming")
            || lower.contains("montage")
            || lower.contains("call of duty")
            || lower.contains("cod")
            || lower.contains("game")
        {
            return Self::GamingMontage;
        }
        // Documentary
        if lower.contains("documentary")
            || lower.contains("doco")
            || lower.contains("netflix")
            || lower.contains("filmmaker")
            || lower.contains("johnny harris")
        {
            return Self::Documentary;
        }
        // Vlog
        if lower.contains("vlog") {
            return Self::Vlog;
        }

        Self::General
    }

    fn intent_tag(&self) -> &'static str {
        match self {
            Self::CinematicTravel => "cinematic_travel_video",
            Self::GamingMontage => "gaming_montage",
            Self::Documentary => "documentary",
            Self::Vlog => "vlog",
            Self::General => "general",
        }
    }

    fn color_grade_style(&self) -> &'static str {
        match self {
            Self::CinematicTravel => "teal_orange_cinematic",
            Self::GamingMontage => "high_contrast_vivid",
            Self::Documentary => "natural_desaturated",
            Self::Vlog => "warm_bright",
            Self::General => "neutral",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-video analysis result
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStyleProfile {
    pub path: String,
    pub genre_tag: String,
    pub avg_scene_duration: f64,
    pub scene_count: usize,
    pub transition_speed: f64,
    pub music_sync_strictness: f64,
    pub color_grade_style: String,
    pub outcome_xp: f64,
}

impl VideoStyleProfile {
    /// Convert to an EditingPattern for the LearningKernel.
    pub fn to_pattern(&self) -> EditingPattern {
        EditingPattern {
            intent_tag: self.genre_tag.clone(),
            avg_scene_duration: self.avg_scene_duration,
            transition_speed: self.transition_speed,
            music_sync_strictness: self.music_sync_strictness,
            color_grade_style: self.color_grade_style.clone(),
            success_rating: 5,
            source_video: Some(self.path.clone()),
            kept_ratio: 0.5,
            outcome_xp: self.outcome_xp,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Learned-video cache — prevents re-analysis on every restart
// ─────────────────────────────────────────────────────────────────────────────

/// One entry per video file that has already been analysed.
/// Keyed by filename (not full path so it survives drive-letter changes).
#[derive(Serialize, Deserialize, Clone, Debug)]
struct LearnedEntry {
    /// File size in bytes — used as a cheap "has this changed?" sentinel.
    file_size: u64,
    /// The profile produced by the last analysis.
    profile: VideoStyleProfile,
}

/// Persisted map of filename → LearnedEntry.
#[derive(Serialize, Deserialize, Default, Debug)]
struct LearnedVideoCache(HashMap<String, LearnedEntry>);

impl LearnedVideoCache {
    fn path() -> PathBuf {
        let suffix = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_default();
        let dir = PathBuf::from(format!("cortex_cache{}", suffix));
        let _ = std::fs::create_dir_all(&dir);
        dir.join("learned_videos.json")
    }

    fn load() -> Self {
        let p = Self::path();
        if let Ok(data) = std::fs::read_to_string(&p) {
            if let Ok(cache) = serde_json::from_str::<LearnedVideoCache>(&data) {
                return cache;
            }
        }
        Self::default()
    }

    fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), json);
        }
    }

    /// Return the cached profile if the file hasn't changed since last analysis.
    fn get_if_current(&self, filename: &str, current_size: u64) -> Option<&VideoStyleProfile> {
        self.0.get(filename).and_then(|e| {
            if e.file_size == current_size {
                Some(&e.profile)
            } else {
                None // File replaced/updated — must re-analyse
            }
        })
    }

    fn insert(&mut self, filename: String, file_size: u64, profile: VideoStyleProfile) {
        self.0.insert(filename, LearnedEntry { file_size, profile });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Core public API
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the directory that holds reference videos for this instance.
pub fn get_download_dir() -> PathBuf {
    let suffix = std::env::var("SYNOID_INSTANCE_ID").unwrap_or_default();
    PathBuf::from(format!("D:\\SYNOID\\Download{}", suffix))
}

/// Directory that holds reference videos downloaded by the autonomous learner.
#[deprecated(note = "Use get_download_dir() instead")]
pub const DOWNLOAD_DIR: &str = r"D:\SYNOID\Download";
/// Maximum reference videos to keep / learn from at any one time.
pub const MAX_VIDEOS: usize = 10;

/// Return value from `learn_from_downloads`.
pub struct LearnResult {
    /// All profiles (cached + newly learned) — used to synthesise strategy.
    pub profiles: Vec<VideoStyleProfile>,
    /// True if any video was newly analysed (strategy should be re-saved).
    pub has_new: bool,
}

/// Scan DOWNLOAD_DIR, analyse up to MAX_VIDEOS MP4s, and inject all learned
/// patterns into the brain's LearningKernel and Neuroplasticity.
///
/// Already-learned videos are skipped (loaded from cache) so the agent only
/// does real work — and only awards XP — for genuinely new or changed files.
/// Returns all profiles (cached + new) so the caller can synthesise a strategy.
pub async fn learn_from_downloads(brain: &mut Brain) -> LearnResult {
    let download_dir_buf = get_download_dir();
    let download_dir = download_dir_buf.as_path();
    if !download_dir.exists() {
        warn!("[STYLE_LEARNER] Download dir not found: {:?}", download_dir);
        return LearnResult { profiles: Vec::new(), has_new: false };
    }

    // Collect MP4 files up to MAX_VIDEOS
    let mut videos: Vec<PathBuf> = std::fs::read_dir(download_dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.eq_ignore_ascii_case("mp4"))
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();

    // Deterministic order (alphabetical) so results are reproducible
    videos.sort();
    videos.truncate(MAX_VIDEOS);

    if videos.is_empty() {
        info!("[STYLE_LEARNER] No MP4 files found in {:?}", get_download_dir());
        return LearnResult { profiles: Vec::new(), has_new: false };
    }

    let mut cache = LearnedVideoCache::load();
    let mut all_profiles: Vec<VideoStyleProfile> = Vec::new();
    let mut new_count = 0usize;
    let mut cache_dirty = false;

    info!(
        "[STYLE_LEARNER] 🎓 Checking {} video(s) ({} already memorized)",
        videos.len(),
        cache.0.len()
    );

    for (idx, path) in videos.iter().enumerate() {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        // Check if this video is already known and unchanged
        if let Some(cached_profile) = cache.get_if_current(filename, file_size) {
            info!(
                "[STYLE_LEARNER] [{}/{}] ⚡ Already learned (cached): {}",
                idx + 1,
                videos.len(),
                filename
            );
            // Still register the pattern in the kernel (it's loaded fresh each run)
            // but do NOT award XP — we already earned it
            let genre = VideoGenre::from_filename(filename);
            brain.learning_kernel.memorize(genre.intent_tag(), cached_profile.to_pattern());
            all_profiles.push(cached_profile.clone());
            continue;
        }

        // New or changed file — full analysis required
        info!(
            "[STYLE_LEARNER] [{}/{}] 🔬 Analysing (new/changed): {}",
            idx + 1,
            videos.len(),
            filename
        );

        let genre = VideoGenre::from_filename(filename);
        let profile = analyse_video(path, &genre).await;

        let xp = profile.outcome_xp;
        let tag = genre.intent_tag();
        brain.learning_kernel.memorize(tag, profile.to_pattern());
        brain.neuroplasticity.record_success_with_quality(xp);

        info!(
            "[STYLE_LEARNER] ✅ Learned '{}': avg_scene={:.2}s, xp={:.2}, speed={:.1}×",
            tag,
            profile.avg_scene_duration,
            xp,
            brain.neuroplasticity.current_speed(),
        );

        cache.insert(filename.to_string(), file_size, profile.clone());
        all_profiles.push(profile);
        new_count += 1;
        cache_dirty = true;
    }

    if cache_dirty {
        cache.save();
    }

    if new_count == 0 {
        info!(
            "[STYLE_LEARNER] ✅ All {} video(s) already memorized — nothing to relearn | {}",
            all_profiles.len(),
            brain.neuroplasticity.acceleration_report()
        );
    } else {
        info!(
            "[STYLE_LEARNER] 🏁 Session complete — {} new, {} total patterns | {}",
            new_count,
            all_profiles.len(),
            brain.neuroplasticity.acceleration_report()
        );
    }

    LearnResult { profiles: all_profiles, has_new: new_count > 0 }
}

/// Remove a video from the learned cache after it has been deleted from disk.
/// Call this whenever you evict a video so stale entries don't persist.
pub fn remove_from_cache(filename: &str) {
    let mut cache = LearnedVideoCache::load();
    if cache.0.remove(filename).is_some() {
        cache.save();
        info!("[STYLE_LEARNER] 🗑️ Removed '{}' from learned cache", filename);
    }
}

/// Synthesise a tuned EditingStrategy from all learned profiles and persist
/// it to cortex_cache so future edits pick it up automatically.
pub fn synthesise_and_save_strategy(profiles: &[VideoStyleProfile]) {
    if profiles.is_empty() {
        return;
    }

    let strategy = synthesise_strategy(profiles);
    strategy.save_to_cortex();

    info!(
        "[STYLE_LEARNER] 💾 EditingStrategy saved — scene_threshold={:.2}, min_scene_score={:.2}, speech_boost={:.2}",
        strategy.scene_threshold,
        strategy.min_scene_score,
        strategy.speech_boost,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Analyse a single video and return a style profile.
/// Falls back to filename-derived heuristics if ffprobe/scene detection fails.
async fn analyse_video(path: &Path, genre: &VideoGenre) -> VideoStyleProfile {
    let mut avg_scene_duration: f64 = genre_default_scene_duration(genre);
    let mut scene_count: usize = 0;

    // Attempt real scene detection via smart_editor
    match crate::agent::smart_editor::detect_scenes(path, 0.3).await {
        Ok(scenes) if !scenes.is_empty() => {
            let total: f64 = scenes.iter().map(|s| s.duration).sum();
            avg_scene_duration = total / scenes.len() as f64;
            scene_count = scenes.len();
            info!(
                "[STYLE_LEARNER] Scene detection: {} scenes, avg {:.2}s",
                scene_count, avg_scene_duration
            );
        }
        Ok(_) => {
            warn!("[STYLE_LEARNER] Scene detection returned 0 scenes — using heuristic");
        }
        Err(e) => {
            warn!(
                "[STYLE_LEARNER] Scene detection failed ({}), using heuristic",
                e
            );
        }
    }

    // Map avg scene duration → transition speed
    // Fast cuts (< 2s) need speed-up; slow (> 5s) need slow-down
    let transition_speed = if avg_scene_duration < 1.5 {
        2.0
    } else if avg_scene_duration < 3.0 {
        1.5
    } else if avg_scene_duration < 6.0 {
        1.0
    } else {
        0.8
    };

    // Music sync is tighter for montage/gaming, looser for documentary
    let music_sync_strictness = match genre {
        VideoGenre::GamingMontage => 0.9,
        VideoGenre::CinematicTravel => 0.7,
        VideoGenre::Vlog => 0.5,
        VideoGenre::Documentary => 0.3,
        VideoGenre::General => 0.5,
    };

    // Quality score: profiles with realistic scene durations (not extremes) score higher
    let outcome_xp = quality_score(avg_scene_duration);

    VideoStyleProfile {
        path: path.to_string_lossy().to_string(),
        genre_tag: genre.intent_tag().to_string(),
        avg_scene_duration,
        scene_count,
        transition_speed,
        music_sync_strictness,
        color_grade_style: genre.color_grade_style().to_string(),
        outcome_xp,
    }
}

/// Genre-based default shot length when scene detection is unavailable.
fn genre_default_scene_duration(genre: &VideoGenre) -> f64 {
    match genre {
        VideoGenre::GamingMontage => 1.2,
        VideoGenre::Vlog => 2.5,
        VideoGenre::CinematicTravel => 5.0,
        VideoGenre::Documentary => 4.0,
        VideoGenre::General => 3.5,
    }
}

/// Quality score based on shot length — penalises extremes (too fast or too slow).
/// Range: 0.3 (poor) … 1.0 (ideal).
fn quality_score(avg_secs: f64) -> f64 {
    // Ideal range for "clean and smooth" editing: 1.5 – 6.0 s
    if avg_secs >= 1.5 && avg_secs <= 6.0 {
        1.0
    } else if avg_secs < 0.5 || avg_secs > 12.0 {
        0.3 // Extreme — very aggressive or very slow
    } else {
        0.65 // Acceptable but not ideal
    }
}

/// Build an EditingStrategy tuned to the learned profiles.
///
/// Strategy rules:
/// - `scene_threshold`         avg across profiles (controls ffprobe sensitivity)
/// - `min_scene_score`         lower when shots are fast (keep more micro-cuts)
/// - `boring_penalty_threshold` linked to avg shot length
/// - `speech_boost`            higher for documentary/vlog where narration drives the edit
/// - `silence_penalty`         harsher for gaming/montage, gentler for cinematic
/// - `continuity_boost`        higher for documentary to preserve narrative
/// - `max_jump_gap_secs`       1.5× the overall avg shot length (capped 30–60 s)
fn synthesise_strategy(profiles: &[VideoStyleProfile]) -> EditingStrategy {
    let n = profiles.len() as f64;

    let avg_shot: f64 = profiles.iter().map(|p| p.avg_scene_duration).sum::<f64>() / n;

    // Count genre types for weighted tuning
    let gaming_count = profiles
        .iter()
        .filter(|p| p.genre_tag == "gaming_montage")
        .count() as f64;
    let docu_count = profiles
        .iter()
        .filter(|p| p.genre_tag == "documentary")
        .count() as f64;
    let cinematic_count = profiles
        .iter()
        .filter(|p| p.genre_tag == "cinematic_travel_video")
        .count() as f64;
    let vlog_count = profiles.iter().filter(|p| p.genre_tag == "vlog").count() as f64;

    // scene_threshold: tighter for fast content, looser for slow cinematic
    // Range 0.20 (very sensitive, catches micro-cuts) – 0.35 (coarser)
    let scene_threshold = if avg_shot < 2.0 {
        0.20
    } else if avg_shot < 4.0 {
        0.25
    } else {
        0.30
    };

    // min_scene_score: lower = keep more scenes
    // Documentary/cinematic: keep more → 0.20–0.25
    // Gaming: keep action beats but cut dead air → 0.22
    let min_scene_score = if gaming_count > docu_count + cinematic_count {
        0.22 // Gaming-dominant: allow micro-cuts
    } else if docu_count + cinematic_count > vlog_count + gaming_count {
        0.20 // Story-dominant: keep nearly everything
    } else {
        0.25 // Mixed / general
    };

    // boring_penalty_threshold: seconds before a long static shot is penalised
    // For fast content: 15s; for cinematic: 40s
    let boring_penalty_threshold = (avg_shot * 6.0).clamp(15.0, 40.0);

    // speech_boost: narration is crucial for documentary/cinematic
    let speech_weight = (docu_count + cinematic_count + vlog_count) / n;
    let speech_boost = 0.45 + 0.25 * speech_weight; // 0.45 – 0.70

    // silence_penalty: harsher for gaming (cut dead air), gentle for cinematic (atmosphere)
    let gaming_weight = gaming_count / n;
    let silence_penalty = -(0.3 + 0.2 * gaming_weight); // -0.30 … -0.50

    // continuity_boost: how much we reward consecutive kept scenes (narrative flow)
    let docu_weight = (docu_count + cinematic_count) / n;
    let continuity_boost = 0.55 + 0.20 * docu_weight; // 0.55 – 0.75

    // speech_ratio_threshold: fraction of scene with speech before we call it "speech-heavy"
    let speech_ratio_threshold = 0.08 + 0.04 * speech_weight; // 0.08 – 0.12

    // action_duration_threshold: min seconds for an action beat to be kept
    let action_duration_threshold = if avg_shot < 2.0 { 0.8 } else { 2.0 };

    // max_jump_gap: prevent jarring narrative jumps
    let max_jump_gap_secs = (avg_shot * 10.0).clamp(30.0, 60.0);

    info!(
        "[STYLE_LEARNER] Synthesised strategy: threshold={:.2}, min_score={:.2}, \
        boring_thresh={:.1}s, speech_boost={:.2}, silence_pen={:.2}, \
        continuity={:.2}, max_gap={:.1}s",
        scene_threshold,
        min_scene_score,
        boring_penalty_threshold,
        speech_boost,
        silence_penalty,
        continuity_boost,
        max_jump_gap_secs,
    );

    EditingStrategy {
        scene_threshold,
        min_scene_score,
        boring_penalty_threshold,
        speech_boost,
        silence_penalty,
        continuity_boost,
        speech_ratio_threshold,
        action_duration_threshold,
        max_jump_gap_secs,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genre_classification() {
        assert_eq!(
            VideoGenre::from_filename("BALI _ A CINEMATIC TRAVEL VIDEO.mp4"),
            VideoGenre::CinematicTravel
        );
        assert_eq!(
            VideoGenre::from_filename("TIMELESS _ Call of Duty Montage.mp4"),
            VideoGenre::GamingMontage
        );
        assert_eq!(
            VideoGenre::from_filename("How To Edit A Documentary Like Johnny Harris.mp4"),
            VideoGenre::Documentary
        );
        assert_eq!(
            VideoGenre::from_filename("How to edit Vlogs LIKE A PRO_.mp4"),
            VideoGenre::Vlog
        );
        assert_eq!(
            VideoGenre::from_filename("4 Editing Secrets Small Channels Learn Too Late.mp4"),
            VideoGenre::General
        );
    }

    #[test]
    fn quality_score_ranges() {
        assert!((quality_score(3.0) - 1.0).abs() < f64::EPSILON);
        assert!(quality_score(0.3) < 0.5);
        assert!(quality_score(15.0) < 0.5);
        assert!(quality_score(2.0) >= 0.9);
    }

    #[test]
    fn strategy_synthesis_gaming_dominant() {
        let profiles = vec![
            VideoStyleProfile {
                path: "a.mp4".into(),
                genre_tag: "gaming_montage".into(),
                avg_scene_duration: 1.0,
                scene_count: 50,
                transition_speed: 2.0,
                music_sync_strictness: 0.9,
                color_grade_style: "vivid".into(),
                outcome_xp: 1.0,
            },
            VideoStyleProfile {
                path: "b.mp4".into(),
                genre_tag: "gaming_montage".into(),
                avg_scene_duration: 1.5,
                scene_count: 40,
                transition_speed: 1.5,
                music_sync_strictness: 0.9,
                color_grade_style: "vivid".into(),
                outcome_xp: 1.0,
            },
        ];
        let strategy = synthesise_strategy(&profiles);
        // Gaming dominant — min_scene_score should be 0.22
        assert!((strategy.min_scene_score - 0.22).abs() < f64::EPSILON);
        // Fast content → lower scene_threshold
        assert!(strategy.scene_threshold <= 0.25);
    }
}
