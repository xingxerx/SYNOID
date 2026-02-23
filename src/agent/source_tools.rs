// SYNOID Source Tools - Video Acquisition
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// This module currently handles:
// 1. YouTube downloading via yt-dlp (with optional browser auth)
// 2. Local file duration extraction via ffprobe
// 3. Directory scanning for video files
// 4. YouTube Search via ytsearch

use crate::agent::production_tools::safe_arg_path;
use std::path::{Path, PathBuf};
use tokio::process::Command;
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

/// Find the available python command (python3, python, or py).
/// Prioritizes a command that has yt-dlp installed, but falls back to a valid python
/// interpreter if none have yt-dlp, so we don't get "No such file or directory".
pub async fn get_python_command() -> String {
    // 1. Check standalone 'yt-dlp' binary first
    let standalone_candidates = ["yt-dlp", "/usr/bin/yt-dlp", "/usr/local/bin/yt-dlp", "~/.local/bin/yt-dlp"];
    for &bin in &standalone_candidates {
        // Toki's Command on Windows might fail to execute python scripts with shebangs if running in some mixed WSL setups.
        // First try it natively.
        match Command::new(bin).arg("--version").output().await {
            Ok(output) => {
                if output.status.success() {
                     tracing::info!("[SOURCE] ✅ Found standalone 'yt-dlp' binary at '{}'", bin);
                     return bin.to_string();
                } else {
                     tracing::warn!("[SOURCE] '{}' binary exists but --version failed: {}", bin, String::from_utf8_lossy(&output.stderr));
                }
            },
            Err(e) => {
                 tracing::warn!("[SOURCE] Command '{}' failed to execute directly: {}", bin, e);
                 // If execution failed (e.g., Exec format error or not found), try explicitly with python3
                 if e.kind() != std::io::ErrorKind::NotFound {
                     tracing::info!("[SOURCE] Trying to execute '{}' via python3...", bin);
                     if let Ok(py_out) = Command::new("python3").arg(bin).arg("--version").output().await {
                         if py_out.status.success() {
                             tracing::info!("[SOURCE] ✅ Found standalone 'yt-dlp' binary via python3 at '{}'", bin);
                             // Return special syntax for our command builder later
                             return format!("python3|{}", bin);
                         }
                     }
                 }
            }
        }
    }

    // 1.5 Try to find yt-dlp using 'which'
    if let Ok(output) = Command::new("which").arg("yt-dlp").output().await {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                // Test the found path
                match Command::new(&path).arg("--version").output().await {
                    Ok(out) => {
                        if out.status.success() {
                            tracing::info!("[SOURCE] ✅ Found standalone 'yt-dlp' via 'which' at '{}'", path);
                            return path;
                        } else {
                            tracing::warn!("[SOURCE] '{}' via which exists but --version failed: {}", path, String::from_utf8_lossy(&out.stderr));
                        }
                    },
                    Err(e) => {
                        tracing::warn!("[SOURCE] Command '{}' (from which) failed to execute: {}", path, e);
                    }
                }
            }
        }
    }

    // 2. Candidate python commands to try (in order of preference for Linux/WSL then Windows)
    let candidates = ["python3", "python", "py"]; // 'python3' first for WSL
    
    let mut best_fallback = None;

    for cmd in candidates {
        // Check if command exists
        let check_args = vec!["--version"];
        match Command::new(cmd).args(&check_args).output().await {
            Ok(output) => {
                if output.status.success() {
                    // Command exists, record it as a fallback
                    if best_fallback.is_none() {
                        best_fallback = Some(cmd.to_string());
                    }

                    // Now check for yt-dlp module
                    let module_args = vec!["-m", "yt_dlp", "--version"];
                    match Command::new(cmd).args(&module_args).output().await {
                        Ok(mod_out) => {
                            if mod_out.status.success() {
                                tracing::info!("[SOURCE] ✅ Found valid Python with yt-dlp module: '{}'", cmd);
                                return cmd.to_string();
                            } else {
                                tracing::debug!("[SOURCE] '{}' exists but yt-dlp missing: {}", cmd, String::from_utf8_lossy(&mod_out.stderr));
                            }
                        },
                        Err(e) => {
                             tracing::debug!("[SOURCE] Failed to run '{} -m yt_dlp': {}", cmd, e);
                        }
                    }
                }
            },
            Err(_) => {
                // Command likely doesn't exist, just continue
                tracing::debug!("[SOURCE] Command '{}' not found", cmd);
            }
        }
    }

    // 3. Return the best fallback we found, or default to "python" if nothing worked
    if let Some(fallback) = best_fallback {
        tracing::warn!("[SOURCE] ⚠️ No valid Python+yt-dlp environment found. Using '{}' as fallback (commands requiring yt-dlp will fail).", fallback);
        fallback
    } else {
        tracing::warn!("[SOURCE] ⚠️ No valid Python environment found. Defaulting to 'python' which may fail.");
        "python".to_string()
    }
}

