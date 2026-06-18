#![doc = include_str!("../README.md")]

pub mod client;
pub mod error;
pub mod local_config;
pub mod manager;
pub mod merger;
mod nested;
pub mod parser;
pub mod template;
pub mod utils;

pub use client::{
    get_nacos_config, reset_global_manager, setup_config_listener, NacosRustClientSource,
};
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
