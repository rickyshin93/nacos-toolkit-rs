//! Error type for the toolkit.

/// Errors produced by config file parsing and the Nacos client layer.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// File extension is not one of `.json`, `.yaml`, `.yml`.
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Yaml(#[from] serde_norway::Error),

    /// Error surfaced from the underlying Nacos client.
    #[error("nacos error: {0}")]
    Nacos(String),
}
