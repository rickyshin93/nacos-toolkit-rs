# nacos-toolkit (Rust)

Rust port of a Python `nacos-toolkit`: Nacos configuration
parsing and management. Fetch configs from Nacos, render `${VAR}` templates,
deep-merge multiple configs, and read local config files.

Dynamic config values are represented as `serde_json::Value` (mirroring
Python's `dict[str, Any]`). YAML is parsed with `serde_norway`, the Nacos
transport is backed by [`nacos_rust_client`](https://crates.io/crates/nacos_rust_client).

## Features

- **Template engine** — `${VAR}` with dot-notation (`${redis.hostname}`),
  recursive resolution (≤ 5 passes), cycle protection, undefined-vars kept as-is.
- **Deep merge** — objects merged recursively, arrays replaced, scalars overridden.
- **YAML / JSON parsing** — invalid input degrades to an empty object.
- **Local config discovery** — priority `.json` → `.yaml` → `.yml`.
- **Async Nacos client** — fetch, cache, and listen for config changes.

## Quick start

### Fetch config from Nacos

```rust,no_run
use nacos_toolkit::{get_nacos_config, ConfigRef, NacosConnection};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let conn = NacosConnection {
    server_addr: "nacos-server:8848".into(),
    namespace: "production".into(),
    username: "nacos".into(),
    password: "nacos".into(),
    use_grpc: true,
};
let base = [
    ConfigRef::new("common.yml", "DEFAULT_GROUP"),
    ConfigRef::new("app.yml", "DEFAULT_GROUP"),
];
let result = get_nacos_config(&conn, &base, None, false).await?;
println!("{}", result.config);
# Ok(()) }
```

Behaviour mirrors the Python implementation: all `base_configs` are fetched in
order and shallow-merged into a variable context; only the **last** config is
processed (rendering `${VAR}`); `DEPLOY_ENV` is auto-injected as the namespace;
results are cached.

### Process a config string

```rust
use nacos_toolkit::{NacosConfigUtils, NacosParser};
use serde_json::json;

let cfg = NacosConfigUtils::process_configuration(
    "host: ${HOST}\nport: 3000",
    NacosParser::Yaml,
    Some(&json!({"HOST": "localhost"})),
    None, // convert_array_fields defaults to ["cors.whitelist"]
);
assert_eq!(cfg["host"], json!("localhost"));
```

### Local config files

```rust,no_run
use nacos_toolkit::{find_local_config, get_local_config, parse_config_file};

let cfg = get_local_config("app", "./config");      // auto-discover + parse
let path = find_local_config("app", "./config");    // path only
let cfg = parse_config_file("/path/to/config.yml");  // parse a specific file
```

## API surface

| Rust | Python |
| --- | --- |
| `TemplateEngine::{contains_template, is_text_only, render_text, render}` | `TemplateEngine` |
| `ConfigMerger::merge` | `ConfigMerger.merge` |
| `ConfigParser::parse`, `NacosParser` | `ConfigParser`, `NacosParser` |
| `NacosConfigUtils::{process_configuration, process_and_merge_custom_config, …}` | `NacosConfigUtils` |
| `find_local_config` / `parse_config_file` / `get_local_config` | same |
| `NacosConfigManager`, `get_nacos_config`, `setup_config_listener` | same |
| `ConfigSource` (trait) | mockable transport boundary |

The Nacos transport is abstracted behind the `ConfigSource` trait, so manager
logic is fully unit-testable with a mock (see `tests/manager.rs`).

## Testing

```bash
cargo test                                  # all logic + cross-check vs Python
cargo test --test live_nacos -- --ignored   # live smoke test (needs a Nacos server)
```

## License

MIT
