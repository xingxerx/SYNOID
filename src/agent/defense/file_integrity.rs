use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use walkdir::WalkDir;
use tracing::{info, warn};

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
    pub fn build_baseline(&mut self) -> std::io::Result<()> {
        self.hashes.clear();
        info!("[DEFENSE] Building integrity baseline...");
        
        for path in &self.watched_paths {
            if path.is_file() {
                if let Ok(hash) = self.hash_file(path) {
                    self.hashes.insert(path.clone(), hash);
                }
            } else if path.is_dir() {
                for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() {
                        let fpath = entry.path().to_path_buf();
                        if let Ok(hash) = self.hash_file(&fpath) {
                            self.hashes.insert(fpath, hash);
                        }
                    }
                }
            }
        }
        info!("[DEFENSE] Baseline complete. monitoring {} files.", self.hashes.len());
        Ok(())
    }

    /// Check for changes against the baseline
    pub fn verify_integrity(&self) -> Vec<String> {
        let mut violations = Vec::new();
        
        for (path, original_hash) in &self.hashes {
            if !path.exists() {
                let msg = format!("MISSING FILE: {:?}", path);
                warn!("[DEFENSE] ❌ {}", msg);
                violations.push(msg);
                continue;
            }

            match self.hash_file(path) {
                Ok(current_hash) => {
                    if *original_hash != current_hash {
                        let msg = format!("TAMPER DETECTED: {:?} (Hash Mismatch)", path);
                        warn!("[DEFENSE] ⚠️ {}", msg);
                        violations.push(msg);
                    }
                },
                Err(e) => {
                    warn!("[DEFENSE] Could not read file {:?}: {}", path, e);
                }
            }
        }
        
        if violations.is_empty() {
             info!("[DEFENSE] Integrity Check Passed. {} files verified.", self.hashes.len());
        }

        violations
    }

    fn hash_file(&self, path: &Path) -> std::io::Result<String> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 1024];

        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }
}
