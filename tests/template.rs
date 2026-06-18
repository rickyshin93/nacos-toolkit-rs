use nacos_toolkit::TemplateEngine;
use serde_json::json;

// ---- contains_template ----

#[test]
fn detects_template_expression() {
    assert!(TemplateEngine::contains_template("${HOST}"));
}

#[test]
fn no_template() {
    assert!(!TemplateEngine::contains_template("just text"));
}

#[test]
fn template_in_mixed_text() {
    assert!(TemplateEngine::contains_template("http://${HOST}:${PORT}"));
}

#[test]
fn empty_string() {
    assert!(!TemplateEngine::contains_template(""));
}

// ---- is_text_only ----

#[test]
fn plain_text_is_text_only() {
    assert!(TemplateEngine::is_text_only(&json!("hello")));
}

#[test]
fn non_string_is_text_only() {
    assert!(TemplateEngine::is_text_only(&json!(123)));
}

#[test]
fn template_is_not_text_only() {
    assert!(!TemplateEngine::is_text_only(&json!("${VAR}")));
}

#[test]
fn mixed_text_is_not_text_only() {
    assert!(!TemplateEngine::is_text_only(&json!("http://${HOST}")));
}

// ---- render_text ----

#[test]
fn simple_variable_substitution() {
    let r = TemplateEngine::render_text("${HOST}", &json!({"HOST": "localhost"}));
    assert_eq!(r, "localhost");
}

#[test]
fn multiple_variables() {
    let r = TemplateEngine::render_text(
        "${HOST}:${PORT}",
        &json!({"HOST": "localhost", "PORT": "8080"}),
    );
    assert_eq!(r, "localhost:8080");
}

#[test]
fn undefined_variable_keeps_original() {
    let r = TemplateEngine::render_text("${UNKNOWN}", &json!({}));
    assert_eq!(r, "${UNKNOWN}");
}

#[test]
fn nested_variable_resolution() {
    let ctx = json!({"URL": "${HOST}:${PORT}", "HOST": "localhost", "PORT": "3000"});
    let r = TemplateEngine::render_text("${URL}", &ctx);
    assert_eq!(r, "localhost:3000");
}

#[test]
fn max_render_depth_prevents_infinite_loop() {
    let ctx = json!({"A": "${B}", "B": "${A}"});
    let r = TemplateEngine::render_text("${A}", &ctx);
    // Just must terminate and return a string.
    assert!(r.contains("${"));
}

// ---- render ----

#[test]
fn render_simple_config() {
    let config = json!({"host": "${HOST}", "port": "${PORT}"});
    let ctx = json!({"HOST": "localhost", "PORT": "8080"});
    let r = TemplateEngine::render(&config, &ctx);
    assert_eq!(r["host"], json!("localhost"));
    assert_eq!(r["port"], json!("8080"));
}

#[test]
fn render_nested_config() {
    let config = json!({"server": {"host": "${HOST}", "port": 3000}});
    let ctx = json!({"HOST": "localhost"});
    let r = TemplateEngine::render(&config, &ctx);
    assert_eq!(r["server"]["host"], json!("localhost"));
    assert_eq!(r["server"]["port"], json!(3000));
}

#[test]
fn render_preserves_arrays() {
    let config = json!({"whitelist": ["http://a.com", "http://b.com"]});
    let r = TemplateEngine::render(&config, &json!({}));
    assert_eq!(r["whitelist"], json!(["http://a.com", "http://b.com"]));
}

#[test]
fn render_templates_in_array_items() {
    let config = json!({"urls": ["${BASE_URL}", "http://localhost"]});
    let ctx = json!({"BASE_URL": "http://example.com"});
    let r = TemplateEngine::render(&config, &ctx);
    assert_eq!(r["urls"], json!(["http://example.com", "http://localhost"]));
}

#[test]
fn render_objects_in_array() {
    let config = json!({"apis": [{"url": "${API_HOST}/users"}]});
    let ctx = json!({"API_HOST": "http://api.com"});
    let r = TemplateEngine::render(&config, &ctx);
    assert_eq!(r["apis"][0]["url"], json!("http://api.com/users"));
}

#[test]
fn render_with_nested_template_references() {
    let config = json!({"full_url": "${URL}"});
    let ctx = json!({"URL": "${PROTO}://${HOST}", "PROTO": "https", "HOST": "example.com"});
    let r = TemplateEngine::render(&config, &ctx);
    assert_eq!(r["full_url"], json!("https://example.com"));
}

#[test]
fn render_preserves_non_string_values() {
    let config = json!({"enabled": true, "count": 42, "name": "${NAME}"});
    let ctx = json!({"NAME": "test"});
    let r = TemplateEngine::render(&config, &ctx);
    assert_eq!(r["enabled"], json!(true));
    assert_eq!(r["count"], json!(42));
    assert_eq!(r["name"], json!("test"));
}

#[test]
fn render_does_not_mutate_original() {
    let config = json!({"host": "${HOST}"});
    let ctx = json!({"HOST": "localhost"});
    let r = TemplateEngine::render(&config, &ctx);
    assert_eq!(r["host"], json!("localhost"));
    assert_eq!(config["host"], json!("${HOST}"));
}

#[test]
fn render_enriches_context_with_resolved_params() {
    let config = json!({"db_url": "${DB_HOST}:${DB_PORT}"});
    let ctx = json!({"DB_HOST": "mysql-server", "DB_PORT": "3306"});
    let r = TemplateEngine::render(&config, &ctx);
    assert_eq!(r["db_url"], json!("mysql-server:3306"));
}

#[test]
fn render_handles_dot_notation_in_context() {
    let config = json!({"url": "${api.host}"});
    let ctx = json!({"api": {"host": "http://api.com"}});
    let r = TemplateEngine::render(&config, &ctx);
    assert_eq!(r["url"], json!("http://api.com"));
}
