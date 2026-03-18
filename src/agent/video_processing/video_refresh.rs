// SYNOID Video Refresh Manager
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Safely manages the Download folder to keep only the most recent MAX_VIDEOS
// reference videos. Older videos are archived (not deleted) to prevent data loss.
//
// SAFETY PRINCIPLES:
// 1. NEVER delete user files - only move to archive
// 2. NEVER download copyrighted content automatically
// 3. Require explicit user consent for any downloads
// 4. Use sentinel pattern to prevent accidental damage

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tracing::{info, warn};

use super::video_style_learner::{get_download_dir, MAX_VIDEOS};

/// Archive directory for videos that exceed MAX_VIDEOS limit
pub fn get_archive_dir() -> PathBuf {
    PathBuf::from(r"D:\SYNOID\Download\_Archive")
}

/// Sentinel file that must exist to enable auto-archiving
const SENTINEL_FILE: &str = "AUTO_ARCHIVE_ENABLED.txt";

/// Check if auto-archiving is enabled via sentinel file
pub fn is_auto_archive_enabled() -> bool {
    get_download_dir().join(SENTINEL_FILE).exists()
}

/// Enable auto-archiving by creating the sentinel file
pub fn enable_auto_archive() -> std::io::Result<()> {
    let sentinel_path = get_download_dir().join(SENTINEL_FILE);
    std::fs::write(
        &sentinel_path,
        "Auto-archiving is ENABLED.\n\
         SYNOID will move old videos to _Archive folder when > 10 videos exist.\n\
         Delete this file to disable auto-archiving.\n\
         \n\
         Created: {}\n",
    )?;
    info!("[VIDEO_REFRESH] ✅ Auto-archiving enabled via sentinel: {:?}", sentinel_path);
    Ok(())
}

/// Disable auto-archiving by removing the sentinel file
pub fn disable_auto_archive() -> std::io::Result<()> {
    let sentinel_path = get_download_dir().join(SENTINEL_FILE);
    if sentinel_path.exists() {
        std::fs::remove_file(&sentinel_path)?;
        info!("[VIDEO_REFRESH] 🚫 Auto-archiving disabled");
    }
    Ok(())
}

/// Get modification time of a file as nanoseconds since UNIX epoch
fn modified_time_nanos(path: &Path) -> u128 {
    std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

/// Result of an archive operation
#[derive(Debug)]
pub struct ArchiveResult {
    pub archived_count: usize,
    pub archived_files: Vec<String>,
    pub kept_count: usize,
}

/// Archive old videos if Download folder exceeds MAX_VIDEOS.
///
/// SAFETY: Only runs if sentinel file exists. Never deletes - only moves to archive.
///
/// Algorithm:
/// 1. List all .mp4 files in Download folder
/// 2. Sort by modification time (newest first)
/// 3. If count > MAX_VIDEOS, move oldest to _Archive folder
/// 4. Return summary of what was archived
pub fn archive_old_videos() -> Result<ArchiveResult, Box<dyn std::error::Error>> {
    // SAFETY: Check sentinel first
    if !is_auto_archive_enabled() {
        return Ok(ArchiveResult {
            archived_count: 0,
            archived_files: Vec::new(),
            kept_count: 0,
        });
    }

    let download_dir = get_download_dir();
    if !download_dir.exists() {
        return Err(format!("Download directory does not exist: {:?}", download_dir).into());
    }

    // Collect all MP4 files
    let mut videos: Vec<PathBuf> = std::fs::read_dir(&download_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|e| e.to_str()) == Some("mp4")
                // Exclude sentinel file
                && p.file_name().and_then(|n| n.to_str()) != Some(SENTINEL_FILE)
        })
        .collect();

    let total_count = videos.len();

    // If we have MAX_VIDEOS or fewer, nothing to do
    if total_count <= MAX_VIDEOS {
        info!(
            "[VIDEO_REFRESH] ✅ Video count ({}) within limit ({}), no archiving needed",
            total_count, MAX_VIDEOS
        );
        return Ok(ArchiveResult {
            archived_count: 0,
            archived_files: Vec::new(),
            kept_count: total_count,
        });
    }

    // Sort by modification time (newest first)
    videos.sort_by(|a, b| {
        modified_time_nanos(b)
            .cmp(&modified_time_nanos(a))
            .then_with(|| a.cmp(b))
    });

    // Split into keep and archive lists
    let (keep, to_archive) = videos.split_at(MAX_VIDEOS);
    let keep_count = keep.len();
    let archive_count = to_archive.len();

    info!(
        "[VIDEO_REFRESH] 📦 Archiving {} old video(s) (keeping newest {})",
        archive_count, keep_count
    );

    // Create archive directory
    let archive_dir = get_archive_dir();
    std::fs::create_dir_all(&archive_dir)?;

    // Move files to archive
    let mut archived_files = Vec::new();
    for old_video in to_archive {
        if let Some(filename) = old_video.file_name() {
            let archive_path = archive_dir.join(filename);

            // If file already exists in archive, add timestamp to avoid collision
            let final_archive_path = if archive_path.exists() {
                let stem = old_video.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
                let timestamp = std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                archive_dir.join(format!("{}_{}.mp4", stem, timestamp))
            } else {
                archive_path
            };

            match std::fs::rename(old_video, &final_archive_path) {
                Ok(_) => {
                    let name = filename.to_string_lossy().to_string();
                    info!("[VIDEO_REFRESH]   ✓ Archived: {}", name);
                    archived_files.push(name);
                }
                Err(e) => {
                    warn!(
                        "[VIDEO_REFRESH]   ✗ Failed to archive {:?}: {}",
                        filename, e
                    );
                }
            }
        }
    }

    Ok(ArchiveResult {
        archived_count: archived_files.len(),
        archived_files,
        kept_count: keep_count,
    })
}

/// Restore a video from archive back to Download folder
pub fn restore_from_archive(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let archive_dir = get_archive_dir();
    let archive_path = archive_dir.join(filename);

    if !archive_path.exists() {
        return Err(format!("Video not found in archive: {}", filename).into());
    }

    let download_path = get_download_dir().join(filename);

    std::fs::rename(&archive_path, &download_path)?;
    info!("[VIDEO_REFRESH] ✅ Restored from archive: {}", filename);

    Ok(())
}

/// List all videos currently in the archive
pub fn list_archived_videos() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let archive_dir = get_archive_dir();

    if !archive_dir.exists() {
        return Ok(Vec::new());
    }

    let archived: Vec<String> = std::fs::read_dir(&archive_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path().extension().and_then(|ext| ext.to_str()) == Some("mp4")
        })
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .collect();

    Ok(archived)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentinel_pattern() {
        // Sentinel should not exist by default
        disable_auto_archive().ok();
        assert!(!is_auto_archive_enabled());

        // Enable it
        enable_auto_archive().unwrap();
        assert!(is_auto_archive_enabled());

        // Disable it
        disable_auto_archive().unwrap();
        assert!(!is_auto_archive_enabled());
    }
}
