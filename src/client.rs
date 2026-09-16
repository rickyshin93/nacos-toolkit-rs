//! Real Nacos transport (plain HTTP, see [`crate::HttpConfigSource`]), plus the
//! module-level `get_nacos_config` / `setup_config_listener` convenience
//! functions (a global singleton manager, matching the Python API).

use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::sync::Mutex as AsyncMutex;

use crate::error::ConfigError;
use crate::http_client::HttpConfigSource;
use crate::manager::{
    ConfigRef, ListenerCallback, NacosConfigManager, NacosConfigResult, NacosConnection,
};

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
    let source = HttpConfigSource::connect(conn);
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
///
/// # Note
///
/// The HTTP transport does not implement change listeners, so this currently
/// returns [`ConfigError::Nacos`] — see [`crate::HttpConfigSource`].
pub async fn setup_config_listener(
    nacos_config: &NacosConnection,
    listen_requests: &[ConfigRef],
    callback: Option<ListenerCallback>,
) -> Result<(), ConfigError> {
    let mgr = global_manager(nacos_config).await;
    mgr.setup_listener(listen_requests, callback).await
}
