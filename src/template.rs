//! `${VAR}` template engine with dot-notation, recursive resolution, and cycle
//! protection. Port of the Python `TemplateEngine`.

use std::collections::BTreeSet;

use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use serde_json::{Map, Value};

use crate::nested::{get_nested, set_nested};

static TEMPLATE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\{([^}]+)\}").unwrap());

/// Maximum number of resolution passes (cycle protection).
pub const MAX_RENDER_DEPTH: usize = 5;

/// Stringify a context value for substitution. Returns `None` when the value is
/// `null` (Python `None`), signalling "leave the placeholder untouched".
fn value_to_string(v: &Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        // Arrays/objects: no test exercises this; emit JSON (Python emits repr).
        other => Some(other.to_string()),
    }
}

fn extract_params(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::String(s) => {
            for cap in TEMPLATE_PATTERN.captures_iter(s) {
                out.insert(cap[1].to_string());
            }
        }
        Value::Array(a) => a.iter().for_each(|it| extract_params(it, out)),
        Value::Object(m) => m.values().for_each(|vv| extract_params(vv, out)),
        _ => {}
    }
}

fn render_value(v: &Value, ctx: &Value) -> Value {
    match v {
        Value::String(s) => Value::String(TemplateEngine::render_text(s, ctx)),
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|item| match item {
                    Value::String(s) => Value::String(TemplateEngine::render_text(s, ctx)),
                    Value::Object(_) => render_value(item, ctx),
                    // Nested arrays / scalars are kept as-is (matches Python).
                    other => other.clone(),
                })
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, vv)| (k.clone(), render_value(vv, ctx)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Zero-sized handle mirroring the Python `TemplateEngine` static API.
pub struct TemplateEngine;

impl TemplateEngine {
    /// Whether `text` contains at least one `${...}` placeholder.
    pub fn contains_template(text: &str) -> bool {
        TEMPLATE_PATTERN.is_match(text)
    }

    /// Whether a value is plain text: non-strings are text-only; strings are
    /// text-only when they hold no placeholder.
    pub fn is_text_only(value: &Value) -> bool {
        match value {
            Value::String(s) => !TEMPLATE_PATTERN.is_match(s),
            _ => true,
        }
    }

    /// Render a single string, resolving `${VAR}` (incl. dot-notation) against
    /// `context`, up to [`MAX_RENDER_DEPTH`] passes. Undefined vars are kept.
    pub fn render_text(text: &str, context: &Value) -> String {
        let mut current = text.to_string();
        for _ in 0..MAX_RENDER_DEPTH {
            let previous = current.clone();
            current = TEMPLATE_PATTERN
                .replace_all(&current, |caps: &Captures| {
                    let key = &caps[1];
                    match get_nested(context, key).and_then(value_to_string) {
                        Some(s) => s,
                        None => caps[0].to_string(),
                    }
                })
                .into_owned();
            if current == previous {
                break;
            }
        }
        current
    }

    /// Render every string in `config` against `context`. Pre-resolves the
    /// referenced params into an enriched context first (matches Python).
    /// Does not mutate `config`.
    pub fn render(config: &Value, context: &Value) -> Value {
        let mut params = BTreeSet::new();
        extract_params(config, &mut params);

        let mut enriched = if context.is_object() {
            context.clone()
        } else {
            Value::Object(Map::new())
        };

        for param in &params {
            let mut val: Value = get_nested(context, param).cloned().unwrap_or(Value::Null);
            // Resolve template chains for this param against the original context.
            while let Value::String(s) = &val {
                if TemplateEngine::is_text_only(&val) {
                    break;
                }
                let new = TemplateEngine::render_text(s, context);
                if &new == s {
                    break;
                }
                val = Value::String(new);
            }
            set_nested(&mut enriched, param, val);
        }

        render_value(config, &enriched)
    }
}
