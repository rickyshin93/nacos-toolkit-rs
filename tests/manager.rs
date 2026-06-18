//! Manager logic tests with a mock [`ConfigSource`], ported from the Python
//! `test_manager.py` (which patches `_create_config_service`).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use nacos_toolkit::error::ConfigError;
use nacos_toolkit::{ConfigRef, ConfigSource, ListenerCallback, NacosConfigManager};

struct MockSource {
    responses: Mutex<VecDeque<String>>,
    get_calls: AtomicUsize,
    listener_calls: AtomicUsize,
}

impl MockSource {
    fn new(responses: &[&str]) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.iter().map(|s| s.to_string()).collect()),
            get_calls: AtomicUsize::new(0),
            listener_calls: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl ConfigSource for MockSource {
    async fn get_config(&self, _data_id: &str, _group: &str) -> Result<String, ConfigError> {
        self.get_calls.fetch_add(1, Ordering::SeqCst);
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| ConfigError::Nacos("no more mock responses".into()))
    }

    async fn add_listener(
        &self,
        _data_id: &str,
        _group: &str,
        _callback: ListenerCallback,
    ) -> Result<(), ConfigError> {
        self.listener_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn cfg(data_id: &str) -> ConfigRef {
    ConfigRef::new(data_id, "DEFAULT_GROUP")
}

#[tokio::test]
async fn fetches_and_processes_config() {
    let mock = MockSource::new(&[
        "db_host: mysql-server\ndb_port: \"3306\"",
        "env: dev1",
        "host: ${db_host}\nport: 8080",
    ]);
    let mgr = NacosConfigManager::new(mock, "dev1");
    let base = [cfg("common.yml"), cfg("env.yml"), cfg("app.yml")];

    let result = mgr.get_nacos_config(&base, None, false).await.unwrap();
    assert_eq!(result.config["host"], serde_json::json!("mysql-server"));
    assert_eq!(result.config["port"], serde_json::json!(8080));
}

#[tokio::test]
async fn debug_mode_returns_raw() {
    let mock = MockSource::new(&["key: value", "name: app"]);
    let mgr = NacosConfigManager::new(mock, "dev");
    let base = [cfg("common.yml"), cfg("app.yml")];

    let result = mgr.get_nacos_config(&base, None, true).await.unwrap();
    assert!(result.raw.is_some());
}

#[tokio::test]
async fn cached_config_returned_on_second_call() {
    let mock = MockSource::new(&["name: app"]);
    let mgr = NacosConfigManager::new(mock.clone(), "dev");
    let base = [cfg("app.yml")];

    let r1 = mgr.get_config(&base, None).await.unwrap();
    let r2 = mgr.get_config(&base, None).await.unwrap();
    assert_eq!(r1, r2);
    assert_eq!(mock.get_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn override_config_merges() {
    let mock = MockSource::new(&["host: localhost\nport: 3000", "port: 9999\nextra: yes"]);
    let mgr = NacosConfigManager::new(mock, "dev");
    let base = [cfg("app.yml")];
    let ovr = cfg("override.yml");

    let result = mgr
        .get_nacos_config(&base, Some(&ovr), false)
        .await
        .unwrap();
    assert_eq!(result.config["host"], serde_json::json!("localhost"));
    assert_eq!(result.config["port"], serde_json::json!(9999));
    assert_eq!(result.config["extra"], serde_json::json!("yes"));
}

#[tokio::test]
async fn deploy_env_injected() {
    let mock = MockSource::new(&["env: ${DEPLOY_ENV}"]);
    let mgr = NacosConfigManager::new(mock, "production");
    let base = [cfg("app.yml")];

    let result = mgr.get_nacos_config(&base, None, false).await.unwrap();
    assert_eq!(result.config["env"], serde_json::json!("production"));
}

#[tokio::test]
async fn clear_cache_resets() {
    let mock = MockSource::new(&["name: app", "name: app2"]);
    let mgr = NacosConfigManager::new(mock, "dev");
    let base = [cfg("app.yml")];

    let r1 = mgr.get_config(&base, None).await.unwrap();
    assert_eq!(r1["name"], serde_json::json!("app"));
    mgr.clear_cache();
    assert!(mgr.get_raw_config().is_none());
    let r2 = mgr.get_config(&base, None).await.unwrap();
    assert_eq!(r2["name"], serde_json::json!("app2"));
}

#[tokio::test]
async fn subscribes_to_configs() {
    let mock = MockSource::new(&[]);
    let mgr = NacosConfigManager::new(mock.clone(), "dev");
    let requests = [cfg("app.yml")];

    mgr.setup_listener(&requests, None).await.unwrap();
    assert_eq!(mock.listener_calls.load(Ordering::SeqCst), 1);
}
