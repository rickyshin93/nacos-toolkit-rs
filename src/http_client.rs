//! Nacos 配置传输层：纯 HTTP 实现（Nacos OpenAPI v1）。
//!
//! 取代原先基于 `nacos_rust_client`（gRPC / tonic）的实现。拉配置本来就是
//! 一次 HTTP GET；gRPC 只在配置变更推送时才需要，而本 toolkit 的调用方
//! 目前都不使用监听，因此整条 tonic / axum 依赖链可以去掉。

use std::time::Duration;

use async_trait::async_trait;

use crate::error::ConfigError;
use crate::manager::{ConfigSource, ListenerCallback, NacosConnection};

/// 百分号编码：RFC 3986 unreserved（`A-Za-z0-9-._~`）之外的字节全部转义。
fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// 构造拉取单个配置的 URL。
///
/// `tenant` 为空表示 public 命名空间，不带该参数；`token` 为 `None` 表示
/// Nacos 未开启鉴权，不带 `accessToken`。
pub(crate) fn build_config_url(
    server_addr: &str,
    data_id: &str,
    group: &str,
    tenant: &str,
    token: Option<&str>,
) -> String {
    let mut url = format!(
        "http://{}/nacos/v1/cs/configs?group={}&dataId={}",
        server_addr,
        encode_component(group),
        encode_component(data_id)
    );
    if !tenant.is_empty() {
        url.push_str("&tenant=");
        url.push_str(&encode_component(tenant));
    }
    if let Some(t) = token {
        url.push_str("&accessToken=");
        url.push_str(&encode_component(t));
    }
    url
}

/// 从登录响应体里取出 `accessToken`。
pub(crate) fn parse_login_token(body: &str) -> Result<String, ConfigError> {
    let value: serde_json::Value = serde_json::from_str(body)?;
    value
        .get("accessToken")
        .and_then(|t| t.as_str())
        .map(str::to_string)
        .ok_or_else(|| ConfigError::Nacos("login response has no accessToken".to_string()))
}

/// [`ConfigSource`] 的纯 HTTP 实现。
///
/// 鉴权是惰性的：只有同时配了用户名和密码时才在首次拉取前请求
/// `accessToken`，之后复用；未配凭据则完全不带该参数（适用于未开启鉴权
/// 的 Nacos）。
///
/// `NacosConnection::use_grpc` 被忽略 —— 本实现只走 HTTP。
pub struct HttpConfigSource {
    server_addr: String,
    tenant: String,
    username: String,
    password: String,
    client: reqwest::Client,
    token: tokio::sync::Mutex<Option<String>>,
}

impl HttpConfigSource {
    pub fn connect(conn: &NacosConnection) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client");
        Self {
            server_addr: conn.server_addr.clone(),
            tenant: conn.namespace.clone(),
            username: conn.username.clone(),
            password: conn.password.clone(),
            client,
            token: tokio::sync::Mutex::new(None),
        }
    }

    fn needs_auth(&self) -> bool {
        !(self.username.is_empty() && self.password.is_empty())
    }

    /// 可用的 `accessToken`；未配凭据时为 `None`。
    async fn access_token(&self) -> Result<Option<String>, ConfigError> {
        if !self.needs_auth() {
            return Ok(None);
        }
        let mut guard = self.token.lock().await;
        if let Some(cached) = guard.as_ref() {
            return Ok(Some(cached.clone()));
        }
        let url = format!("http://{}/nacos/v1/auth/login", self.server_addr);
        let body = format!(
            "username={}&password={}",
            encode_component(&self.username),
            encode_component(&self.password)
        );
        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| ConfigError::Nacos(format!("nacos login request failed: {e}")))?;
        if !resp.status().is_success() {
            return Err(ConfigError::Nacos(format!(
                "nacos login failed: HTTP {}",
                resp.status()
            )));
        }
        let body = resp
            .text()
            .await
            .map_err(|e| ConfigError::Nacos(format!("nacos login body: {e}")))?;
        let token = parse_login_token(&body)?;
        *guard = Some(token.clone());
        Ok(Some(token))
    }
}

