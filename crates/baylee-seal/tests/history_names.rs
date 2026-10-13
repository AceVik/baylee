//! History file names: keyed by the vault key, so a name says nothing of who the conversation is with.

use baylee_seal::history_name;
use baylee_seal::testkit::device;

const CONV: &str = "dm:0012:af03";

#[test]
fn a_history_name_is_sixteen_lower_case_hex_characters() {
    let name = history_name(device(1).vault_key(), CONV);
    assert_eq!(name.len(), 16);
    assert!(
        name.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "{name}"
    );
}

#[test]
fn a_history_name_is_stable_for_the_same_key_and_conversation() {
    let key = *device(1).vault_key();
    assert_eq!(history_name(&key, CONV), history_name(&key, CONV));
}

#[test]
fn a_history_name_differs_per_conversation() {
    let key = *device(1).vault_key();
    let names: Vec<String> = [CONV, "dm:0012:af04", "dm:0013:af03", "table:af03:1", ""]
        .iter()
        .map(|conv| history_name(&key, conv))
        .collect();
    for (i, a) in names.iter().enumerate() {
        for b in &names[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn a_history_name_differs_per_vault_key() {
    assert_ne!(
        history_name(device(1).vault_key(), CONV),
        history_name(device(2).vault_key(), CONV)
    );
}

#[test]
fn a_history_name_does_not_contain_the_conversation() {
    let name = history_name(device(1).vault_key(), CONV);
    assert!(!name.contains("0012"));
    assert!(!name.contains("af03"));
    assert!(!name.contains(CONV));
}
