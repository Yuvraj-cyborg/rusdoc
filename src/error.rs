use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RusdocError {
    #[error("item not found: `{query}`")]
    NotFound { query: String },

    #[error("ambiguous query `{query}` matched {count} items — use a more specific path")]
    Ambiguous { query: String, count: usize },

    #[error("crate `{name}` not found on docs.rs")]
    CrateNotFound { name: String },

    #[error("failed to fetch docs for `{name}`: {reason}")]
    FetchFailed { name: String, reason: String },

    #[error("cache directory unavailable: {0}")]
    CacheDir(String),

    #[error("corrupt cache at {path}: {reason}")]
    CacheCorrupt { path: PathBuf, reason: String },

    #[error("failed to generate local docs: {reason}")]
    LocalDocgen { reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, RusdocError>;
