//! Deep-merge for config objects. Port of the Python `ConfigMerger`.

use serde_json::{Map, Value};

/// Zero-sized handle mirroring the Python `ConfigMerger` static API.
pub struct ConfigMerger;

impl ConfigMerger {
    /// Deep-merge `custom` on top of `base` and return a new value.
    ///
    /// - objects are merged recursively
    /// - arrays are replaced entirely (no element-level merge)
    /// - scalars from `custom` override `base`
    /// - `custom = None` returns a clone of `base`
    ///
    /// Neither input is mutated.
    pub fn merge(base: &Value, custom: Option<&Value>) -> Value {
        let custom = custom.cloned().unwrap_or_else(|| Value::Object(Map::new()));
        let mut result = base.clone();
        deep_merge(&mut result, &custom);
        result
    }
}

fn deep_merge(base: &mut Value, override_: &Value) {
    match (base.as_object_mut(), override_.as_object()) {
        (Some(b), Some(o)) => {
            for (k, v) in o {
                match b.get_mut(k) {
                    Some(bv) if bv.is_object() && v.is_object() => deep_merge(bv, v),
                    _ => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        _ => *base = override_.clone(),
    }
}
