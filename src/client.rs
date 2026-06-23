//! Real Nacos transport backed by the `nacos_rust_client` crate, plus the
//! module-level `get_nacos_config` / `setup_config_listener` convenience
//! functions (a global singleton manager, matching the Python API).

use std::sync::Arc;

use async_trait::async_trait;
use once_cell::sync::Lazy;
use tokio::sync::Mutex as AsyncMutex;

use nacos_rust_client::client::config_client::listener::ConfigListener;
use nacos_rust_client::client::config_client::{ConfigClient, ConfigKey};
use nacos_rust_client::client::{AuthInfo, ClientBuilder};

use crate::error::ConfigError;
use crate::manager::{
    ConfigRef, ConfigSource, ListenerCallback, NacosConfigManager, NacosConfigResult,
    NacosConnection,
};

/// A [`ConfigListener`] that forwards changes to a [`ListenerCallback`].
struct CallbackListener {
    key: ConfigKey,
    callback: ListenerCallback,
}

impl ConfigListener for CallbackListener {
    fn get_key(&self) -> ConfigKey {
        self.key.clone()
    }

    fn change(&self, _key: &ConfigKey, value: &str) {
        (self.callback)(value.to_string());
    }
}

/// [`ConfigSource`] implementation backed by `nacos_rust_client`.
pub struct NacosRustClientSource {
    client: Arc<ConfigClient>,
}

impl NacosRustClientSource {
    /// Build a config client for the given connection. Username/password are
    /// passed only when both are present.
    pub fn connect(conn: &NacosConnection) -> Self {
        let auth_info = if conn.username.is_empty() && conn.password.is_empty() {
            None
        } else {
            Some(AuthInfo::new(&conn.username, &conn.password))
        };
        let client = ClientBuilder::new()
            .set_endpoint_addrs(&conn.server_addr)
            .set_auth_info(auth_info)
            .set_tenant(conn.namespace.clone())
            .set_use_grpc(conn.use_grpc)
            .build_config_client();
        Self { client }
    }
}

#[async_trait]
impl ConfigSource for NacosRustClientSource {
    async fn get_config(&self, data_id: &str, group: &str) -> Result<String, ConfigError> {
        let key = self.client.gene_config_key(data_id, group);
        self.client
            .get_config(&key)
            .await
            .map_err(|e| ConfigError::Nacos(e.to_string()))
    }

    async fn add_listener(
        &self,
        data_id: &str,
        group: &str,
        callback: ListenerCallback,
    ) -> Result<(), ConfigError> {
        let key = self.client.gene_config_key(data_id, group);
        let listener = CallbackListener { key, callback };
        self.client
            .subscribe(Box::new(listener))
            .await
            .map_err(|e| ConfigError::Nacos(e.to_string()))
    }
}

/// Global singleton manager (mirrors the Python module-level singleton).
///
/// # Warning
///
/// The singleton is created **once**, on the first call, from the first
/// `connection`. Every later call **ignores its `conn` argument** and returns
/// that same manager — you cannot switch servers/namespaces at runtime through
/// these free functions. Call [`reset_global_manager`] first if you need to
/// rebind, or construct a [`NacosConfigManager`] directly (one per connection)
/// instead of using the global helpers.
static GLOBAL_MANAGER: Lazy<AsyncMutex<Option<Arc<NacosConfigManager>>>> =
    Lazy::new(|| AsyncMutex::new(None));

async fn global_manager(conn: &NacosConnection) -> Arc<NacosConfigManager> {
    let mut guard = GLOBAL_MANAGER.lock().await;
    if let Some(mgr) = guard.as_ref() {
        return mgr.clone();
    }
    let source = NacosRustClientSource::connect(conn);
    let mgr = Arc::new(NacosConfigManager::new(
        Arc::new(source),
        conn.namespace.clone(),
    ));
    *guard = Some(mgr.clone());
    mgr
}

/// Reset the global singleton (mainly for tests / re-initialisation).
pub async fn reset_global_manager() {
    *GLOBAL_MANAGER.lock().await = None;
}

/// Fetch and process configuration from Nacos using the global singleton.
///
/// Equivalent to the Python top-level `get_nacos_config`.
///
/// # Warning
///
/// `connection` is only honoured on the **first** call (it builds the global
/// singleton); later calls reuse that manager and ignore `connection`. The
/// manager also caches the **first** result and ignores `base_configs` /
/// `override_config` on later calls until [`reset_global_manager`] is invoked.
/// For multiple connections or changing config sets, build separate
/// [`NacosConfigManager`] instances instead.
pub async fn get_nacos_config(
    connection: &NacosConnection,
    base_configs: &[ConfigRef],
    override_config: Option<&ConfigRef>,
    debug: bool,
) -> Result<NacosConfigResult, ConfigError> {
    let mgr = global_manager(connection).await;
    mgr.get_nacos_config(base_configs, override_config, debug)
        .await
}

/// Subscribe to Nacos config changes using the global singleton.
///
/// Equivalent to the Python top-level `setup_config_listener`.
pub async fn setup_config_listener(
    nacos_config: &NacosConnection,
    listen_requests: &[ConfigRef],
    callback: Option<ListenerCallback>,
) -> Result<(), ConfigError> {
    let mgr = global_manager(nacos_config).await;
    mgr.setup_listener(listen_requests, callback).await
}
