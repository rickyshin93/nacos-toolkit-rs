//! Local config file discovery and parsing. Port of the Python
//! `local_config` module.

use std::path::Path;

use serde_json::Value;

use crate::error::ConfigError;

/// Extension search priority, matching the Python implementation.
const CONFIG_EXTENSIONS: [&str; 3] = [".json", ".yaml", ".yml"];

/// Find a config file by base name in `file_path`, trying extensions in
/// priority order (`.json` -> `.yaml` -> `.yml`). Returns the resolved
/// absolute path, or `None` if none exist.
pub fn find_local_config(file_name: &str, file_path: &str) -> Option<String> {
    let dir = std::fs::canonicalize(file_path).ok()?;
    for ext in CONFIG_EXTENSIONS {
        let candidate = dir.join(format!("{file_name}{ext}"));
        if candidate.exists() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

/// Parse a specific config file by extension (`.json`, `.yaml`, `.yml`).
pub fn parse_config_file(file_path: &str) -> Result<Value, ConfigError> {
    let path = Path::new(file_path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    let content = std::fs::read_to_string(path)?;
    match ext.as_str() {
        "json" => Ok(serde_json::from_str(&content)?),
        "yaml" | "yml" => Ok(serde_norway::from_str(&content)?),
        _ => Err(ConfigError::UnsupportedFormat(format!(".{ext}"))),
    }
}

/// Auto-discover (by priority) and parse a local config. Returns `None` when
/// `file_name` is empty or no matching file is found.
pub fn get_local_config(file_name: &str, file_path: &str) -> Option<Value> {
    if file_name.is_empty() {
        return None;
    }
    match find_local_config(file_name, file_path) {
        Some(path) => parse_config_file(&path).ok(),
        None => {
            tracing::warn!("No local configuration found for {file_name}");
            None
        }
    }
}
