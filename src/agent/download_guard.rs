// SYNOID Download Guard — Safe Acquisition Layer
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Protects the multi-agent mixture from downloading viruses, malware,
// or corrupt media. Every URL is screened before fetch, and every
// downloaded file is validated before the system learns from it.

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use tracing::{info, warn};

/// Allowed media extensions for downloaded content.
const SAFE_EXTENSIONS: &[&str] = &[
    ".mp4", ".mkv", ".webm", ".mov", ".avi",
    ".wav", ".mp3", ".flac", ".ogg", ".aac",
];

/// Suspicious URL patterns that indicate non-media content.
const BLOCKED_URL_PATTERNS: &[&str] = &[
    ".exe", ".bat", ".cmd", ".ps1", ".msi", ".scr",
    ".vbs", ".js", ".hta", ".pif", ".cpl",
    ".dll", ".sys", ".inf", ".reg",
    "malware", "trojan", "crack", "keygen", "warez",
];

/// Minimum sane file size (10 KB) — smaller files are likely stubs/traps.
const MIN_FILE_SIZE: u64 = 10 * 1024;

/// Maximum sane file size (10 GB).
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024 * 1024;

pub struct DownloadGuard;

impl DownloadGuard {
    // -----------------------------------------------------------------------
    // URL Validation
    // -----------------------------------------------------------------------

    /// Validate a URL before downloading. Returns `Ok(())` if safe.
    pub fn validate_url(url: &str) -> Result<(), String> {
        let url_lower = url.to_lowercase();

        // 1. Must be HTTPS (or known safe local path)
        if !url_lower.starts_with("https://") && !url_lower.starts_with("http://localhost") {
            // Allow ytsearch: protocol used by yt-dlp
            if !url_lower.starts_with("ytsearch") {
                warn!("[GUARD] 🛡️ Blocked non-HTTPS URL: {}", url);
                return Err(format!("Unsafe protocol — only HTTPS allowed: {}", url));
            }
        }

        // 2. Check for blocked patterns in URL
        for pattern in BLOCKED_URL_PATTERNS {
            if url_lower.contains(pattern) {
                warn!(
                    "[GUARD] 🛡️ Blocked suspicious URL pattern '{}': {}",
                    pattern, url
                );
                return Err(format!(
                    "URL contains blocked pattern '{}' — possible malware",
                    pattern
                ));
            }
        }

        // 3. Block data URIs and javascript URIs
        if url_lower.starts_with("data:") || url_lower.starts_with("javascript:") {
            return Err("Blocked injection URI scheme".to_string());
        }

        info!("[GUARD] ✅ URL passed safety check: {}", url);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Downloaded File Validation
    // -----------------------------------------------------------------------

    /// Validate a downloaded file on disk. Returns `Ok(())` if safe to learn from.
    pub fn validate_downloaded_file(path: &Path) -> Result<(), String> {
        // 1. File must exist
        if !path.exists() {
            return Err(format!("File does not exist: {:?}", path));
        }

        // 2. Extension check
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_lowercase()))
            .unwrap_or_default();

        if !SAFE_EXTENSIONS.contains(&ext.as_str()) {
            warn!(
                "[GUARD] 🛡️ Blocked unsafe file extension '{}': {:?}",
                ext, path
            );
            return Err(format!(
                "Unsafe file extension '{}' — only media files allowed",
                ext
            ));
        }

        // 3. File size bounds
        let metadata = fs::metadata(path)
            .map_err(|e| format!("Cannot read file metadata: {}", e))?;

        let size = metadata.len();
        if size < MIN_FILE_SIZE {
            return Err(format!(
                "File too small ({} bytes) — likely a stub or trap",
                size
            ));
        }
        if size > MAX_FILE_SIZE {
            return Err(format!(
                "File too large ({} bytes) — exceeds 10 GB limit",
                size
            ));
        }

        // 4. Magic byte check — detect executables disguised as media
        Self::check_magic_bytes(path)?;

