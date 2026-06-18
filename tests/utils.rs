use nacos_toolkit::{NacosConfigUtils, NacosParser};
use serde_json::json;

// ---- get_nested_property ----

#[test]
fn get_simple_key() {
    assert_eq!(
        NacosConfigUtils::get_nested_property(&json!({"a": 1}), "a"),
        Some(json!(1))
    );
}

#[test]
fn get_nested_key() {
    assert_eq!(
        NacosConfigUtils::get_nested_property(&json!({"a": {"b": 2}}), "a.b"),
        Some(json!(2))
    );
}

#[test]
fn get_missing_key() {
    assert_eq!(
        NacosConfigUtils::get_nested_property(&json!({"a": 1}), "b"),
        None
    );
}

#[test]
fn get_deeply_nested() {
    let obj = json!({"a": {"b": {"c": {"d": 42}}}});
    assert_eq!(
        NacosConfigUtils::get_nested_property(&obj, "a.b.c.d"),
        Some(json!(42))
    );
}

#[test]
fn get_none_intermediate() {
    assert_eq!(
        NacosConfigUtils::get_nested_property(&json!({"a": null}), "a.b"),
        None
    );
}

// ---- set_nested_property ----

#[test]
fn set_simple() {
    let mut obj = json!({});
    NacosConfigUtils::set_nested_property(&mut obj, "a", json!(1));
    assert_eq!(obj, json!({"a": 1}));
}

#[test]
fn set_nested() {
    let mut obj = json!({});
    NacosConfigUtils::set_nested_property(&mut obj, "a.b", json!(2));
    assert_eq!(obj, json!({"a": {"b": 2}}));
}

#[test]
fn set_overwrite_existing() {
    let mut obj = json!({"a": {"b": 1}});
    NacosConfigUtils::set_nested_property(&mut obj, "a.b", json!(99));
    assert_eq!(obj, json!({"a": {"b": 99}}));
}

// ---- convert_string_fields_to_arrays ----

#[test]
fn converts_comma_separated_string() {
    let config = json!({"cors": {"whitelist": "http://a.com, http://b.com"}});
    let r = NacosConfigUtils::convert_string_fields_to_arrays(config, &["cors.whitelist"]);
    assert_eq!(
        r["cors"]["whitelist"],
        json!(["http://a.com", "http://b.com"])
    );
}

#[test]
fn no_comma_keeps_string() {
    let config = json!({"cors": {"whitelist": "http://a.com"}});
    let r = NacosConfigUtils::convert_string_fields_to_arrays(config, &["cors.whitelist"]);
    assert_eq!(r["cors"]["whitelist"], json!("http://a.com"));
}

#[test]
fn already_array_unchanged() {
    let config = json!({"cors": {"whitelist": ["http://a.com", "http://b.com"]}});
    let r = NacosConfigUtils::convert_string_fields_to_arrays(config, &["cors.whitelist"]);
    assert_eq!(
        r["cors"]["whitelist"],
        json!(["http://a.com", "http://b.com"])
    );
}

#[test]
fn missing_field_no_error() {
    let config = json!({"other": "value"});
    let r = NacosConfigUtils::convert_string_fields_to_arrays(config, &["cors.whitelist"]);
    assert_eq!(r, json!({"other": "value"}));
}

#[test]
fn trims_whitespace() {
    let config = json!({"tags": "a , b , c"});
    let r = NacosConfigUtils::convert_string_fields_to_arrays(config, &["tags"]);
    assert_eq!(r["tags"], json!(["a", "b", "c"]));
}

// ---- process_configuration ----

#[test]
fn basic_yaml_processing() {
    let r = NacosConfigUtils::process_configuration(
        "name: test\nport: 8080",
        NacosParser::Yaml,
        None,
        None,
    );
    assert_eq!(r, json!({"name": "test", "port": 8080}));
}

#[test]
fn yaml_with_template_vars() {
    let r = NacosConfigUtils::process_configuration(
        "host: ${HOST}\nport: 3000",
        NacosParser::Yaml,
        Some(&json!({"HOST": "localhost"})),
        None,
    );
    assert_eq!(r["host"], json!("localhost"));
    assert_eq!(r["port"], json!(3000));
}

