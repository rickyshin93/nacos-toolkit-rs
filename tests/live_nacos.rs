//! Live smoke test against a real Nacos server. Ignored by default; requires a
//! running Nacos and env vars. Run with:
//!
//! ```bash
//! NACOS_ADDR=127.0.0.1:8848 NACOS_NS=public NACOS_USER=nacos NACOS_PASS=nacos \
//!   NACOS_DATA_ID=app.yml NACOS_GROUP=DEFAULT_GROUP \
//!   cargo test --test live_nacos -- --ignored --nocapture
//! ```

use nacos_toolkit::{get_nacos_config, ConfigRef, NacosConnection};

fn env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::test]
#[ignore = "requires a live Nacos server"]
async fn live_fetch_config() {
    let conn = NacosConnection {
        server_addr: env("NACOS_ADDR", "127.0.0.1:8848"),
        namespace: env("NACOS_NS", "public"),
        username: env("NACOS_USER", "nacos"),
        password: env("NACOS_PASS", "nacos"),
    };
    let base = [ConfigRef::new(
        env("NACOS_DATA_ID", "app.yml"),
        env("NACOS_GROUP", "DEFAULT_GROUP"),
    )];

    let result = get_nacos_config(&conn, &base, None, true)
        .await
        .expect("fetch config");
    println!(
        "config = {}",
        serde_json::to_string_pretty(&result.config).unwrap()
    );
    assert!(result.config.is_object());
}
