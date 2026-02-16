use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use directories::ProjectDirs;

use crate::error::{Result, RusdocError};
use crate::source::DocSource;

const CACHE_TTL_SECS: u64 = 7 * 24 * 60 * 60; // 1 week

/// Manages on-disk caching of raw JSON doc files to avoid re-fetching.
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new() -> Result<Self> {
        let dirs = ProjectDirs::from("dev", "rusdoc", "rusdoc")
            .ok_or_else(|| RusdocError::CacheDir("cannot determine cache directory".into()))?;

        let root = dirs.cache_dir().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Cache { root })
    }

    /// Return the cached JSON path for a source, if it exists and is fresh.
    pub fn get(&self, source: &DocSource) -> Option<PathBuf> {
        let path = self.path_for(source);
        if !path.exists() {
            return None;
        }

        if let Ok(meta) = fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                let age = SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or(Duration::ZERO);
                if age < Duration::from_secs(CACHE_TTL_SECS) {
                    return Some(path);
                }
            }
        }

        None
    }

    /// Store raw JSON bytes in the cache for a given source.
    pub fn put(&self, source: &DocSource, json_bytes: &[u8]) -> Result<PathBuf> {
        let path = self.path_for(source);
        fs::write(&path, json_bytes)?;
        Ok(path)
    }

    /// Remove a single cached entry.
    pub fn evict(&self, source: &DocSource) -> Result<()> {
        let path = self.path_for(source);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Clear all cached documentation.
    pub fn clear(&self) -> Result<()> {
        if self.root.exists() {
            fs::remove_dir_all(&self.root)?;
            fs::create_dir_all(&self.root)?;
        }
        Ok(())
    }

    /// Report cache location and total size.
    pub fn info(&self) -> Result<CacheInfo> {
        let mut total_bytes: u64 = 0;
        let mut file_count: usize = 0;

        if self.root.exists() {
            for entry in fs::read_dir(&self.root)? {
                let entry = entry?;
                if let Ok(meta) = entry.metadata() {
                    total_bytes += meta.len();
                    file_count += 1;
                }
            }
        }

        Ok(CacheInfo {
            path: self.root.clone(),
            total_bytes,
            file_count,
        })
    }

    fn path_for(&self, source: &DocSource) -> PathBuf {
        self.root.join(format!("{}.json", source.cache_key()))
    }
}

pub struct CacheInfo {
    pub path: PathBuf,
    pub total_bytes: u64,
    pub file_count: usize,
}

impl std::fmt::Display for CacheInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size = if self.total_bytes > 1_048_576 {
            format!("{:.1} MB", self.total_bytes as f64 / 1_048_576.0)
        } else if self.total_bytes > 1024 {
            format!("{:.1} KB", self.total_bytes as f64 / 1024.0)
        } else {
            format!("{} B", self.total_bytes)
        };

        write!(
            f,
            "Cache: {}\nEntries: {}\nSize: {}",
            self.path.display(),
            self.file_count,
            size,
        )
    }
}