#[async_trait]
impl ConfigSource for HttpConfigSource {
    async fn get_config(&self, data_id: &str, group: &str) -> Result<String, ConfigError> {
        let token = self.access_token().await?;
        let url = build_config_url(
            &self.server_addr,
            data_id,
            group,
            &self.tenant,
            token.as_deref(),
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ConfigError::Nacos(format!("get config request failed: {e}")))?;
        if !resp.status().is_success() {
            return Err(ConfigError::Nacos(format!(
                "get config failed: HTTP {} ({data_id}/{group})",
                resp.status()
            )));
        }
        resp.text()
            .await
            .map_err(|e| ConfigError::Nacos(format!("get config body: {e}")))
    }

    async fn add_listener(
        &self,
        _data_id: &str,
        _group: &str,
        _callback: ListenerCallback,
    ) -> Result<(), ConfigError> {
        Err(ConfigError::Nacos(
            "config change listener is not supported by the HTTP transport".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_unreserved_characters_alone() {
        assert_eq!(encode_component("app.yml"), "app.yml");
        assert_eq!(encode_component("DEFAULT_GROUP"), "DEFAULT_GROUP");
        assert_eq!(encode_component("a-_.~9Z"), "a-_.~9Z");
    }

    #[test]
    fn escapes_reserved_and_non_ascii() {
        assert_eq!(encode_component("a b"), "a%20b");
        assert_eq!(encode_component("a&b=c"), "a%26b%3Dc");
        assert_eq!(encode_component("中"), "%E4%B8%AD");
    }

    #[test]
    fn builds_url_without_tenant_or_token() {
        assert_eq!(
            build_config_url("127.0.0.1:8848", "app.yml", "DEFAULT_GROUP", "", None),
            "http://127.0.0.1:8848/nacos/v1/cs/configs?group=DEFAULT_GROUP&dataId=app.yml"
        );
    }

    #[test]
    fn builds_url_with_tenant_and_token() {
        assert_eq!(
            build_config_url("nacos:8848", "app.yml", "DEFAULT_GROUP", "dev1", Some("tok")),
            "http://nacos:8848/nacos/v1/cs/configs?group=DEFAULT_GROUP&dataId=app.yml&tenant=dev1&accessToken=tok"
        );
    }

    #[test]
    fn encodes_data_id_with_special_characters() {
        assert_eq!(
            build_config_url("h:1", "my app.yml", "G", "", None),
            "http://h:1/nacos/v1/cs/configs?group=G&dataId=my%20app.yml"
        );
    }

    #[test]
    fn parses_access_token_from_login_response() {
        let body = r#"{"accessToken":"abc123","tokenTtl":18000,"globalAdmin":true}"#;
        assert_eq!(parse_login_token(body).unwrap(), "abc123");
    }

    #[test]
    fn rejects_login_response_without_token() {
        assert!(parse_login_token(r#"{"tokenTtl":18000}"#).is_err());
    }

    #[test]
    fn rejects_malformed_login_response() {
        assert!(parse_login_token("not json").is_err());
    }

    fn connection(server: &str, user: &str, password: &str) -> NacosConnection {
        NacosConnection {
            server_addr: server.to_string(),
            namespace: String::new(),
            username: user.to_string(),
            password: password.to_string(),
            use_grpc: false,
        }
    }

    #[tokio::test]
    async fn skips_login_when_no_credentials_configured() {
        // 指向不可达端口：未配凭据时不应发起任何请求，因此不会卡住或报错
        let source = HttpConfigSource::connect(&connection("127.0.0.1:1", "", ""));
        assert_eq!(source.access_token().await.unwrap(), None);
    }

    #[tokio::test]
    async fn listener_reports_unsupported() {
        let source = HttpConfigSource::connect(&connection("127.0.0.1:1", "", ""));
        let err = source
            .add_listener("app.yml", "DEFAULT_GROUP", std::sync::Arc::new(|_| {}))
            .await
            .unwrap_err();
        assert!(matches!(err, ConfigError::Nacos(_)), "got {err:?}");
    }
}
