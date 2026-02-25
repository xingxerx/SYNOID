// SYNOID Multicam Engine
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// AI Multicam Sync & SmartSwitch (Feature 2)
// ------------------------------------------
// Synchronises multiple camera angles using audio waveform cross-correlation
// and automatically switches to the camera that shows the active speaker,
// mirroring DaVinci Resolve's Multicam SmartSwitch workflow.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::info;

// ─────────────────────────────────────────────────────────────────────────────
// Data Structures
// ─────────────────────────────────────────────────────────────────────────────

/// A single camera track supplied to the multicam engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MulticamTrack {
    /// Path to the video file for this camera angle.
    pub path: PathBuf,
    /// Human-readable label (e.g. "Camera A", "Wide Shot").
    pub label: String,
    /// Audio channel index to use for sync analysis (0 = first).
    pub audio_channel: usize,
}

/// A timed cut-point produced by SmartSwitch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchPoint {
    /// Time in the *master* (synced) timeline at which to cut (seconds).
    pub master_time: f64,
    /// Index into `MulticamTrack` list for the camera to switch *to*.
    pub target_track: usize,
    /// Confidence score (0.0–1.0) from the activity detector.
    pub confidence: f64,
}

/// Per-track audio energy sample used during synchronisation.
#[derive(Debug, Clone)]
struct EnergyFrame {
    time: f64,
    energy: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// MulticamEngine
// ─────────────────────────────────────────────────────────────────────────────

pub struct MulticamEngine;

impl MulticamEngine {
    // ── Public API ───────────────────────────────────────────────────────────

    /// Align multiple camera tracks to a common timeline by cross-correlating
    /// their audio waveforms.  Returns per-track time offsets (seconds) that
    /// must be applied before assembly.
    ///
    /// The first track is treated as the master (offset = 0.0).
    pub async fn sync_tracks(tracks: &[MulticamTrack]) -> Result<Vec<f64>> {
        if tracks.is_empty() {
            return Ok(Vec::new());
        }

        info!("[MULTICAM] Syncing {} tracks via audio cross-correlation…", tracks.len());

        // Extract per-track energy profiles
        let mut profiles: Vec<Vec<EnergyFrame>> = Vec::new();
        for track in tracks {
            let frames = Self::extract_energy_profile(&track.path).await?;
            profiles.push(frames);
        }

        // Master is tracks[0]; compute offset for all others
        let master = &profiles[0];
        let mut offsets = vec![0.0f64];

        for slave in profiles.iter().skip(1) {
            let offset = Self::cross_correlate_offset(master, slave);
            info!("[MULTICAM] Detected offset: {:.3}s", offset);
            offsets.push(offset);
        }

        Ok(offsets)
    }

    /// Analyse a set of *already-synced* tracks and produce a list of
    /// `SwitchPoint`s that form an auto-edited multicam sequence.
    ///
    /// The engine picks the camera with the highest audio energy (active
    /// speaker) in each analysis window.
    pub async fn smart_switch(
        tracks: &[MulticamTrack],
        offsets: &[f64],
        window_secs: f64,
    ) -> Result<Vec<SwitchPoint>> {
        if tracks.is_empty() {
            return Ok(Vec::new());
        }

        info!("[MULTICAM-SWITCH] Analysing {} tracks with {:.1}s windows…", tracks.len(), window_secs);

        let mut profiles: Vec<Vec<EnergyFrame>> = Vec::new();
        for (i, track) in tracks.iter().enumerate() {
            let mut frames = Self::extract_energy_profile(&track.path).await?;
            // Apply sync offset
            let off = offsets.get(i).copied().unwrap_or(0.0);
            for f in &mut frames {
                f.time += off;
            }
            profiles.push(frames);
        }

        // Determine total duration from the longest track
        let total_duration = profiles
            .iter()
            .flat_map(|p| p.last().map(|f| f.time))
            .fold(0.0f64, f64::max);

        let mut switch_points: Vec<SwitchPoint> = Vec::new();
        let mut current_track: usize = 0;
        let mut t = 0.0f64;

        while t < total_duration {
            let window_end = (t + window_secs).min(total_duration);

            // Find the camera with the highest average energy in this window
            let mut best_track = current_track;
            let mut best_energy = -1.0f64;

            for (idx, profile) in profiles.iter().enumerate() {
                let avg = Self::average_energy_in_window(profile, t, window_end);
                if avg > best_energy {
                    best_energy = avg;
                    best_track = idx;
                }
            }

            // Only emit a cut if the active camera changed
            if best_track != current_track {
                switch_points.push(SwitchPoint {
                    master_time: t,
                    target_track: best_track,
                    confidence: (best_energy / (best_energy + 1.0)).min(1.0),
                });
                current_track = best_track;
            }

            t = window_end;
        }

        info!("[MULTICAM-SWITCH] Generated {} switch points.", switch_points.len());
        Ok(switch_points)
    }

