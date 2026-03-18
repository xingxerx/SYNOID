// SYNOID Video Downloader Module
// Automatically downloads reference videos for the learning system
// Uses yt-dlp for safe, legal video downloads from approved sources

use std::path::{Path, PathBuf};
use std::fs;
use tokio::process::Command;
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};

const SENTINEL_FILE: &str = "AUTO_DOWNLOAD_ENABLED.txt";
const CONFIG_FILE: &str = "download_sources.json";
const MAX_VIDEOS: usize = 10;
const MIN_VIDEO_SIZE_MB: u64 = 5;
const MAX_VIDEO_SIZE_MB: u64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSource {
    /// URL or channel to download from
    pub url: String,
    /// Maximum number of videos to download from this source
    pub max_downloads: usize,
    /// Search query for this source (optional)
    pub search_query: Option<String>,
    /// Whether this source is enabled
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    pub sources: Vec<VideoSource>,
    /// Global maximum video duration in seconds
    pub max_duration_secs: u32,
    /// Preferred video quality (e.g., "720p", "1080p")
    pub quality: String,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            sources: vec![
                VideoSource {
                    url: "https://www.youtube.com/results?search_query=creative+commons+gaming+montage".to_string(),
                    max_downloads: 3,
                    search_query: Some("creative commons gaming montage".to_string()),
                    enabled: false,
                },
                VideoSource {
                    url: "https://www.youtube.com/results?search_query=creative+commons+tutorial".to_string(),
                    max_downloads: 2,
                    search_query: Some("creative commons tutorial".to_string()),
                    enabled: false,
                },
            ],
            max_duration_secs: 600, // 10 minutes max
            quality: "720p".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct DownloadResult {
    pub downloaded_count: usize,
    pub deleted_count: usize,
    pub errors: Vec<String>,
}

fn get_download_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Download")
}

fn get_config_path() -> PathBuf {
    get_download_dir().join(CONFIG_FILE)
}

/// Check if auto-download is enabled via sentinel file
pub fn is_auto_download_enabled() -> bool {
    get_download_dir().join(SENTINEL_FILE).exists()
}

/// Load download configuration from JSON file
pub fn load_config() -> Result<DownloadConfig, Box<dyn std::error::Error>> {
    let config_path = get_config_path();

    if !config_path.exists() {
        // Create default config
        let default_config = DownloadConfig::default();
        let json = serde_json::to_string_pretty(&default_config)?;
        fs::write(&config_path, json)?;
        info!("[DOWNLOADER] Created default config at {}", config_path.display());
        return Ok(default_config);
    }

    let json = fs::read_to_string(&config_path)?;
    let config: DownloadConfig = serde_json::from_str(&json)?;
    Ok(config)
}

/// Get current video count in Download folder
fn count_videos() -> usize {
    let download_dir = get_download_dir();
    if !download_dir.exists() {
        return 0;
    }

    fs::read_dir(&download_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(|ext| matches!(ext.to_lowercase().as_str(), "mp4" | "mkv" | "avi" | "mov"))
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

/// Delete oldest videos to make room for new downloads
fn delete_oldest_videos(target_count: usize) -> Result<usize, Box<dyn std::error::Error>> {
    let download_dir = get_download_dir();
    if !download_dir.exists() {
        return Ok(0);
    }

    // Get all video files with metadata
    let mut videos: Vec<(PathBuf, std::time::SystemTime)> = fs::read_dir(&download_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| matches!(ext.to_lowercase().as_str(), "mp4" | "mkv" | "avi" | "mov"))
                .unwrap_or(false)
        })
        .filter_map(|e| {
            let path = e.path();
            e.metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|modified| (path, modified))
        })
        .collect();

    if videos.len() <= target_count {
        return Ok(0);
    }

    // Sort by modification time (oldest first)
    videos.sort_by_key(|(_, modified)| *modified);

    let to_delete = videos.len() - target_count;
    let mut deleted = 0;

    for (path, _) in videos.iter().take(to_delete) {
        match fs::remove_file(path) {
            Ok(_) => {
                info!("[DOWNLOADER] 🗑️  Deleted old video: {}", path.display());
                deleted += 1;
            }
            Err(e) => {
                warn!("[DOWNLOADER] Failed to delete {}: {}", path.display(), e);
            }
        }
    }

    Ok(deleted)
}