/// Check if yt-dlp is installed and accessible
pub async fn check_ytdlp() -> bool {
    let cmd = get_python_command().await;
    
    // If get_python_command returned "yt-dlp" or an absolute path to it, it's a standalone binary
    if cmd.ends_with("yt-dlp") {
        return true;
    }

    // Otherwise it's a python interpreter, check module
    Command::new(&cmd)
        .args(["-m", "yt_dlp", "--version"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn build_ytdlp_info_args(
    command: &str,
    url: &str,
    auth_browser: Option<&str>,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut args = Vec::new();
    
    // Only add "-m yt_dlp" if we are running via python
    if !command.ends_with("yt-dlp") {
        args.push("-m".to_string());
        args.push("yt_dlp".to_string());
    }

    args.extend_from_slice(&[
        "--print".to_string(),
        "%(title)s".to_string(),
        "--print".to_string(),
        "%(duration)s".to_string(),
        "--print".to_string(),
        "%(width)s".to_string(),
        "--print".to_string(),
        "%(height)s".to_string(),
        "--no-download".to_string(),
    ]);

    if let Some(browser) = auth_browser {
        if browser.starts_with('-') {
            return Err("Browser name cannot start with '-'".into());
        }
        args.push("--cookies-from-browser".to_string());
        args.push(browser.to_string());
    }

    args.push("--".to_string());
    args.push(url.to_string());

    Ok(args)
}

fn build_ytdlp_download_args(
    command: &str,
    url: &str,
    output_path: &Path,
    auth_browser: Option<&str>,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut args = Vec::new();

    // Only add "-m yt_dlp" if we are running via python
    if !command.ends_with("yt-dlp") {
        args.push("-m".to_string());
        args.push("yt_dlp".to_string());
    }
    
    args.extend_from_slice(&[
        "-f".to_string(),
        "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best".to_string(),
        "-o".to_string(),
        safe_arg_path(output_path).to_string_lossy().to_string(),
    ]);

    if let Some(browser) = auth_browser {
        if browser.starts_with('-') {
            return Err("Browser name cannot start with '-'".into());
        }
        args.push("--cookies-from-browser".to_string());
        args.push(browser.to_string());
    }

    args.push("--".to_string());
    args.push(url.to_string());

    Ok(args)
}

/// Download a YouTube video using yt-dlp
pub async fn download_youtube(
    url: &str,
    output_dir: &Path,
    auth_browser: Option<&str>,
) -> Result<SourceInfo, Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "[SOURCE] Downloading from YouTube: {} (Auth: {:?})",
        url, auth_browser
    );

    // Create output directory if it doesn't exist
    tokio::fs::create_dir_all(output_dir).await?;

    // Construct info arguments using helper
    let python = get_python_command().await; // Get command ONCE
    let args = build_ytdlp_info_args(&python, url, auth_browser)?;

    // First, get video info without downloading
    let info_output = Command::new(&python).args(&args).output().await?;
    if !info_output.status.success() {
        return Err(format!(
            "yt-dlp info failed with command '{}': {}",
            python,
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

    // Construct download arguments using helper
    let download_args = build_ytdlp_download_args(&python, url, &output_path, auth_browser)?;

    info!("[SOURCE] Starting download to: {}", output_template);
    // Reuse python command
    let status = Command::new(&python).args(&download_args).status().await?;

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
) -> Result<Vec<SourceInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let search_query = format!("ytsearch{}:{}", limit, query);
    info!("[SOURCE] Searching YouTube: {}", search_query);

    let python = get_python_command().await;
    
    let mut args = Vec::new();
    if !python.ends_with("yt-dlp") {
        args.push("-m".to_string());
        args.push("yt_dlp".to_string());
    }
    
    args.extend_from_slice(&[
        "--print".to_string(),
        "%(title)s|%(id)s|%(duration)s|%(webpage_url)s".to_string(),
        "--no-download".to_string(),
        "--".to_string(),
    ]);
    args.push(search_query);

    let output = Command::new(&python)
        .args(&args)
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!(
            "Search failed with command '{}': {}",
            python,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
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

/// Get video duration using ffprobe with a timeout
pub async fn get_video_duration(path: &Path) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let safe_path = safe_arg_path(path);

    // Execute ffprobe with a timeout to prevent hanging
    // Getting duration from header is usually instant.
    let output = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        Command::new("ffprobe")
            .kill_on_drop(true) // Ensure process is killed if timeout occurs
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(&safe_path)
            .output(),
    )
    .await
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "ffprobe duration check timed out"))??;
    let duration: f64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|_| {
            format!(
                "Failed to parse duration from ffprobe output"
            )
        })?;
    Ok(duration)
}