        info!(
            "[GUARD] ✅ File passed safety check: {:?} ({} bytes)",
            path.file_name().unwrap_or_default(),
            size
        );
        Ok(())
    }

    /// Inspect the first bytes of a file for executable signatures.
    fn check_magic_bytes(path: &Path) -> Result<(), String> {
        let mut file = File::open(path)
            .map_err(|e| format!("Cannot open file for magic-byte check: {}", e))?;

        let mut header = [0u8; 4];
        let bytes_read = file
            .read(&mut header)
            .map_err(|e| format!("Cannot read file header: {}", e))?;

        if bytes_read < 2 {
            return Err("File too small to validate header".to_string());
        }

        // PE executable (Windows .exe/.dll)
        if header[0] == b'M' && header[1] == b'Z' {
            warn!("[GUARD] 🛡️ PE executable detected: {:?}", path);
            return Err("File contains Windows executable (MZ header) — BLOCKED".to_string());
        }

        // ELF executable (Linux)
        if bytes_read >= 4 && header[0] == 0x7F && &header[1..4] == b"ELF" {
            warn!("[GUARD] 🛡️ ELF executable detected: {:?}", path);
            return Err("File contains Linux executable (ELF header) — BLOCKED".to_string());
        }

        // Script shebang (#!)
        if header[0] == b'#' && header[1] == b'!' {
            warn!("[GUARD] 🛡️ Script shebang detected: {:?}", path);
            return Err("File contains script shebang (#!) — BLOCKED".to_string());
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Filename Sanitization
    // -----------------------------------------------------------------------

    /// Strip path traversal attacks and dangerous characters from filenames.
    pub fn sanitize_filename(name: &str) -> String {
        name.replace("..", "")
            .replace('/', "_")
            .replace('\\', "_")
            .replace('\0', "")
            .replace(':', "_")
            .replace('*', "_")
            .replace('?', "_")
            .replace('"', "_")
            .replace('<', "_")
            .replace('>', "_")
            .replace('|', "_")
            .trim()
            .to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_allow_https_url() {
        let result = DownloadGuard::validate_url("https://www.youtube.com/watch?v=abc123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_block_http_url() {
        let result = DownloadGuard::validate_url("http://evil-site.com/video.mp4");
        assert!(result.is_err());
    }

    #[test]
    fn test_allow_localhost() {
        let result = DownloadGuard::validate_url("http://localhost:3000/api");
        assert!(result.is_ok());
    }

    #[test]
    fn test_block_executable_url() {
        let result = DownloadGuard::validate_url("https://example.com/download.exe");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(".exe"));
    }

    #[test]
    fn test_block_malware_keyword_url() {
        let result = DownloadGuard::validate_url("https://crack-site.com/keygen-video.mp4");
        assert!(result.is_err());
    }

    #[test]
    fn test_block_data_uri() {
        let result = DownloadGuard::validate_url("data:text/html,<script>alert(1)</script>");
        assert!(result.is_err());
    }

    #[test]
    fn test_block_javascript_uri() {
        let result = DownloadGuard::validate_url("javascript:alert(1)");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_nonexistent_file() {
        let result =
            DownloadGuard::validate_downloaded_file(Path::new("__nonexistent_xyz_test.mp4"));
        assert!(result.is_err());
    }

    #[test]
    fn test_block_executable_bytes() {
        let dir = std::env::temp_dir().join("synoid_guard_test");
        let _ = fs::create_dir_all(&dir);
        let fake_exe = dir.join("sneaky.mp4");

        // Write a PE header disguised as .mp4
        let mut f = File::create(&fake_exe).unwrap();
        f.write_all(b"MZ").unwrap();
        // Pad to pass minimum size check
        f.write_all(&vec![0u8; 20_000]).unwrap();
        f.flush().unwrap();

        let result = DownloadGuard::validate_downloaded_file(&fake_exe);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("MZ"));

        let _ = fs::remove_file(&fake_exe);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_block_elf_bytes() {
        let dir = std::env::temp_dir().join("synoid_guard_test_elf");
        let _ = fs::create_dir_all(&dir);
        let fake = dir.join("sneaky.mp4");

        let mut f = File::create(&fake).unwrap();
        f.write_all(&[0x7F, b'E', b'L', b'F']).unwrap();
        f.write_all(&vec![0u8; 20_000]).unwrap();
        f.flush().unwrap();

        let result = DownloadGuard::validate_downloaded_file(&fake);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ELF"));

        let _ = fs::remove_file(&fake);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sanitize_path_traversal() {
        assert_eq!(
            DownloadGuard::sanitize_filename("../../etc/passwd"),
            "__etc_passwd"
        );
        assert_eq!(
            DownloadGuard::sanitize_filename("video<>|.mp4"),
            "video___. mp4"
                .replace(". ", ".")
        );
    }

    #[test]
    fn test_sanitize_normal_name() {
        assert_eq!(
            DownloadGuard::sanitize_filename("cool_video_2026.mp4"),
            "cool_video_2026.mp4"
        );
    }

    #[test]
    fn test_block_unsafe_extension() {
        let dir = std::env::temp_dir().join("synoid_guard_ext_test");
        let _ = fs::create_dir_all(&dir);
        let bad_file = dir.join("payload.exe");

        let mut f = File::create(&bad_file).unwrap();
        f.write_all(&vec![0u8; 20_000]).unwrap();
        f.flush().unwrap();

        let result = DownloadGuard::validate_downloaded_file(&bad_file);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(".exe"));

        let _ = fs::remove_file(&bad_file);
        let _ = fs::remove_dir_all(&dir);
    }
}