/// Download a single video using yt-dlp
async fn download_video(
    url: &str,
    output_dir: &Path,
    config: &DownloadConfig,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // Check if yt-dlp is installed
    let check_ytdlp = Command::new("yt-dlp")
        .arg("--version")
        .output()
        .await;

    if check_ytdlp.is_err() {
        return Err("yt-dlp is not installed. Please install it from https://github.com/yt-dlp/yt-dlp".into());
    }

    let output_template = output_dir.join("%(title)s.%(ext)s");
    let max_duration = config.max_duration_secs.to_string();

    info!("[DOWNLOADER] 📥 Downloading from: {}", url);

    let output = Command::new("yt-dlp")
        .args([
            "--format", "bestvideo[height<=720]+bestaudio/best[height<=720]",
            "--merge-output-format", "mp4",
            "--no-playlist",
            "--max-downloads", "1",
            "--match-filter", &format!("duration < {}", max_duration),
            "--output", output_template.to_str().unwrap_or(""),
            "--no-overwrites",
            "--quiet",
            "--progress",
            url,
        ])
        .output()
        .await?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp failed: {}", err_msg).into());
    }

    // Find the downloaded file
    let downloaded = fs::read_dir(output_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext == "mp4")
                .unwrap_or(false)
        })
        .max_by_key(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
        .ok_or("No video file found after download")?;

    // Validate file size
    let file_size_mb = fs::metadata(&downloaded)?.len() / (1024 * 1024);
    if file_size_mb < MIN_VIDEO_SIZE_MB {
        fs::remove_file(&downloaded)?;
        return Err(format!("Video too small ({} MB < {} MB)", file_size_mb, MIN_VIDEO_SIZE_MB).into());
    }
    if file_size_mb > MAX_VIDEO_SIZE_MB {
        fs::remove_file(&downloaded)?;
        return Err(format!("Video too large ({} MB > {} MB)", file_size_mb, MAX_VIDEO_SIZE_MB).into());
    }

    info!("[DOWNLOADER] ✅ Downloaded: {} ({} MB)", downloaded.display(), file_size_mb);
    Ok(downloaded)
}

/// Main function: Download new videos and manage the collection
pub async fn refresh_videos() -> Result<DownloadResult, Box<dyn std::error::Error>> {
    let mut result = DownloadResult {
        downloaded_count: 0,
        deleted_count: 0,
        errors: Vec::new(),
    };

    // Check if auto-download is enabled
    if !is_auto_download_enabled() {
        info!("[DOWNLOADER] Auto-download is disabled (no {} file)", SENTINEL_FILE);
        return Ok(result);
    }

    let download_dir = get_download_dir();
    fs::create_dir_all(&download_dir)?;

    // Load configuration
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            error!("[DOWNLOADER] Failed to load config: {}", e);
            result.errors.push(format!("Config load failed: {}", e));
            return Ok(result);
        }
    };

    let current_count = count_videos();
    info!("[DOWNLOADER] Current video count: {} / {}", current_count, MAX_VIDEOS);

    if current_count >= MAX_VIDEOS {
        info!("[DOWNLOADER] At max capacity, will delete oldest videos");
        // Delete enough videos to make room for new downloads
        let enabled_sources: Vec<_> = config.sources.iter().filter(|s| s.enabled).collect();
        let target_downloads = enabled_sources.iter().map(|s| s.max_downloads).sum::<usize>().min(3);

        if target_downloads > 0 {
            match delete_oldest_videos(MAX_VIDEOS - target_downloads) {
                Ok(deleted) => {
                    result.deleted_count = deleted;
                    info!("[DOWNLOADER] Deleted {} old video(s) to make room", deleted);
                }
                Err(e) => {
                    warn!("[DOWNLOADER] Failed to delete old videos: {}", e);
                    result.errors.push(format!("Deletion failed: {}", e));
                }
            }
        }
    }

    // Download from enabled sources
    for source in config.sources.iter().filter(|s| s.enabled) {
        let current = count_videos();
        if current >= MAX_VIDEOS {
            info!("[DOWNLOADER] Reached max videos ({}), stopping downloads", MAX_VIDEOS);
            break;
        }

        let remaining_slots = MAX_VIDEOS - current;
        let downloads_for_source = source.max_downloads.min(remaining_slots);

        info!(
            "[DOWNLOADER] Attempting {} download(s) from: {}",
            downloads_for_source, source.url
        );

        for i in 0..downloads_for_source {
            match download_video(&source.url, &download_dir, &config).await {
                Ok(path) => {
                    result.downloaded_count += 1;
                    info!("[DOWNLOADER] [{}/{}] Downloaded: {}", i + 1, downloads_for_source, path.display());
                }
                Err(e) => {
                    warn!("[DOWNLOADER] [{}/{}] Download failed: {}", i + 1, downloads_for_source, e);
                    result.errors.push(format!("Download {}: {}", i + 1, e));
                }
            }
        }
    }

    if result.downloaded_count > 0 || result.deleted_count > 0 {
        info!(
            "[DOWNLOADER] 📊 Session complete: {} downloaded, {} deleted",
            result.downloaded_count, result.deleted_count
        );
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = DownloadConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: DownloadConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sources.len(), config.sources.len());
    }

    #[test]
    fn test_sentinel_check() {
        // Should not crash if directory doesn't exist
        let _ = is_auto_download_enabled();
    }
}