/// Scan a directory for all valid video files (Async)
pub async fn scan_directory_for_videos_async(dir: &Path) -> Vec<PathBuf> {
    let mut videos = Vec::new();
    let extensions = ["mp4", "mov", "mkv", "avi", "webm"];

    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(_) => return videos,
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
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
    videos
}

/// Legacy blocking scan (Use async version preferred)
#[allow(dead_code)]
pub async fn scan_directory_for_videos(dir: &Path) -> Vec<PathBuf> {
    let mut videos = Vec::new();
    let extensions = ["mp4", "mov", "mkv", "avi", "webm"];

    if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
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

/// Performs a free web search via DuckDuckGo (HTML scraping).
/// This provides a search capability without requiring a paid API.
pub async fn web_search(query: &str) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    info!("[SOURCE] 🌐 Searching web for: '{}'", query);
    
    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    let resp = client.get(&url).send().await?.text().await?;
    let document = scraper::Html::parse_document(&resp);
    let result_selector = scraper::Selector::parse(".result__body").unwrap();
    let title_selector = scraper::Selector::parse(".result__title a").unwrap();
    let snippet_selector = scraper::Selector::parse(".result__snippet").unwrap();

    let mut results = Vec::new();
    for result in document.select(&result_selector).take(5) {
        let title = result.select(&title_selector).next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "No Title".to_string());
        
        let snippet = result.select(&snippet_selector).next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "No Snippet".to_string());

        if !title.is_empty() {
            results.push((title, snippet));
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ytdlp_info_args() {
        // Test with "python"
        let args =
            build_ytdlp_info_args("python", "https://youtube.com/watch?v=123", Some("chrome")).unwrap();

        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"yt_dlp".to_string()));
        assert!(args.contains(&"--".to_string()));

        // Test with standalone "yt-dlp"
        let args_standalone =
            build_ytdlp_info_args("yt-dlp", "https://youtube.com", None).unwrap();
        assert!(!args_standalone.contains(&"-m".to_string()));
    }

    #[test]
    fn test_build_ytdlp_info_args_injection() {
        // Try to inject a flag via URL
        let args = build_ytdlp_info_args("python", "-v", None).unwrap();

        // Verify -v is after --
        let separator_idx = args.iter().position(|r| r == "--").unwrap();
        let url_idx = args.iter().position(|r| r == "-v").unwrap();
        assert!(separator_idx < url_idx);
    }

    #[test]
    fn test_build_ytdlp_download_args() {
        let path = Path::new("out.mp4");
        // Test with "python"
        let args = build_ytdlp_download_args("python", "https://youtube.com", path, None).unwrap();
        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"yt_dlp".to_string()));

        // Test with standalone
        let args_sa = build_ytdlp_download_args("yt-dlp", "https://youtube.com", path, None).unwrap();
        assert!(!args_sa.contains(&"-m".to_string()));
    }

    #[test]
    fn test_build_ytdlp_download_args_injection() {
        let path = Path::new("-out.mp4");
        let args = build_ytdlp_download_args("python", "https://youtube.com", path, None).unwrap();
        // Should be sanitized to ./ -out.mp4 or similar to prevent flag interpretation
        // safe_arg_path turns "-out.mp4" into "./-out.mp4"
        assert!(
            args.contains(&"./-out.mp4".to_string()) || args.contains(&".\\-out.mp4".to_string())
        );
    }

    #[test]
    fn test_bad_browser_name() {
        let res = build_ytdlp_info_args("python", "url", Some("-bad"));
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_web_search() {
        let results = web_search("rust programming").await.unwrap();
        assert!(!results.is_empty());
        println!("Search Results: {:?}", results);
    }

    // #[tokio::test]
    // async fn test_python_resolver() {
    //     let python = get_python_command().await;
    //     let output = Command::new(python)
    //         .arg("--version")
    //         .output()
    //         .await
    //         .unwrap();
    //     assert!(output.status.success());
    // }
}
