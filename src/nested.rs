//! Dot-notation nested property access on `serde_json::Value`.
//!
//! Mirrors the Python `_get_nested_property` / `_set_nested_property` helpers.

use serde_json::{Map, Value};

/// Get a nested value by dot-path (e.g. `"redis.hostname"`).
///
/// Matches Python semantics: a missing key, a non-object intermediate, or a
/// `null` (Python `None`) at any step yields `None`.
pub fn get_nested<'a>(obj: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = obj;
    for key in path.split('.') {
        let map = cur.as_object()?;
        cur = map.get(key)?;
        if cur.is_null() {
            return None;
        }
    }
    Some(cur)
}

/// Set a nested value by dot-path, creating intermediate objects as needed.
///
/// Matches Python: if an intermediate key is missing or is not an object, it is
/// replaced with a fresh object.
pub fn set_nested(obj: &mut Value, path: &str, value: Value) {
    let keys: Vec<&str> = path.split('.').collect();
    if keys.is_empty() {
        return;
    }
    let mut cur = obj;
    for key in &keys[..keys.len() - 1] {
        let map = match cur.as_object_mut() {
            Some(m) => m,
            None => return,
        };
        let child = map
            .entry((*key).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !child.is_object() {
            *child = Value::Object(Map::new());
        }
        cur = child;
    }
    if let Some(map) = cur.as_object_mut() {
        map.insert(keys[keys.len() - 1].to_string(), value);
    }
}
