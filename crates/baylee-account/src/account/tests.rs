use super::*;
use serde_json::json;

#[test]
fn typed_settings_preserve_unknown_nested_fields_and_explicit_known_resets() {
    let mut prior = json!({"chat":{"enabled":true,"future":{"key":3}},"new":4});
    overlay(&mut prior, json!({"chat":{"enabled":false}}));
    assert_eq!(
        prior,
        json!({"chat":{"enabled":false,"future":{"key":3}},"new":4})
    );
    overlay(&mut prior, json!({"chat":{"enabled":null}}));
    assert_eq!(prior["chat"]["enabled"], serde_json::Value::Null);
    assert_eq!(prior["chat"]["future"]["key"], 3);
}

#[test]
fn concurrent_preferences_preserve_independent_changes_but_reject_same_value_conflicts() {
    let base = json!({"keymap":{"confirm":"A", "cancel":"B"}, "future":1});
    let local = json!({"keymap":{"confirm":"C", "cancel":"B"}, "future":1});
    let remote = json!({"keymap":{"confirm":"A", "cancel":"D"}, "future":2});
    assert_eq!(
        merge(&base, &local, &remote).unwrap(),
        json!({"keymap":{"confirm":"C","cancel":"D"},"future":2})
    );
    assert_eq!(
        merge(&base, &local, &local).unwrap(),
        local,
        "a lost acknowledgement is idempotent"
    );
    assert!(
        merge(
            &base,
            &local,
            &json!({"keymap":{"confirm":"E","cancel":"B"},"future":1})
        )
        .is_err()
    );
    assert!(merge(&json!({}), &json!({"new":1}), &json!({"new":2})).is_err());
}

#[test]
fn profile_handle_renders_canonical_lowercase_hex_tags() {
    assert_eq!(format_profile_handle("Alice".into(), Some(7)), "Alice#0007");
    assert_eq!(
        format_profile_handle("Argon".into(), Some(42)),
        "Argon#002a"
    );
    assert_eq!(
        format_profile_handle("AceVik".into(), Some(100)),
        "AceVik#0064"
    );
    assert_eq!(
        format_profile_handle("Alice".into(), Some(44803)),
        "Alice#af03"
    );
    assert_eq!(
        format_profile_handle("Alice".into(), Some(0x1_0000)),
        "Alice#10000"
    );
    assert_eq!(format_profile_handle("Alice".into(), None), "Alice");
}
