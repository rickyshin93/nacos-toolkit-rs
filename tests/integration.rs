//! End-to-end config processing, ported from the Python `test_integration.py`.

use nacos_toolkit::{NacosConfigUtils, NacosParser};
use serde_json::json;

#[test]
fn preserve_array_types_in_yaml() {
    let yaml = "cors:\n  whitelist:\n    - http://localhost:8000\n    - http://example.com\nlogLevel: info";
    let r = NacosConfigUtils::process_configuration(yaml, NacosParser::Yaml, None, None);
    assert_eq!(
        r["cors"]["whitelist"],
        json!(["http://localhost:8000", "http://example.com"])
    );
    assert_eq!(r["logLevel"], json!("info"));
}

#[test]
fn arrays_with_template_variables() {
    let yaml = "cors:\n  whitelist:\n    - ${BASE_URL}\n    - http://localhost:8000\n    - ${CUSTOM_DOMAIN}";
    let r = NacosConfigUtils::process_configuration(
        yaml,
        NacosParser::Yaml,
        Some(&json!({"BASE_URL": "http://example.com", "CUSTOM_DOMAIN": "http://custom.com"})),
        None,
    );
    assert_eq!(
        r["cors"]["whitelist"],
        json!([
            "http://example.com",
            "http://localhost:8000",
            "http://custom.com"
        ])
    );
}

#[test]
fn end_to_end_config_processing() {
    let common_yaml = "REDIS_HOSTNAME: master.redis.example.com\nREDIS_PORT: \"6379\"\nREDIS_PASSWORD: test-password-placeholder";
    let app_yaml = r#"
logLevel: info
debugMode: false
redis:
  database: 0
  hostname: ${REDIS_HOSTNAME}
  port: ${REDIS_PORT}
  password: ${REDIS_PASSWORD}
cors:
  enabled: "true"
  whitelist:
    - http://base-web.dev.example.com
    - http://localhost:8000
  credentials: "true"
serverOptions:
  proxyTimeout: "60000"
  timeout: "60000"
"#;
    let common =
        NacosConfigUtils::process_configuration(common_yaml, NacosParser::Yaml, None, None);
    let mut external = common;
    external
        .as_object_mut()
        .unwrap()
        .insert("DEPLOY_ENV".into(), json!("dev1"));

    let r =
        NacosConfigUtils::process_configuration(app_yaml, NacosParser::Yaml, Some(&external), None);
    assert_eq!(r["logLevel"], json!("info"));
    assert_eq!(r["debugMode"], json!(false));
    assert_eq!(r["redis"]["hostname"], json!("master.redis.example.com"));
    assert_eq!(r["redis"]["port"], json!("6379"));
    assert_eq!(r["redis"]["database"], json!(0));
    assert_eq!(r["cors"]["whitelist"].as_array().unwrap().len(), 2);
}

#[test]
fn end_to_end_with_override() {
    let base = json!({"host": "localhost", "port": 3000, "cors": {"whitelist": ["http://a.com"]}});
    let override_yaml = "port: 9999\ncors:\n  whitelist:\n    - http://b.com\n    - http://c.com";
    let r = NacosConfigUtils::process_and_merge_custom_config(
        &base,
        override_yaml,
        NacosParser::Yaml,
        None,
        None,
    );
    assert_eq!(r["host"], json!("localhost"));
    assert_eq!(r["port"], json!(9999));
    assert_eq!(
        r["cors"]["whitelist"],
        json!(["http://b.com", "http://c.com"])
    );
}
