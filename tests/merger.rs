use nacos_toolkit::ConfigMerger;
use serde_json::json;

#[test]
fn merge_flat_objects() {
    let r = ConfigMerger::merge(&json!({"a": 1, "b": 2}), Some(&json!({"b": 3, "c": 4})));
    assert_eq!(r, json!({"a": 1, "b": 3, "c": 4}));
}

#[test]
fn merge_nested_objects() {
    let r = ConfigMerger::merge(
        &json!({"server": {"host": "localhost", "port": 3000}}),
        Some(&json!({"server": {"port": 8080}})),
    );
    assert_eq!(r, json!({"server": {"host": "localhost", "port": 8080}}));
}

#[test]
fn arrays_are_replaced_not_merged() {
    let r = ConfigMerger::merge(
        &json!({"items": [1, 2, 3]}),
        Some(&json!({"items": [4, 5]})),
    );
    assert_eq!(r, json!({"items": [4, 5]}));
}

#[test]
fn merge_with_none_custom() {
    let r = ConfigMerger::merge(&json!({"a": 1}), None);
    assert_eq!(r, json!({"a": 1}));
}

#[test]
fn merge_does_not_mutate_originals() {
    let base = json!({"a": 1, "nested": {"x": 1}});
    let custom = json!({"nested": {"y": 2}});
    let r = ConfigMerger::merge(&base, Some(&custom));
    assert_eq!(r["nested"], json!({"x": 1, "y": 2}));
    assert_eq!(base["nested"], json!({"x": 1}));
}

#[test]
fn merge_deeply_nested() {
    let r = ConfigMerger::merge(
        &json!({"a": {"b": {"c": 1, "d": 2}}}),
        Some(&json!({"a": {"b": {"c": 99}}})),
    );
    assert_eq!(r, json!({"a": {"b": {"c": 99, "d": 2}}}));
}

#[test]
fn merge_empty_custom() {
    let r = ConfigMerger::merge(&json!({"a": 1}), Some(&json!({})));
    assert_eq!(r, json!({"a": 1}));
}

#[test]
fn merge_empty_base() {
    let r = ConfigMerger::merge(&json!({}), Some(&json!({"a": 1})));
    assert_eq!(r, json!({"a": 1}));
}
