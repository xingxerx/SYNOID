use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use walkdir::WalkDir;

/// Monitor changes to critical files by hashing them
pub struct IntegrityGuard {
    watched_paths: Vec<PathBuf>,
    hashes: HashMap<PathBuf, String>,
}

impl IntegrityGuard {
    pub fn new() -> Self {
        Self {
            watched_paths: Vec::new(),
            hashes: HashMap::new(),
        }
    }

    /// Add a directory or file to the watch list
    pub fn watch_path(&mut self, path: PathBuf) {
        if path.exists() {
            self.watched_paths.push(path);
        }
    }

    /// Build the initial database of file hashes
    pub async fn build_baseline(&mut self) -> std::io::Result<()> {
        self.hashes.clear();
        info!("[DEFENSE] Building integrity baseline...");

        // We clone paths to avoid borrowing self in async loop
        let watched = self.watched_paths.clone();

        for path in watched {
            if path.is_file() {
                if let Ok(hash) = self.hash_file(&path).await {
                    self.hashes.insert(path, hash);
                }
            } else if path.is_dir() {
                // Walking directory is blocking, so we collect paths first or wrap in blocking?
                // WalkDir is efficient. Let's collect file paths first.
                let mut files = Vec::new();
                for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
                     if entry.file_type().is_file() {
                        files.push(entry.path().to_path_buf());
                     }
                }

                for fpath in files {
                    if let Ok(hash) = self.hash_file(&fpath).await {
                        self.hashes.insert(fpath, hash);
                    }
                }
            }
        }
        info!(
            "[DEFENSE] Baseline complete. monitoring {} files.",
            self.hashes.len()
        );
        Ok(())
    }

    /// Check for changes against the baseline
    pub async fn verify_integrity(&self) -> Vec<String> {
        let mut violations = Vec::new();

        for (path, original_hash) in &self.hashes {
            if !path.exists() {
                let msg = format!("MISSING FILE: {:?}", path);
                warn!("[DEFENSE] ❌ {}", msg);
                violations.push(msg);
                continue;
            }

            match self.hash_file(path).await {
                Ok(current_hash) => {
                    if *original_hash != current_hash {
                        let msg = format!("TAMPER DETECTED: {:?} (Hash Mismatch)", path);
                        warn!("[DEFENSE] ⚠️ {}", msg);
                        violations.push(msg);
                    }
                }
                Err(e) => {
                    warn!("[DEFENSE] Could not read file {:?}: {}", path, e);
                }
            }
        }

        if violations.is_empty() {
            info!(
                "[DEFENSE] Integrity Check Passed. {} files verified.",
                self.hashes.len()
            );
        }

        violations
    }

    async fn hash_file(&self, path: &Path) -> std::io::Result<String> {
        let path_buf = path.to_path_buf();

        // Offload heavy hashing and I/O to blocking thread
        tokio::task::spawn_blocking(move || {
            let mut file = File::open(&path_buf)?;
            let mut hasher = Sha256::new();
            // Use 64KB buffer for optimal I/O performance
            let mut buffer = [0; 65536];

            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hasher.update(&buffer[..count]);
            }

            Ok(format!("{:x}", hasher.finalize()))
        }).await?
    }
}