#[test]
fn json_processing() {
    let r = NacosConfigUtils::process_configuration(
        r#"{"name": "test"}"#,
        NacosParser::Json,
        None,
        None,
    );
    assert_eq!(r, json!({"name": "test"}));
}

#[test]
fn self_referencing_variables() {
    let r = NacosConfigUtils::process_configuration(
        "host: localhost\nurl: http://${host}:8080",
        NacosParser::Yaml,
        None,
        None,
    );
    assert_eq!(r["url"], json!("http://localhost:8080"));
}

#[test]
fn default_cors_whitelist_conversion() {
    let r = NacosConfigUtils::process_configuration(
        "cors:\n  whitelist: 'http://a.com, http://b.com'",
        NacosParser::Yaml,
        None,
        None,
    );
    assert_eq!(
        r["cors"]["whitelist"],
        json!(["http://a.com", "http://b.com"])
    );
}

#[test]
fn preserves_yaml_arrays() {
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
fn mixed_types() {
    let yaml = r#"
features:
  apis:
    - name: user
      endpoints: ["/api/user", "/api/profile"]
    - name: auth
      endpoints: ["/api/login", "/api/logout"]
  enabled: true
  count: 42
"#;
    let r = NacosConfigUtils::process_configuration(yaml, NacosParser::Yaml, None, None);
    assert_eq!(r["features"]["apis"].as_array().unwrap().len(), 2);
    assert_eq!(
        r["features"]["apis"][0]["endpoints"],
        json!(["/api/user", "/api/profile"])
    );
    assert_eq!(r["features"]["enabled"], json!(true));
    assert_eq!(r["features"]["count"], json!(42));
}

#[test]
fn real_whitelist_issue() {
    let yaml = "cors:\n  whitelist:\n    - http://web-app-dev.dev1.eks.example.com\n    - http://localhost:8000";
    let r = NacosConfigUtils::process_configuration(yaml, NacosParser::Yaml, None, None);
    assert_eq!(
        r["cors"]["whitelist"],
        json!([
            "http://web-app-dev.dev1.eks.example.com",
            "http://localhost:8000"
        ])
    );
}

// ---- process_and_merge_custom_config ----

#[test]
fn merge_custom_overrides_base() {
    let base = json!({"host": "localhost", "port": 3000});
    let r = NacosConfigUtils::process_and_merge_custom_config(
        &base,
        "port: 8080\nnewKey: value",
        NacosParser::Yaml,
        None,
        None,
    );
    assert_eq!(r["host"], json!("localhost"));
    assert_eq!(r["port"], json!(8080));
    assert_eq!(r["newKey"], json!("value"));
}

#[test]
fn custom_can_use_base_vars() {
    let base = json!({"host": "localhost", "port": 3000});
    let r = NacosConfigUtils::process_and_merge_custom_config(
        &base,
        "url: http://${host}:${port}",
        NacosParser::Yaml,
        None,
        None,
    );
    assert_eq!(r["url"], json!("http://localhost:3000"));
}

#[test]
fn external_vars_available() {
    let base = json!({"host": "localhost"});
    let r = NacosConfigUtils::process_and_merge_custom_config(
        &base,
        "env: ${DEPLOY_ENV}",
        NacosParser::Yaml,
        Some(&json!({"DEPLOY_ENV": "production"})),
        None,
    );
    assert_eq!(r["env"], json!("production"));
}

#[test]
fn json_custom_config() {
    let base = json!({"a": 1});
    let r = NacosConfigUtils::process_and_merge_custom_config(
        &base,
        r#"{"b": 2}"#,
        NacosParser::Json,
        None,
        None,
    );
    assert_eq!(r, json!({"a": 1, "b": 2}));
}

#[test]
fn merge_configurations_delegates() {
    let r = NacosConfigUtils::merge_configurations(&json!({"a": 1}), Some(&json!({"b": 2})));
    assert_eq!(r, json!({"a": 1, "b": 2}));
}