    /// Assemble a final multicam cut using the supplied switch-points.
    ///
    /// Writes an FFmpeg concat-demuxer script and runs it, producing a single
    /// output file that alternates between camera angles at the cut-points.
    pub async fn assemble(
        tracks: &[MulticamTrack],
        offsets: &[f64],
        switch_points: &[SwitchPoint],
        output: &Path,
    ) -> Result<()> {
        if tracks.is_empty() {
            anyhow::bail!("No tracks supplied to multicam assembler.");
        }

        info!("[MULTICAM-ASSEMBLE] Building concat script for {} cuts…", switch_points.len() + 1);

        // Build a timeline of (start, end, track_index) segments
        let total_duration = {
            let mut dur = 0.0f64;
            for (i, track) in tracks.iter().enumerate() {
                if let Ok(d) = Self::probe_duration(&track.path).await {
                    let adjusted = d + offsets.get(i).copied().unwrap_or(0.0);
                    dur = dur.max(adjusted);
                }
            }
            dur
        };

        let mut segments: Vec<(f64, f64, usize)> = Vec::new();
        let mut prev_time = 0.0f64;
        let mut prev_track = switch_points.first().map(|_| 0usize).unwrap_or(0);

        for sp in switch_points {
            segments.push((prev_time, sp.master_time, prev_track));
            prev_time = sp.master_time;
            prev_track = sp.target_track;
        }
        segments.push((prev_time, total_duration, prev_track));

        // Write individual clips via FFmpeg trim, then concatenate
        let tmp_dir = std::env::temp_dir().join("synoid_multicam");
        std::fs::create_dir_all(&tmp_dir).context("Creating multicam tmp dir")?;

        let mut clip_paths: Vec<PathBuf> = Vec::new();
        for (seg_idx, (start, end, track_idx)) in segments.iter().enumerate() {
            if end <= start {
                continue;
            }
            let track = tracks
                .get(*track_idx)
                .ok_or_else(|| anyhow::anyhow!("Track index {} out of range", track_idx))?;

            let clip_path = tmp_dir.join(format!("seg_{:04}.mp4", seg_idx));
            let offset = offsets.get(*track_idx).copied().unwrap_or(0.0);
            let actual_start = (start - offset).max(0.0);
            let duration = end - start;

            let status = Command::new("ffmpeg")
                .args(["-y", "-ss", &actual_start.to_string(), "-i"])
                .arg(&track.path)
                .args([
                    "-t",
                    &duration.to_string(),
                    "-c:v",
                    "libx264",
                    "-c:a",
                    "aac",
                    "-preset",
                    "fast",
                ])
                .arg(&clip_path)
                .status()
                .await
                .context("FFmpeg clip extraction")?;

            if status.success() {
                clip_paths.push(clip_path);
            }
        }

        // Build concat list
        let mut concat_txt = String::new();
        for p in &clip_paths {
            concat_txt.push_str(&format!("file '{}'\n", p.display()));
        }
        let list_path = tmp_dir.join("concat_list.txt");
        std::fs::write(&list_path, &concat_txt).context("Writing concat list")?;

        let status = Command::new("ffmpeg")
            .args(["-y", "-f", "concat", "-safe", "0", "-i"])
            .arg(&list_path)
            .args(["-c", "copy"])
            .arg(output)
            .status()
            .await
            .context("FFmpeg multicam concat")?;

        // Clean up temp clips
        for p in &clip_paths {
            let _ = std::fs::remove_file(p);
        }
        let _ = std::fs::remove_file(&list_path);
        let _ = std::fs::remove_dir(&tmp_dir);

        if !status.success() {
            anyhow::bail!("FFmpeg multicam assembly failed.");
        }

        info!("[MULTICAM-ASSEMBLE] Assembly complete: {:?}", output);
        Ok(())
    }

