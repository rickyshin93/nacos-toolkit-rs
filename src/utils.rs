//! High-level config processing. Port of the Python `NacosConfigUtils`.

use serde_json::{Map, Value};

use crate::merger::ConfigMerger;
use crate::nested::{get_nested, set_nested};
use crate::parser::{ConfigParser, NacosParser, RawConfig};
use crate::template::TemplateEngine;

/// Default fields converted from comma-separated strings to arrays.
pub const DEFAULT_CONVERT_ARRAY_FIELDS: &[&str] = &["cors.whitelist"];

fn object_or_empty(v: Option<&Value>) -> Value {
    match v {
        Some(v) if v.is_object() => v.clone(),
        _ => Value::Object(Map::new()),
    }
}

/// Shallow-merge the entries of `src` over `dst` (top-level keys only).
fn shallow_extend(dst: &mut Value, src: &Value) {
    if let (Some(d), Some(s)) = (dst.as_object_mut(), src.as_object()) {
        for (k, v) in s {
            d.insert(k.clone(), v.clone());
        }
    }
}

/// Zero-sized handle mirroring the Python `NacosConfigUtils` static API.
pub struct NacosConfigUtils;

impl NacosConfigUtils {
    /// Parse a config, render `${VAR}` templates against `external_vars` + the
    /// parsed config itself, then convert comma-separated fields to arrays.
    ///
    /// `external_vars = None` => `{}`; `convert_array_fields = None` =>
    /// [`DEFAULT_CONVERT_ARRAY_FIELDS`].
    pub fn process_configuration(
        raw_config: impl Into<RawConfig>,
        fmt: NacosParser,
        external_vars: Option<&Value>,
        convert_array_fields: Option<&[&str]>,
    ) -> Value {
        let parsed = ConfigParser::parse(raw_config, fmt);

        // context = {**external_vars, **parsed} — parsed wins.
        let mut context = object_or_empty(external_vars);
        shallow_extend(&mut context, &parsed);

        let rendered = TemplateEngine::render(&parsed, &context);
        let fields = convert_array_fields.unwrap_or(DEFAULT_CONVERT_ARRAY_FIELDS);
        Self::convert_string_fields_to_arrays(rendered, fields)
    }

    /// Process a custom config (which may reference base vars) and deep-merge it
    /// on top of `base_config`.
    pub fn process_and_merge_custom_config(
        base_config: &Value,
        custom_config: impl Into<RawConfig>,
        fmt: NacosParser,
        external_vars: Option<&Value>,
        convert_array_fields: Option<&[&str]>,
    ) -> Value {
        // merged_vars = {**external_vars, **base} — base wins.
        let mut merged_vars = object_or_empty(external_vars);
        shallow_extend(&mut merged_vars, base_config);

        let processed_custom = Self::process_configuration(
            custom_config,
            fmt,
            Some(&merged_vars),
            convert_array_fields,
        );
        ConfigMerger::merge(base_config, Some(&processed_custom))
    }

    /// Deep-merge `custom` on top of `base` (delegates to [`ConfigMerger`]).
    pub fn merge_configurations(base: &Value, custom: Option<&Value>) -> Value {
        ConfigMerger::merge(base, custom)
    }

    /// Whether `text` contains a `${...}` placeholder.
    pub fn contains_template(text: &str) -> bool {
        TemplateEngine::contains_template(text)
    }

    /// For each dot-path, if the value is a comma-containing string, split it
    /// into a trimmed array. Returns a new value.
    pub fn convert_string_fields_to_arrays(config: Value, field_paths: &[&str]) -> Value {
        let mut result = config;
        for path in field_paths {
            let converted = match get_nested(&result, path) {
                Some(Value::String(s)) if s.contains(',') => Some(
                    s.split(',')
                        .map(|item| Value::String(item.trim().to_string()))
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            };
            if let Some(arr) = converted {
                set_nested(&mut result, path, Value::Array(arr));
            }
        }
        result
    }

    /// Get a nested value by dot-path (returns an owned clone).
    pub fn get_nested_property(obj: &Value, path: &str) -> Option<Value> {
        get_nested(obj, path).cloned()
    }

    /// Set a nested value by dot-path, creating intermediate objects.
    pub fn set_nested_property(obj: &mut Value, path: &str, value: Value) {
        set_nested(obj, path, value);
    }
}
