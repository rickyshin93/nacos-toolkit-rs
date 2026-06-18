//! # nacos-toolkit
//!
//! Rust port of the Python `nacos-toolkit`: Nacos configuration parsing and
//! management — fetch configs from Nacos, render `${VAR}` templates,
//! deep-merge multiple configs, and read local config files.
//!
//! Dynamic config values are represented as [`serde_json::Value`] (mirroring
//! Python's `dict[str, Any]`).

pub mod client;
pub mod error;
pub mod local_config;
pub mod manager;
pub mod merger;
mod nested;
pub mod parser;
pub mod template;
pub mod utils;

pub use client::{get_nacos_config, setup_config_listener, NacosRustClientSource};
pub use error::ConfigError;
pub use local_config::{find_local_config, get_local_config, parse_config_file};
pub use manager::{
    ConfigRef, ConfigSource, ListenerCallback, NacosConfigManager, NacosConfigResult,
    NacosConnection,
};
pub use merger::ConfigMerger;
pub use parser::{ConfigParser, NacosParser, RawConfig};
pub use template::TemplateEngine;
pub use utils::NacosConfigUtils;
