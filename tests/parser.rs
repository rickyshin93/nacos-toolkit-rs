use nacos_toolkit::{ConfigParser, NacosParser};
use serde_json::json;

#[test]
fn parse_yaml_string() {
    let r = ConfigParser::parse("name: test\nport: 8080", NacosParser::Yaml);
    assert_eq!(r, json!({"name": "test", "port": 8080}));
}

#[test]
fn parse_json_string() {
    let r = ConfigParser::parse(r#"{"name": "test", "port": 8080}"#, NacosParser::Json);
    assert_eq!(r, json!({"name": "test", "port": 8080}));
}

#[test]
fn parse_json_object() {
    // Already-parsed value passed through unchanged (Python `dict(raw)`).
    let obj = json!({"name": "test", "port": 8080});
    let r = ConfigParser::parse(&obj, NacosParser::Json);
    assert_eq!(r, json!({"name": "test", "port": 8080}));
}

#[test]
fn parse_yaml_with_nested() {
    let r = ConfigParser::parse(
        "server:\n  host: localhost\n  port: 3000",
        NacosParser::Yaml,
    );
    assert_eq!(r, json!({"server": {"host": "localhost", "port": 3000}}));
}

#[test]
fn parse_yaml_with_array() {
    let r = ConfigParser::parse("items:\n  - a\n  - b\n  - c", NacosParser::Yaml);
    assert_eq!(r, json!({"items": ["a", "b", "c"]}));
}

#[test]
fn parse_invalid_yaml_returns_empty() {
    let r = ConfigParser::parse(":::invalid", NacosParser::Yaml);
    assert_eq!(r, json!({}));
}

#[test]
fn parse_invalid_json_returns_empty() {
    let r = ConfigParser::parse("{invalid}", NacosParser::Json);
    assert_eq!(r, json!({}));
}

#[test]
fn nacos_parser_extensions() {
    assert_eq!(NacosParser::Yaml.extension(), ".yml");
    assert_eq!(NacosParser::Json.extension(), ".json");
}
