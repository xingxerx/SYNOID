// SYNOID Source Tools - Video Acquisition
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This module currently handles:
// 1. YouTube downloading via yt-dlp (with optional browser auth)
// 2. Local file duration extraction via ffprobe
// 3. Directory scanning for video files
// 4. YouTube Search via ytsearch

use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub title: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub local_path: PathBuf,
    pub original_url: Option<String>,
    pub format: String,
}

/// Check if yt-dlp is installed and accessible
pub async fn check_ytdlp() -> bool {
    Command::new("python")
        .args(["-m", "yt_dlp", "--version"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Download a YouTube video using yt-dlp
pub async fn download_youtube(
    url: &str,
    output_dir: &Path,
    auth_browser: Option<&str>,
) -> Result<SourceInfo, Box<dyn std::error::Error>> {
    info!(
        "[SOURCE] Downloading from YouTube: {} (Auth: {:?})",
        url, auth_browser
    );

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)?;

    // Construct base arguments
    let mut args = vec![
        "-m",
        "yt_dlp",
        "--print",
        "%(title)s",
        "--print",
        "%(duration)s",
        "--print",
        "%(width)s",
        "--print",
        "%(height)s",
        "--no-download",
    ];

    // Add authentication if provided
    if let Some(browser) = auth_browser {
        args.push("--cookies-from-browser");
        args.push(browser);
    }

    // [SENTINEL] Fix Argument Injection:
    // Ensure URL is treated as positional argument, not a flag
    args.push("--");
    args.push(url);

    // First, get video info without downloading
    let info_output = Command::new("python").args(&args).output()?;

    if !info_output.status.success() {
        return Err(format!(
            "yt-dlp info failed: {}",
            String::from_utf8_lossy(&info_output.stderr)
        )
        .into());
    }

    let stdout = String::from_utf8_lossy(&info_output.stdout);
    let mut lines = stdout.lines();

    let title = lines.next().unwrap_or("Unknown").to_string();
    let duration: f64 = lines.next().unwrap_or("0").parse().unwrap_or(0.0);
    let width: u32 = lines.next().unwrap_or("0").parse().unwrap_or(0);
    let height: u32 = lines.next().unwrap_or("0").parse().unwrap_or(0);

    // Prepare output filename (sanitized)
    let safe_title: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let filename = format!("{}.mp4", safe_title);
    let output_path = output_dir.join(&filename);
    let output_template = output_path.to_string_lossy().to_string();

    // Now download
    let mut download_args = vec![
        "-m",
        "yt_dlp",
        "-f",
        "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best",
        "-o",
        &output_template,
    ];

    if let Some(browser) = auth_browser {
        download_args.push("--cookies-from-browser");
        download_args.push(browser);
    }
    
    // [SENTINEL] Fix Argument Injection:
    download_args.push("--");
    download_args.push(url);

    info!("[SOURCE] Starting download to: {}", output_template);
    let status = Command::new("python").args(&download_args).status()?;

    if !status.success() {
        return Err("Download process failed".into());
    }

    Ok(SourceInfo {
        title,
        duration,
        width,
        height,
        local_path: output_path,
        original_url: Some(url.to_string()),
        format: "mp4".to_string(),
    })
}

/// Search YouTube for videos matching a query
pub async fn search_youtube(
    query: &str,
    limit: usize,
) -> Result<Vec<SourceInfo>, Box<dyn std::error::Error>> {
    let search_query = format!("ytsearch{}:{}", limit, query);
    info!("[SOURCE] Searching YouTube: {}", search_query);

    let output = Command::new("python")
        .args([
            "-m",
            "yt_dlp",
            "--print",
            "%(title)s|%(id)s|%(duration)s|%(webpage_url)s",
            "--no-download",
            "--",
            &search_query,
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!("Search failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            let title = parts[0].to_string();
            let _id = parts[1]; // Unused for now
            let duration: f64 = parts[2].parse().unwrap_or(0.0);
            let url = parts[3].to_string();

            // Filter out obviously bad results (e.g. 0 duration)
            if duration > 0.0 {
                results.push(SourceInfo {
                    title,
                    duration,
                    width: 0, // Search doesn't give dimensions easily without more API calls
                    height: 0,
                    local_path: PathBuf::new(), // Not downloaded yet
                    original_url: Some(url),
                    format: "online".to_string(),
                });
            }
        }
    }

    info!("[SOURCE] Found {} results", results.len());
    Ok(results)
}

/// Get video duration using ffprobe
pub async fn get_video_duration(path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            "--",
        ])
        .arg(path)
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let duration: f64 = output_str.trim().parse()?;
    Ok(duration)
}

/// Scan a directory for all valid video files
#[allow(dead_code)]
pub fn scan_directory_for_videos(dir: &Path) -> Vec<PathBuf> {
    let mut videos = Vec::new();
    let extensions = ["mp4", "mov", "mkv", "avi", "webm"];

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if extensions.contains(&ext_str.to_lowercase().as_str()) {
                            videos.push(path);
                        }
                    }
                }
            }
        }
    }
    videos
}
