//! Cross-check against the Python reference implementation. The expected value
//! is the verbatim JSON output of the Python `nacos-toolkit` on the same input.

use nacos_toolkit::{NacosConfigUtils, NacosParser};
use serde_json::json;

#[test]
fn matches_python_on_complex_template_graph() {
    let yaml = r#"
proto: https
host: api.example.com
base: ${proto}://${host}
endpoints:
  - ${base}/users
  - ${base}/orders
nested:
  url: ${base}/v2
cors:
  whitelist: 'http://a.com, http://b.com, http://c.com'
port: 8080
ref: ${nested.url}
"#;
    let r = NacosConfigUtils::process_configuration(
        yaml,
        NacosParser::Yaml,
        Some(&json!({"EXTRA": "x"})),
        None,
    );

    // Verbatim Python output (json.dumps, sort_keys=True).
    let expected = json!({
        "base": "https://api.example.com",
        "cors": {"whitelist": ["http://a.com", "http://b.com", "http://c.com"]},
        "endpoints": ["https://api.example.com/users", "https://api.example.com/orders"],
        "host": "api.example.com",
        "nested": {"url": "https://api.example.com/v2"},
        "port": 8080,
        "proto": "https",
        "ref": "https://api.example.com/v2"
    });

    assert_eq!(r, expected);
}
