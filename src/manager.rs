//! Nacos config manager: fetch, cache, process, and listen. Port of the Python
//! `NacosConfigManager` + `get_nacos_config` / `setup_config_listener`.
//!
//! The actual Nacos transport is abstracted behind [`ConfigSource`] so the
//! manager logic is fully unit-testable with a mock (mirroring the Python tests
//! that patch `_create_config_service`). The real `nacos_rust_client`-backed
//! source lives in [`crate::client`].

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Map, Value};

use crate::error::ConfigError;
use crate::parser::{ConfigParser, NacosParser};
use crate::utils::NacosConfigUtils;

/// Connection parameters for a Nacos server.
#[derive(Clone, Debug)]
pub struct NacosConnection {
    pub server_addr: String,
    pub namespace: String,
    pub username: String,
    pub password: String,
    /// Transport for config fetch. `true` = gRPC (Nacos 2.x default, port
    /// `+1000`); `false` = HTTP over `server_addr`. Use HTTP when the gRPC
    /// port is firewalled/unreachable.
    pub use_grpc: bool,
}

impl Default for NacosConnection {
    fn default() -> Self {
        Self {
            server_addr: String::new(),
            namespace: String::new(),
            username: String::new(),
            password: String::new(),
            use_grpc: true,
        }
    }
}

/// A reference to one config entry (`data_id` within a `group`).
#[derive(Clone, Debug)]
pub struct ConfigRef {
    pub data_id: String,
    pub group: String,
}

impl ConfigRef {
    pub fn new(data_id: impl Into<String>, group: impl Into<String>) -> Self {
        Self {
            data_id: data_id.into(),
            group: group.into(),
        }
    }
}

/// Callback invoked on a config change with the new raw content.
pub type ListenerCallback = Arc<dyn Fn(String) + Send + Sync>;

/// Result of [`NacosConfigManager::get_nacos_config`]. `raw` is populated only
/// when `debug` is requested.
#[derive(Clone, Debug)]
pub struct NacosConfigResult {
    pub config: Value,
    pub raw: Option<Value>,
}

/// Transport abstraction over a Nacos config service.
#[async_trait]
pub trait ConfigSource: Send + Sync {
    /// Fetch the raw content of one config entry.
    async fn get_config(&self, data_id: &str, group: &str) -> Result<String, ConfigError>;

    /// Register a change listener for one config entry.
    async fn add_listener(
        &self,
        data_id: &str,
        group: &str,
        callback: ListenerCallback,
    ) -> Result<(), ConfigError>;
}

/// Shallow-merge top-level keys of `src` into `dst`.
fn shallow_extend(dst: &mut Value, src: &Value) {
    if let (Some(d), Some(s)) = (dst.as_object_mut(), src.as_object()) {
        for (k, v) in s {
            d.insert(k.clone(), v.clone());
        }
    }
}

fn determine_format(data_id: &str) -> NacosParser {
    if data_id.ends_with(".json") {
        NacosParser::Json
    } else {
        NacosParser::Yaml
    }
}

/// Fetches, caches and processes Nacos configuration.
pub struct NacosConfigManager {
    source: Arc<dyn ConfigSource>,
    namespace: String,
    config_cache: Arc<Mutex<Option<Value>>>,
    raw_config: Arc<Mutex<Option<Value>>>,
}

impl NacosConfigManager {
    /// Build a manager over an arbitrary [`ConfigSource`]. `namespace` is used
    /// to inject `DEPLOY_ENV`.
    pub fn new(source: Arc<dyn ConfigSource>, namespace: impl Into<String>) -> Self {
        Self {
            source,
            namespace: namespace.into(),
            config_cache: Arc::new(Mutex::new(None)),
            raw_config: Arc::new(Mutex::new(None)),
        }
    }

