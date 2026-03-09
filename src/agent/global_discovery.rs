// SYNOID Global Discovery - System-wide Media Indexing
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub name: String,
    pub path: PathBuf,
    pub extension: String,
    pub size: u64,
}

pub struct GlobalDiscovery {
    index: Arc<Mutex<Vec<DiscoveredFile>>>,
    pub search_paths: Vec<PathBuf>,
}

impl GlobalDiscovery {
    pub fn new() -> Self {
        let mut search_paths = Vec::new();

        // 1. Add current workspace
        search_paths.push(PathBuf::from("."));

        // 2. Add WSL mount points if roaming in Linux
        #[cfg(target_os = "linux")]
        {
            let mnt_paths = ["/mnt/c/Users", "/mnt/d"];
            for p in mnt_paths {
                let path = PathBuf::from(p);
                if path.exists() {
                    search_paths.push(path);
                }
            }
            // Also include home
            if let Ok(home) = std::env::var("HOME") {
                search_paths.push(PathBuf::from(home));
            }
        }

        // 3. Add Windows common paths if native
        #[cfg(target_os = "windows")]
        {
            if let Ok(user_profile) = std::env::var("USERPROFILE") {
                let p = PathBuf::from(user_profile);
                search_paths.push(p.join("Videos"));
                search_paths.push(p.join("Downloads"));
                search_paths.push(p.join("Documents"));
                search_paths.push(p.join("Desktop"));
                search_paths.push(p.join("Pictures"));
            }
            // Add all common drives
            for drive in b'C'..=b'Z' {
                let path = PathBuf::from(format!("{}:\\", drive as char));
                if path.exists() {
                    // Only add if not already covered by USERPROFILE mappings
                    if !search_paths.iter().any(|p| p.starts_with(&path)) {
                        search_paths.push(path);
                    }
                }
            }
        }

        Self {
            index: Arc::new(Mutex::new(Vec::new())),
            search_paths,
        }
    }

    /// Recursively scan all configured search paths for media files.
    pub async fn scan(&self) -> usize {
        info!("[DISCOVERY] 🔎 Starting global system scan...");
        let mut new_index = Vec::new();
        let extensions = [
            "mp4", "mov", "mkv", "avi", "webm", "wav", "mp3", "jpg", "png", "svg",
        ];

        for root in &self.search_paths {
            info!("[DISCOVERY] Scanning root: {:?}", root);

            // We use a sync walkdir inside a spawned blocking task for performance
            let root_clone = root.clone();
            let ext_list = extensions.to_vec();

            let found = tokio::task::spawn_blocking(move || {
                let mut results = Vec::new();
                for entry in WalkDir::new(&root_clone)
                    .follow_links(true)
                    .max_depth(20) // Deep scan to find all files
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        if let Some(ext) = entry.path().extension() {
                            if let Some(ext_str) = ext.to_str() {
                                if ext_list.contains(&ext_str.to_lowercase().as_str()) {
                                    results.push(DiscoveredFile {
                                        name: entry.file_name().to_string_lossy().to_string(),
                                        path: entry.path().to_path_buf(),
                                        extension: ext_str.to_lowercase(),
                                        size: entry.metadata().map(|m| m.len()).unwrap_or(0),
                                    });
                                }
                            }
                        }
                    }
                }
                results
            })
            .await
            .unwrap_or_default();

            new_index.extend(found);
        }

        let count = new_index.len();
        let mut index = self.index.lock().await;
        *index = new_index;
        info!(
            "[DISCOVERY] ✅ Scan complete. Indexed {} media files.",
            count
        );
        count
    }

    /// Search the index for a file matching the query string.
    pub async fn find(&self, query: &str) -> Vec<DiscoveredFile> {
        let query_lower = query.to_lowercase();
        let index = self.index.lock().await;

        index
            .iter()
            .filter(|f| {
                f.name.to_lowercase().contains(&query_lower)
                    || f.path
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    /// Get all indexed files
    pub async fn get_all(&self) -> Vec<DiscoveredFile> {
        self.index.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discovery_init() {
        let discovery = GlobalDiscovery::new();
        assert!(!discovery.search_paths.is_empty());
        println!("Search Paths: {:?}", discovery.search_paths);
    }

    #[tokio::test]
    async fn test_scan_local() {
        let discovery = GlobalDiscovery::new();
        // Limit paths for fast test
        let count = discovery.scan().await;
        println!("Found {} files", count);
    }
}
