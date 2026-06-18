//! YAML/JSON parsing into `serde_json::Value`. Port of the Python
//! `ConfigParser` + `NacosParser`.

use serde_json::{Map, Value};

/// Config format selector. Values mirror the Python `NacosParser` enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NacosParser {
    Yaml,
    Json,
}

impl NacosParser {
    /// File extension associated with the format (`.yml` / `.json`).
    pub fn extension(self) -> &'static str {
        match self {
            NacosParser::Yaml => ".yml",
            NacosParser::Json => ".json",
        }
    }
}

/// Raw config input: either an unparsed string or an already-parsed value.
///
/// Mirrors Python's `str | dict` argument to `ConfigParser.parse`.
pub enum RawConfig {
    Str(String),
    Value(Value),
}

impl From<&str> for RawConfig {
    fn from(s: &str) -> Self {
        RawConfig::Str(s.to_string())
    }
}

impl From<String> for RawConfig {
    fn from(s: String) -> Self {
        RawConfig::Str(s)
    }
}

impl From<Value> for RawConfig {
    fn from(v: Value) -> Self {
        RawConfig::Value(v)
    }
}

impl From<&Value> for RawConfig {
    fn from(v: &Value) -> Self {
        RawConfig::Value(v.clone())
    }
}

fn empty() -> Value {
    Value::Object(Map::new())
}

/// Zero-sized handle mirroring the Python `ConfigParser` static API.
pub struct ConfigParser;

impl ConfigParser {
    /// Parse `raw` according to `fmt`. On any parse error an empty object is
    /// returned (a warning is logged), matching the Python behaviour.
    ///
    /// - `Json` + string: parsed as JSON (result returned as-is)
    /// - `Yaml` + string: parsed as YAML; non-object results become `{}`
    /// - already-parsed value: returned as-is for `Json`; for `Yaml` returned
    ///   when it is an object, else `{}`
    pub fn parse(raw: impl Into<RawConfig>, fmt: NacosParser) -> Value {
        match raw.into() {
            RawConfig::Value(v) => match fmt {
                NacosParser::Json => v,
                NacosParser::Yaml => {
                    if v.is_object() {
                        v
                    } else {
                        empty()
                    }
                }
            },
            RawConfig::Str(s) => match fmt {
                NacosParser::Json => match serde_json::from_str::<Value>(&s) {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!("Failed to parse config, returning empty dict");
                        empty()
                    }
                },
                NacosParser::Yaml => match serde_norway::from_str::<Value>(&s) {
                    Ok(v) if v.is_object() => v,
                    Ok(_) => empty(),
                    Err(_) => {
                        tracing::warn!("Failed to parse config, returning empty dict");
                        empty()
                    }
                },
            },
        }
    }
}