    /// Fetch and process configuration. Subsequent calls return the cache until
    /// [`clear_cache`](Self::clear_cache) is called.
    ///
    /// # Warning
    ///
    /// The cache is keyed on **nothing**: the first successful call wins, and
    /// later calls return that cached value while **ignoring `base_configs` and
    /// `override_config`**. To fetch a different config set, call
    /// [`clear_cache`](Self::clear_cache) first (this mirrors the Python
    /// implementation's module-level cache).
    ///
    /// Behaviour mirrors the Python implementation:
    /// 1. fetch all `base_configs` in order
    /// 2. shallow-merge them into a variable context (`all_data`, last wins)
    /// 3. process only the LAST config, rendering `${VAR}` against
    ///    `all_data` + `DEPLOY_ENV = namespace`
    /// 4. optionally deep-merge an `override_config` on top
    pub async fn get_config(
        &self,
        base_configs: &[ConfigRef],
        override_config: Option<&ConfigRef>,
    ) -> Result<Value, ConfigError> {
        {
            let guard = self.config_cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(cached) = guard.as_ref() {
                return Ok(cached.clone());
            }
        }

        let mut contents: Vec<String> = Vec::with_capacity(base_configs.len());
        for cfg in base_configs {
            contents.push(self.source.get_config(&cfg.data_id, &cfg.group).await?);
        }

        let mut all_data = Value::Object(Map::new());
        for content in &contents {
            let parsed = ConfigParser::parse(content.as_str(), NacosParser::Yaml);
            shallow_extend(&mut all_data, &parsed);
        }

        let last_content = contents.last().cloned().unwrap_or_default();
        let last_config = ConfigParser::parse(last_content.as_str(), NacosParser::Yaml);

        *self.raw_config.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(all_data.clone());

        // external_vars = {**all_data, "DEPLOY_ENV": namespace}
        let mut external_vars = all_data.clone();
        if let Some(m) = external_vars.as_object_mut() {
            m.insert(
                "DEPLOY_ENV".to_string(),
                Value::String(self.namespace.clone()),
            );
        }

        let mut config = NacosConfigUtils::process_configuration(
            last_config,
            NacosParser::Json,
            Some(&external_vars),
            None,
        );

        if let Some(ovr) = override_config {
            if !ovr.data_id.is_empty() {
                let custom = self.source.get_config(&ovr.data_id, &ovr.group).await?;
                let fmt = determine_format(&ovr.data_id);
                config = NacosConfigUtils::process_and_merge_custom_config(
                    &config,
                    custom.as_str(),
                    fmt,
                    Some(&external_vars),
                    None,
                );
            }
        }

        *self.config_cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(config.clone());
        Ok(config)
    }

    /// Convenience wrapper returning `{config, raw?}`. `debug` populates `raw`.
    pub async fn get_nacos_config(
        &self,
        base_configs: &[ConfigRef],
        override_config: Option<&ConfigRef>,
        debug: bool,
    ) -> Result<NacosConfigResult, ConfigError> {
        let config = self.get_config(base_configs, override_config).await?;
        let raw = if debug { self.get_raw_config() } else { None };
        Ok(NacosConfigResult { config, raw })
    }

    /// Subscribe to config changes. With no `callback`, the default handler
    /// parses the new content as YAML and shallow-updates the cache.
    pub async fn setup_listener(
        &self,
        listen_requests: &[ConfigRef],
        callback: Option<ListenerCallback>,
    ) -> Result<(), ConfigError> {
        let cb: ListenerCallback = match callback {
            Some(cb) => cb,
            None => {
                let cache = self.config_cache.clone();
                Arc::new(move |content: String| {
                    let parsed = ConfigParser::parse(content.as_str(), NacosParser::Yaml);
                    if let Some(src) = parsed.as_object() {
                        let mut guard = cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                        if let Some(Value::Object(m)) = guard.as_mut() {
                            for (k, v) in src {
                                m.insert(k.clone(), v.clone());
                            }
                        }
                    }
                })
            }
        };

        for req in listen_requests {
            self.source
                .add_listener(&req.data_id, &req.group, cb.clone())
                .await?;
        }
        Ok(())
    }

    /// Clear the processed and raw caches.
    pub fn clear_cache(&self) {
        *self.config_cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        *self.raw_config.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    /// The merged raw variable context from the last fetch, if any.
    pub fn get_raw_config(&self) -> Option<Value> {
        self.raw_config.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }
}
