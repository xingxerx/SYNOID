// SYNOID Download Guard — Safe Acquisition Layer v2.0
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID
//
// Protects the multi-agent mixture from downloading viruses, malware,
// or corrupt media. Every URL is screened before fetch, and every
// downloaded file is validated before the system learns from it.
//
// Security tiers:
//   Trusted  — YouTube + known free-culture repositories (HTTPS required)
//   Allowed  — Any HTTPS URL that passes pattern checks
//   Blocked  — Non-HTTPS, blocked patterns, injection URIs

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use tracing::{info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Allowed media extensions for downloaded content.
const SAFE_EXTENSIONS: &[&str] = &[
    ".mp4", ".mkv", ".webm", ".mov", ".avi", ".wav", ".mp3", ".flac", ".ogg", ".aac",
];

/// Suspicious URL patterns that indicate non-media content.
const BLOCKED_URL_PATTERNS: &[&str] = &[
    ".exe", ".bat", ".cmd", ".ps1", ".msi", ".scr", ".vbs", ".js", ".hta", ".pif", ".cpl",
    ".dll", ".sys", ".inf", ".reg", "malware", "trojan", "crack", "keygen", "warez",
    "ransomware", "rootkit", "spyware",
];

/// Domains known to serve free/open-source or Creative-Commons video content.
/// Downloads from these domains are treated as Trusted (still fully validated).
pub const TRUSTED_VIDEO_DOMAINS: &[&str] = &[
    // YouTube (primary learning source)
    "youtube.com",
    "youtu.be",
    "www.youtube.com",
    // Internet Archive — vast public-domain collection
    "archive.org",
    "ia600",   // archive.org CDN prefix pattern
    // Free stock footage (CC0 / royalty-free)
    "pexels.com",
    "videos.pexels.com",
    "pixabay.com",
    "cdn.pixabay.com",
    "coverr.co",
    "assets.coverr.co",
    // Videvo free stock
    "videvo.net",
    // Vimeo — many CC-licensed uploads
    "vimeo.com",
    "player.vimeo.com",
    // Wikimedia Commons — free media files
    "commons.wikimedia.org",
    "upload.wikimedia.org",
    // PeerTube — open-source federated video
    "joinpeertube.org",
    "video.blender.org",  // Blender Foundation CC films
    // Blender open movies (CC BY)
    "download.blender.org",
];

/// Minimum sane file size (10 KB) — smaller files are likely stubs/traps.
const MIN_FILE_SIZE: u64 = 10 * 1024;

/// Maximum sane file size (10 GB).
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024 * 1024;

// ─────────────────────────────────────────────────────────────────────────────
// Security tier
// ─────────────────────────────────────────────────────────────────────────────

/// The security classification of a URL.
#[derive(Debug, PartialEq)]
pub enum SecurityTier {
    /// Recognised free/open-source domain — still fully validated.
    Trusted,
    /// HTTPS URL that passed all pattern checks but is not in the trusted list.
    Allowed,
}

// ─────────────────────────────────────────────────────────────────────────────
// Guard
// ─────────────────────────────────────────────────────────────────────────────

pub struct DownloadGuard;

impl DownloadGuard {
    // -----------------------------------------------------------------------
    // URL Validation
    // -----------------------------------------------------------------------

    /// Validate a URL before downloading. Returns `Ok(tier)` if safe.
    pub fn validate_url(url: &str) -> Result<SecurityTier, String> {
        let url_lower = url.to_lowercase();

        // 1. Protocol check — must be HTTPS, localhost, or yt-dlp search protocol
        if !url_lower.starts_with("https://") && !url_lower.starts_with("http://localhost") {
            if !url_lower.starts_with("ytsearch") {
                warn!("[GUARD] 🛡️ Blocked non-HTTPS URL: {}", url);
                return Err(format!("Unsafe protocol — only HTTPS allowed: {}", url));
            }
        }

        // 2. Block injection URIs unconditionally
        if url_lower.starts_with("data:") || url_lower.starts_with("javascript:") {
            return Err("Blocked injection URI scheme".to_string());
        }

        // 3. Blocked content patterns (executables, malware keywords)
        for pattern in BLOCKED_URL_PATTERNS {
            if url_lower.contains(pattern) {
                warn!("[GUARD] 🛡️ Blocked suspicious URL pattern '{}': {}", pattern, url);
                return Err(format!(
                    "URL contains blocked pattern '{}' — possible malware",
                    pattern
                ));
            }
        }

        // 4. Domain trust classification
        let tier = if Self::is_trusted_domain(&url_lower) {
            info!("[GUARD] ✅ Trusted domain URL: {}", url);
            SecurityTier::Trusted
        } else {
            info!("[GUARD] ✅ Allowed URL (untrusted domain, passed checks): {}", url);
            SecurityTier::Allowed
        };

        Ok(tier)
    }

    /// Check whether the URL's host belongs to a trusted free-video domain.
    pub fn is_trusted_domain(url_lower: &str) -> bool {
        TRUSTED_VIDEO_DOMAINS
            .iter()
            .any(|domain| url_lower.contains(domain))
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
            warn!("[GUARD] 🛡️ Blocked unsafe file extension '{}': {:?}", ext, path);
            return Err(format!(
                "Unsafe file extension '{}' — only media files allowed",
                ext
            ));
        }

        // 3. File size bounds
        let metadata =
            fs::metadata(path).map_err(|e| format!("Cannot read file metadata: {}", e))?;

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

        // 5. Container signature check — verify the file is actually a valid media container
        Self::check_container_signature(path, &ext)?;

        info!(
            "[GUARD] ✅ File passed full security check: {:?} ({} bytes)",
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

        // Java class file (0xCAFEBABE)
        if bytes_read >= 4
            && header[0] == 0xCA
            && header[1] == 0xFE
            && header[2] == 0xBA
            && header[3] == 0xBE
        {
            return Err("File contains Java class bytecode — BLOCKED".to_string());
        }

        Ok(())
    }

    /// Verify that the file matches the expected media-container format.
    /// This catches files that pass the extension check but aren't real video files
    /// (e.g., a plain-text script renamed to .mp4).
    fn check_container_signature(path: &Path, ext: &str) -> Result<(), String> {
        let mut file = File::open(path)
            .map_err(|e| format!("Cannot open file for container check: {}", e))?;

        // Read first 12 bytes — enough for MP4 ftyp, RIFF/AVI, EBML/WebM/MKV
        let mut header = [0u8; 12];
        let n = file.read(&mut header).unwrap_or(0);

        if n < 4 {
            return Ok(()); // Too short to check — size guard already caught truly tiny files
        }

        match ext {
            ".mp4" | ".mov" | ".m4v" => {
                // MP4 / QuickTime: bytes 4-7 must contain "ftyp" or "moov" or "free" or "mdat"
                // (the first 4 bytes are the box size, big-endian u32)
                let valid = n >= 8
                    && matches!(
                        &header[4..8],
                        b"ftyp" | b"moov" | b"free" | b"mdat" | b"wide" | b"pnot"
                    );
                if !valid {
                    warn!("[GUARD] 🛡️ File claims to be MP4/MOV but has invalid container: {:?}", path);
                    return Err(
                        "File extension is .mp4/.mov but content is not a valid MP4 container"
                            .to_string(),
                    );
                }
            }
            ".webm" | ".mkv" => {
                // EBML header: 0x1A 0x45 0xDF 0xA3
                if !(header[0] == 0x1A
                    && header[1] == 0x45
                    && header[2] == 0xDF
                    && header[3] == 0xA3)
                {
                    warn!("[GUARD] 🛡️ File claims to be WebM/MKV but has invalid container: {:?}", path);
                    return Err(
                        "File extension is .webm/.mkv but content is not a valid EBML container"
                            .to_string(),
                    );
                }
            }
            ".avi" => {
                // RIFF header: bytes 0-3 = "RIFF", bytes 8-11 = "AVI "
                let valid = n >= 12
                    && &header[0..4] == b"RIFF"
                    && &header[8..12] == b"AVI ";
                if !valid {
                    warn!("[GUARD] 🛡️ File claims to be AVI but has invalid container: {:?}", path);
                    return Err(
                        "File extension is .avi but content is not a valid RIFF/AVI container"
                            .to_string(),
                    );
                }
            }
            // Audio and other formats: magic-byte check above is sufficient
            _ => {}
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
        assert!(DownloadGuard::validate_url("https://www.youtube.com/watch?v=abc123").is_ok());
    }

    #[test]
    fn test_trusted_tier_for_youtube() {
        let result = DownloadGuard::validate_url("https://www.youtube.com/watch?v=abc123");
        assert_eq!(result.unwrap(), SecurityTier::Trusted);
    }

    #[test]
    fn test_trusted_tier_for_pexels() {
        let result = DownloadGuard::validate_url("https://videos.pexels.com/video-files/abc.mp4");
        assert_eq!(result.unwrap(), SecurityTier::Trusted);
    }

    #[test]
    fn test_trusted_tier_for_archive_org() {
        let result =
            DownloadGuard::validate_url("https://archive.org/download/some-film/film.mp4");
        assert_eq!(result.unwrap(), SecurityTier::Trusted);
    }

    #[test]
    fn test_allowed_tier_for_unknown_domain() {
        let result = DownloadGuard::validate_url("https://someunknownhost.com/video.mp4");
        assert_eq!(result.unwrap(), SecurityTier::Allowed);
    }

    #[test]
    fn test_block_http_url() {
        assert!(DownloadGuard::validate_url("http://evil-site.com/video.mp4").is_err());
    }

    #[test]
    fn test_allow_localhost() {
        assert!(DownloadGuard::validate_url("http://localhost:3000/api").is_ok());
    }

    #[test]
    fn test_block_executable_url() {
        let result = DownloadGuard::validate_url("https://example.com/download.exe");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(".exe"));
    }

    #[test]
    fn test_block_malware_keyword_url() {
        assert!(
            DownloadGuard::validate_url("https://crack-site.com/keygen-video.mp4").is_err()
        );
    }

    #[test]
    fn test_block_data_uri() {
        assert!(
            DownloadGuard::validate_url("data:text/html,<script>alert(1)</script>").is_err()
        );
    }

    #[test]
    fn test_block_javascript_uri() {
        assert!(DownloadGuard::validate_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn test_validate_nonexistent_file() {
        assert!(
            DownloadGuard::validate_downloaded_file(Path::new("__nonexistent_xyz_test.mp4"))
                .is_err()
        );
    }

    #[test]
    fn test_block_executable_bytes() {
        let dir = std::env::temp_dir().join("synoid_guard_test");
        let _ = fs::create_dir_all(&dir);
        let fake_exe = dir.join("sneaky.mp4");

        let mut f = File::create(&fake_exe).unwrap();
        f.write_all(b"MZ").unwrap();
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
