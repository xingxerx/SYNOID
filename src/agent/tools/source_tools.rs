// SYNOID Source Tools - Video Acquisition
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// This module currently handles:
// 1. YouTube downloading via yt-dlp (with optional browser auth)
// 2. Local file duration extraction via ffprobe
// 3. Directory scanning for video files
// 4. YouTube Search via ytsearch

use crate::agent::engines::process_utils::CommandExt;
use crate::agent::tools::production_tools::safe_arg_path;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::info;

pub(crate) fn sanitize_title_for_filename(title: &str) -> String {
    let sanitized: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        "Unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Find the Deno binary path to pass to yt-dlp's --js-runtimes flag.
fn find_deno_path() -> Option<String> {
    let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let userprofile = std::env::var("USERPROFILE").unwrap_or_default();

    let candidates = [
        // winget / installer default locations
        format!("{}\\Programs\\deno\\deno.exe", localappdata),
        format!("{}\\.deno\\bin\\deno.exe", userprofile),
        format!("{}\\deno\\deno.exe", localappdata),
        format!("{}\\Microsoft\\WinGet\\Packages\\DenoLand.Deno_Microsoft.Winget.Source_8wekyb3d8bbwe\\deno.exe", localappdata),
        format!("{}\\.lmstudio\\.internal\\utils\\deno.exe", userprofile),
    ];

    for path in &candidates {
        if std::path::Path::new(path).exists() {
            tracing::info!("[SOURCE] 🦕 Found Deno at: {}", path);
            return Some(path.clone());
        }
    }

    // Fallback: resolve via PATH
    let resolver = if cfg!(windows) { "where" } else { "which" };
    if let Ok(out) = std::process::Command::new(resolver).stealth().arg("deno").output() {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !path.is_empty() {
                tracing::info!("[SOURCE] 🦕 Found Deno via {}: {}", resolver, path);
                return Some(path);
            }
        }
    }

    tracing::warn!("[SOURCE] ⚠️ Deno not found. yt-dlp JS runtime unavailable.");
    None
}

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
    // 1. Check standalone 'yt-dlp' binary first.
    // On Windows the pip-installed "yt-dlp" is a Python launcher wrapper; spawning it
    // with CREATE_NO_WINDOW still lets it internally call python.exe without that flag,
    // which opens a visible console. Skip the standalone check on Windows and fall through
    // to the python -m yt_dlp path so we can apply stealth directly to python.exe.
    let standalone_candidates: &[&str] = if cfg!(windows) {
        &[]
    } else {
        &[
            "yt-dlp",
            "/usr/bin/yt-dlp",
            "/usr/local/bin/yt-dlp",
            "~/.local/bin/yt-dlp",
        ]
    };
    for &bin in standalone_candidates {
        // Toki's Command on Windows might fail to execute python scripts with shebangs if running in some mixed WSL setups.
        // First try it natively.
        match Command::new(bin).stealth().arg("--version").output().await {
            Ok(output) => {
                if output.status.success() {
                    tracing::info!("[SOURCE] ✅ Found standalone 'yt-dlp' binary at '{}'", bin);
                    return bin.to_string();
                } else {
                    tracing::warn!(
                        "[SOURCE] '{}' binary exists but --version failed: {}",
                        bin,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    "[SOURCE] Command '{}' failed to execute directly: {}",
                    bin,
                    e
                );
                // If execution failed (e.g., Exec format error or not found), try explicitly with python3
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::info!("[SOURCE] Trying to execute '{}' via python3...", bin);
                    if let Ok(py_out) = Command::new("python3")
                        .stealth()
                        .arg(bin)
                        .arg("--version")
                        .output()
                        .await
                    {
                        if py_out.status.success() {
                            tracing::info!(
                                "[SOURCE] ✅ Found standalone 'yt-dlp' binary via python3 at '{}'",
                                bin
                            );
                            // Return special syntax for our command builder later
                            return format!("python3|{}", bin);
                        }
                    }
                }
            }
        }
    }

    // 1.5 Try to find yt-dlp using 'which'
    if let Ok(output) = Command::new("which").stealth().arg("yt-dlp").output().await {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                // Test the found path
                match Command::new(&path).stealth().arg("--version").output().await {
                    Ok(out) => {
                        if out.status.success() {
                            tracing::info!(
                                "[SOURCE] ✅ Found standalone 'yt-dlp' via 'which' at '{}'",
                                path
                            );
                            return path;
                        } else {
                            tracing::warn!(
                                "[SOURCE] '{}' via which exists but --version failed: {}",
                                path,
                                String::from_utf8_lossy(&out.stderr)
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "[SOURCE] Command '{}' (from which) failed to execute: {}",
                            path,
                            e
                        );
                    }
                }
            }
        }
    }

    // 2. Candidate python commands to try (in order of preference for Linux/WSL then Windows)
    let candidates = if cfg!(windows) {
        vec!["python", "python3", "py"]
    } else {
        vec!["python3", "python"]
    };

    let mut best_fallback = None;

    for cmd in candidates {
        // Check if command exists
        let check_args = vec!["--version"];
        match Command::new(cmd).stealth().args(&check_args).output().await {
            Ok(output) => {
                if output.status.success() {
                    // Command exists, record it as a fallback
                    if best_fallback.is_none() {
                        best_fallback = Some(cmd.to_string());
                    }

                    // Now check for yt-dlp module
                    let module_args = vec!["-m", "yt_dlp", "--version"];
                    match Command::new(cmd).stealth().args(&module_args).output().await {
                        Ok(mod_out) => {
                            if mod_out.status.success() {
                                tracing::info!(
                                    "[SOURCE] ✅ Found valid Python with yt-dlp module: '{}'",
                                    cmd
                                );
                                return cmd.to_string();
                            } else {
                                tracing::debug!(
                                    "[SOURCE] '{}' exists but yt-dlp missing: {}",
                                    cmd,
                                    String::from_utf8_lossy(&mod_out.stderr)
                                );
                            }
                        }
                        Err(e) => {
                            tracing::debug!("[SOURCE] Failed to run '{} -m yt_dlp': {}", cmd, e);
                        }
                    }
                }
            }
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
        tracing::warn!(
            "[SOURCE] ⚠️ No valid Python environment found. Defaulting to 'python' which may fail."
        );
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
        .stealth()
        .args(["-m", "yt_dlp", "--version"])
        .stdin(std::process::Stdio::null())
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

    // Inject Deno JS runtime if available
    if let Some(deno) = find_deno_path() {
        args.push("--js-runtimes".to_string());
        args.push(format!("deno:{}", deno));
    }

    // If a specific browser cookie override is requested, honour it.
    // When cookies are present we use the standard web client — mobile client
    // emulation (ios/android) conflicts with web cookies and causes bot errors.
    // When no cookies are available, fall back to mobile client emulation.
    if let Some(browser) = auth_browser {
        if browser.starts_with('-') {
            return Err("Browser name cannot start with '-'".into());
        }
        args.push("--cookies-from-browser".to_string());
        args.push(browser.to_string());
    } else {
        // Use iOS/Android client emulation to bypass bot detection when cookies aren't used
        args.push("--extractor-args".to_string());
        args.push("youtube:player_client=ios,android".to_string());
    }

    args.extend_from_slice(&[
        "--no-warnings".to_string(),
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

    // Inject Deno JS runtime if available (required for modern YouTube)
    if let Some(deno) = find_deno_path() {
        args.push("--js-runtimes".to_string());
        args.push(format!("deno:{}", deno));
    }

    // If a specific browser cookie override is requested, honour it
    if let Some(browser) = auth_browser {
        if browser.starts_with('-') {
            return Err("Browser name cannot start with '-'".into());
        }
        args.push("--cookies-from-browser".to_string());
        args.push(browser.to_string());
    } else {
        // Use iOS/Android client emulation to bypass bot detection when cookies aren't used
        args.push("--extractor-args".to_string());
        args.push("youtube:player_client=ios,android".to_string());
    }

    args.extend_from_slice(&[
        "--no-warnings".to_string(),
        "-f".to_string(),
        "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best".to_string(),
        "-o".to_string(),
        safe_arg_path(output_path).to_string_lossy().to_string(),
    ]);

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
    let info_output = tokio::time::timeout(
        tokio::time::Duration::from_secs(120),
        Command::new(&python)
            .stealth()
            .args(&args)
            .stdin(std::process::Stdio::null())
            .output(),
    )
    .await
    .map_err(|_| format!("yt-dlp info command timed out after 120s"))??;
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
    let safe_title = sanitize_title_for_filename(&title);
    let filename = format!("{}.mp4", safe_title);
    let output_path = output_dir.join(&filename);
    let output_template = output_path.to_string_lossy().to_string();

    // Construct download arguments using helper
    let download_args = build_ytdlp_download_args(&python, url, &output_path, auth_browser)?;

    info!("[SOURCE] Starting download to: {}", output_template);
    // Reuse python command
    let status = tokio::time::timeout(
        tokio::time::Duration::from_secs(1800), // 30 mins
        Command::new(&python)
            .stealth()
            .args(&download_args)
            .stdin(std::process::Stdio::null())
            .status(),
    )
    .await
    .map_err(|_| format!("yt-dlp download command timed out"))??;

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

/// Detect the first available browser for --cookies-from-browser on this machine.
/// Returns e.g. "chrome", "edge", "firefox", or None if nothing found.
pub fn detect_browser() -> Option<String> {
    // Check env override first
    if let Ok(b) = std::env::var("SYNOID_BROWSER") {
        return Some(b);
    }

    #[cfg(windows)]
    {
        let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let candidates = [
            (format!("{}\\Google\\Chrome\\User Data", localappdata), "chrome"),
            (format!("{}\\Microsoft\\Edge\\User Data", localappdata), "edge"),
            (format!("{}\\Mozilla\\Firefox\\Profiles", appdata), "firefox"),
            (format!("{}\\BraveSoftware\\Brave-Browser\\User Data", localappdata), "brave"),
        ];
        for (path, name) in &candidates {
            if std::path::Path::new(path).exists() {
                tracing::info!("[SOURCE] 🍪 Auto-detected browser for cookies: {}", name);
                return Some(name.to_string());
            }
        }
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        let candidates = [
            (format!("{}/.config/google-chrome", home), "chrome"),
            (format!("{}/.config/chromium", home), "chromium"),
            (format!("{}/.mozilla/firefox", home), "firefox"),
        ];
        for (path, name) in &candidates {
            if std::path::Path::new(path).exists() {
                tracing::info!("[SOURCE] 🍪 Auto-detected browser for cookies: {}", name);
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Search YouTube for videos matching a query.
/// Pass `auth_browser` (e.g. "chrome") to use --cookies-from-browser for bot bypass.
pub async fn search_youtube(
    query: &str,
    limit: usize,
) -> Result<Vec<SourceInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let search_query = format!("ytsearch{}:{}", limit, query);
    info!("[SOURCE] Searching YouTube: {}", search_query);

    let python = get_python_command().await;
    let auth_browser = detect_browser();

    let mut args = Vec::new();
    if !python.ends_with("yt-dlp") {
        args.push("-m".to_string());
        args.push("yt_dlp".to_string());
    }

    // Inject Deno JS runtime if available
    if let Some(deno) = find_deno_path() {
        args.push("--js-runtimes".to_string());
        args.push(format!("deno:{}", deno));
    }

    // Pass browser cookies when available — needed on accounts flagged as bots.
    // When no cookies are available, fall back to mobile client emulation.
    if let Some(ref browser) = auth_browser {
        args.push("--cookies-from-browser".to_string());
        args.push(browser.clone());
    } else {
        // Use iOS/Android client emulation to bypass bot detection (avoids DPAPI issues)
        args.push("--extractor-args".to_string());
        args.push("youtube:player_client=ios,android".to_string());
    }

    args.extend_from_slice(&[
        "--no-warnings".to_string(),
        // flat-playlist: only reads the search index — no per-video page requests,
        // dramatically less likely to trigger bot detection
        "--flat-playlist".to_string(),
        "--print".to_string(),
        "%(title)s|%(id)s|%(duration)s".to_string(),
        "--no-download".to_string(),
        "--".to_string(),
    ]);
    args.push(search_query);

    let output = tokio::time::timeout(
        tokio::time::Duration::from_secs(120),
        Command::new(&python)
            .stealth()
            .args(&args)
            .stdin(std::process::Stdio::null())
            .output(),
    )
    .await
    .map_err(|_| format!("Search command timed out after 120s"))??;

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
        if parts.len() >= 3 {
            let title = parts[0].to_string();
            let id = parts[1].trim();
            let duration: f64 = parts[2].parse().unwrap_or(0.0);

            // Skip entries with no usable ID
            if id.is_empty() || id == "NA" {
                continue;
            }
            let url = format!("https://www.youtube.com/watch?v={}", id);

            // Filter out obviously bad results (e.g. 0 duration)
            if duration > 0.0 {
                results.push(SourceInfo {
                    title,
                    duration,
                    width: 0,
                    height: 0,
                    local_path: PathBuf::new(),
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
pub async fn get_video_duration(
    path: &Path,
) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let safe_path = safe_arg_path(path);

    // Execute ffprobe with a timeout to prevent hanging
    // Getting duration from header is usually instant.
    let output = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        Command::new("ffprobe")
            .stealth()
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
    .map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "ffprobe duration check timed out",
        )
    })??;
    let duration: f64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|_| format!("Failed to parse duration from ffprobe output"))?;
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
pub async fn web_search(
    query: &str,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    info!("[SOURCE] 🌐 Searching web for: '{}'", query);

    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );
    // Use a browser UA so DuckDuckGo returns HTML results; add 15 s timeout.
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(15))
        .pool_max_idle_per_host(2)
        .tcp_nodelay(true)
        .gzip(true)
        .build()?;

    let resp = client.get(&url).send().await?.text().await?;
    let document = scraper::Html::parse_document(&resp);
    let result_selector = scraper::Selector::parse(".result__body").unwrap();
    let title_selector = scraper::Selector::parse(".result__title a").unwrap();
    let snippet_selector = scraper::Selector::parse(".result__snippet").unwrap();

    let mut results = Vec::new();
    for result in document.select(&result_selector).take(5) {
        let title = result
            .select(&title_selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "No Title".to_string());

        let snippet = result
            .select(&snippet_selector)
            .next()
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
            build_ytdlp_info_args("python", "https://youtube.com/watch?v=123", Some("chrome"))
                .unwrap();

        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"yt_dlp".to_string()));
        assert!(args.contains(&"--".to_string()));

        // Test with standalone "yt-dlp"
        let args_standalone = build_ytdlp_info_args("yt-dlp", "https://youtube.com", None).unwrap();
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
        let args_sa =
            build_ytdlp_download_args("yt-dlp", "https://youtube.com", path, None).unwrap();
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

    #[test]
    fn test_sanitize_title_for_filename() {
        assert_eq!(
            sanitize_title_for_filename("So You Want To See The World? (Travel Film)"),
            "So You Want To See The World_ _Travel Film_"
        );
        assert_eq!(sanitize_title_for_filename("   "), "Unknown");
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
