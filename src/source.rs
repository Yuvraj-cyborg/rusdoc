use std::path::PathBuf;
use std::process::Command;

use crate::doc::DocIndex;
use crate::error::{Result, RusdocError};

/// Where documentation comes from.
pub enum DocSource {
    /// A published crate on docs.rs
    Remote { name: String, version: String },
    /// A local JSON file (already generated)
    LocalFile { path: PathBuf },
    /// The current project (we'll run cargo rustdoc)
    LocalProject { manifest_dir: PathBuf },
}

impl DocSource {
    /// Determine the source from a query path like "std::vec::Vec" or "tokio::spawn".
    pub fn from_query(query: &str, local: bool) -> Self {
        if local {
            let manifest_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            return DocSource::LocalProject { manifest_dir };
        }

        let crate_name = query
            .split("::")
            .next()
            .unwrap_or(query)
            .to_lowercase();

        DocSource::Remote {
            name: crate_name,
            version: "latest".to_string(),
        }
    }

    /// The cache key for this source (used for cache file naming).
    pub fn cache_key(&self) -> String {
        match self {
            DocSource::Remote { name, version } => format!("{name}-{version}"),
            DocSource::LocalFile { path } => path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("local")
                .to_string(),
            DocSource::LocalProject { manifest_dir } => manifest_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project")
                .to_string(),
        }
    }

    /// Fetch, parse, and index the documentation.
    pub fn load(&self) -> Result<DocIndex> {
        let json_bytes = match self {
            DocSource::Remote { name, version } => fetch_remote(name, version)?,
            DocSource::LocalFile { path } => std::fs::read(path)?,
            DocSource::LocalProject { manifest_dir } => load_local_project(manifest_dir)?,
        };

        let krate: rustdoc_types::Crate =
            serde_json::from_slice(&json_bytes).map_err(|e| match self {
                DocSource::Remote { name, .. } => RusdocError::FetchFailed {
                    name: name.clone(),
                    reason: format!("JSON parse error: {e}"),
                },
                _ => RusdocError::Json(e),
            })?;

        Ok(DocIndex::from_crate(krate))
    }
}

/// Download compressed JSON from docs.rs and decompress it.
fn fetch_remote(name: &str, version: &str) -> Result<Vec<u8>> {
    let url = format!("https://docs.rs/crate/{name}/{version}/json.zst");

    let mut response = ureq::get(&url)
        .call()
        .map_err(|e| RusdocError::FetchFailed {
            name: name.to_string(),
            reason: e.to_string(),
        })?;

    let compressed = response
        .body_mut()
        .read_to_vec()
        .map_err(|e| RusdocError::FetchFailed {
            name: name.to_string(),
            reason: format!("download error: {e}"),
        })?;

    let decompressed = zstd::stream::decode_all(compressed.as_slice()).map_err(|e| {
        RusdocError::FetchFailed {
            name: name.to_string(),
            reason: format!("decompression error: {e}"),
        }
    })?;

    Ok(decompressed)
}

fn load_local_project(manifest_dir: &PathBuf) -> Result<Vec<u8>> {
    let output = Command::new("cargo")
        .args([
            "+nightly",
            "rustdoc",
            "--lib",
            "--",
            "-Z",
            "unstable-options",
            "--output-format",
            "json",
        ])
        .current_dir(manifest_dir)
        .output()
        .map_err(|e| RusdocError::LocalDocgen {
            reason: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RusdocError::LocalDocgen {
            reason: stderr.to_string(),
        });
    }

    let doc_dir = manifest_dir.join("target/doc");
    let json_file = find_json_in_dir(&doc_dir)?;
    let bytes = std::fs::read(&json_file)?;
    Ok(bytes)
}

fn find_json_in_dir(dir: &PathBuf) -> Result<PathBuf> {
    let entries = std::fs::read_dir(dir).map_err(|e| RusdocError::LocalDocgen {
        reason: format!("cannot read {}: {}", dir.display(), e),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| RusdocError::LocalDocgen {
            reason: e.to_string(),
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            return Ok(path);
        }
    }

    Err(RusdocError::LocalDocgen {
        reason: format!("no .json file found in {}", dir.display()),
    })
}