    // ── Internal Helpers ─────────────────────────────────────────────────────

    /// Use FFmpeg's `astats` filter to extract per-frame RMS energy.
    async fn extract_energy_profile(path: &Path) -> Result<Vec<EnergyFrame>> {
        let output = Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(path)
            .args([
                "-af",
                "astats=metadata=1:reset=1,ametadata=print:key=lavfi.astats.Overall.RMS_level:file=-",
                "-vn",
                "-f",
                "null",
                "-",
            ])
            .output()
            .await
            .context("FFmpeg astats extraction")?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut frames: Vec<EnergyFrame> = Vec::new();
        let mut last_pts: f64 = 0.0;

        for line in text.lines() {
            if line.starts_with("frame:") {
                if let Some(ts_part) = line.split("pts_time:").nth(1) {
                    if let Ok(ts) = ts_part.trim().split_whitespace().next().unwrap_or("").parse::<f64>() {
                        last_pts = ts;
                    }
                }
            } else if line.contains("lavfi.astats.Overall.RMS_level=") {
                if let Some(val_str) = line.split('=').last() {
                    if let Ok(rms) = val_str.trim().parse::<f64>() {
                        // Convert from dBFS to linear for easier comparison
                        let linear = 10.0f64.powf(rms / 20.0);
                        frames.push(EnergyFrame { time: last_pts, energy: linear });
                    }
                }
            }
        }

        Ok(frames)
    }

    /// Simple cross-correlation: returns the time offset (seconds) of `slave`
    /// relative to `master` that maximises their energy profile similarity.
    fn cross_correlate_offset(master: &[EnergyFrame], slave: &[EnergyFrame]) -> f64 {
        if master.is_empty() || slave.is_empty() {
            return 0.0;
        }

        // Sample both profiles at 0.1s intervals up to 30 s search range
        let step = 0.1f64;
        let max_offset = 30.0f64;
        let mut best_offset = 0.0f64;
        let mut best_score = f64::NEG_INFINITY;

        let mut offset = -max_offset;
        while offset <= max_offset {
            let score = Self::correlation_score(master, slave, offset, step);
            if score > best_score {
                best_score = score;
                best_offset = offset;
            }
            offset += step;
        }

        best_offset
    }

    fn correlation_score(
        master: &[EnergyFrame],
        slave: &[EnergyFrame],
        shift: f64,
        step: f64,
    ) -> f64 {
        let duration = master.last().map(|f| f.time).unwrap_or(0.0);
        let mut t = 0.0f64;
        let mut score = 0.0f64;
        let mut count = 0u32;

        while t < duration {
            let m_e = Self::interp_energy(master, t);
            let s_e = Self::interp_energy(slave, t + shift);
            score += m_e * s_e;
            count += 1;
            t += step;
        }

        if count > 0 { score / count as f64 } else { 0.0 }
    }

    fn interp_energy(frames: &[EnergyFrame], t: f64) -> f64 {
        if frames.is_empty() {
            return 0.0;
        }
        // Binary search for the closest frame
        match frames.binary_search_by(|f| f.time.partial_cmp(&t).unwrap_or(std::cmp::Ordering::Less)) {
            Ok(idx) => frames[idx].energy,
            Err(idx) => {
                if idx == 0 {
                    frames[0].energy
                } else if idx >= frames.len() {
                    frames[frames.len() - 1].energy
                } else {
                    let a = &frames[idx - 1];
                    let b = &frames[idx];
                    let ratio = (t - a.time) / (b.time - a.time + 1e-9);
                    a.energy + ratio * (b.energy - a.energy)
                }
            }
        }
    }

    fn average_energy_in_window(frames: &[EnergyFrame], start: f64, end: f64) -> f64 {
        let relevant: Vec<f64> = frames
            .iter()
            .filter(|f| f.time >= start && f.time < end)
            .map(|f| f.energy)
            .collect();
        if relevant.is_empty() {
            return 0.0;
        }
        relevant.iter().sum::<f64>() / relevant.len() as f64
    }

    async fn probe_duration(path: &Path) -> Result<f64> {
        crate::agent::source_tools::get_video_duration(path)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}
