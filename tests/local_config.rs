use std::fs;

use nacos_toolkit::{find_local_config, get_local_config, parse_config_file};
use serde_json::json;
use tempfile::tempdir;

// ---- find_local_config ----

#[test]
fn finds_json_file() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.json"), "{}").unwrap();
    let r = find_local_config("app", dir.path().to_str().unwrap());
    assert!(r.unwrap().ends_with("app.json"));
}

#[test]
fn finds_yaml_file() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.yaml"), "name: test").unwrap();
    let r = find_local_config("app", dir.path().to_str().unwrap());
    assert!(r.unwrap().ends_with("app.yaml"));
}

#[test]
fn finds_yml_file() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.yml"), "name: test").unwrap();
    let r = find_local_config("app", dir.path().to_str().unwrap());
    assert!(r.unwrap().ends_with("app.yml"));
}

#[test]
fn json_has_priority_over_yaml() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.json"), "{}").unwrap();
    fs::write(dir.path().join("app.yaml"), "name: test").unwrap();
    let r = find_local_config("app", dir.path().to_str().unwrap());
    assert!(r.unwrap().ends_with("app.json"));
}

#[test]
fn returns_none_when_not_found() {
    let dir = tempdir().unwrap();
    let r = find_local_config("missing", dir.path().to_str().unwrap());
    assert!(r.is_none());
}

// ---- parse_config_file ----

#[test]
fn parse_json_file() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("config.json");
    fs::write(&f, r#"{"name": "test", "port": 8080}"#).unwrap();
    let r = parse_config_file(f.to_str().unwrap()).unwrap();
    assert_eq!(r, json!({"name": "test", "port": 8080}));
}

#[test]
fn parse_yaml_file() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("config.yaml");
    fs::write(&f, "name: test\nport: 8080").unwrap();
    let r = parse_config_file(f.to_str().unwrap()).unwrap();
    assert_eq!(r, json!({"name": "test", "port": 8080}));
}

#[test]
fn parse_yml_file() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("config.yml");
    fs::write(&f, "items:\n  - 1\n  - 2\n  - 3").unwrap();
    let r = parse_config_file(f.to_str().unwrap()).unwrap();
    assert_eq!(r, json!({"items": [1, 2, 3]}));
}

#[test]
fn unsupported_format_raises() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("config.txt");
    fs::write(&f, "hello").unwrap();
    let err = parse_config_file(f.to_str().unwrap()).unwrap_err();
    assert!(err.to_string().contains("Unsupported"));
}

// ---- get_local_config ----

#[test]
fn reads_existing_config() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.json"), r#"{"key": "value"}"#).unwrap();
    let r = get_local_config("app", dir.path().to_str().unwrap());
    assert_eq!(r, Some(json!({"key": "value"})));
}

#[test]
fn get_returns_none_when_not_found() {
    let dir = tempdir().unwrap();
    let r = get_local_config("missing", dir.path().to_str().unwrap());
    assert!(r.is_none());
}

#[test]
fn returns_none_when_empty_filename() {
    let dir = tempdir().unwrap();
    let r = get_local_config("", dir.path().to_str().unwrap());
    assert!(r.is_none());
}
