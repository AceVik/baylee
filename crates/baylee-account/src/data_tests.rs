//! What a restored account document is held to: it is sealed by the client,
//! and an old or damaged one must be refused whole rather than half read.

use crate::{
    Error,
    data::{AccountData, LegacyData},
};
use central_api::legacy::{ArchiveManifest, MAX_SNAPSHOT_BYTES, source_generation};

fn plain() -> AccountData {
    AccountData {
        version: 1,
        gateways: Vec::new(),
        preferences: serde_json::json!({}),
        legacy: None,
        chat: crate::chat::ChatArchive::default(),
        deck_library: None,
    }
}

fn with_gateways(gateways: Vec<String>) -> Result<(), Error> {
    let data = AccountData {
        gateways,
        ..plain()
    };
    data.write().map(drop)
}

fn one(gateway: &str) -> bool {
    with_gateways(vec![gateway.into()]).is_ok()
}

/// A move from the official gateway that is finished and small.
fn moved() -> LegacyData {
    LegacyData {
        digest: [3; 32],
        complete: true,
        gateway: central_api::LEGACY_ORIGIN.into(),
        account_id: "account".into(),
        username: "Ada".into(),
        decks: serde_json::json!([{"id": "deck"}]),
        paged: None,
        restored_digest: None,
        restored_root: None,
    }
}

const DIGEST: [u8; 32] = [9; 32];

/// A move whose decks came in pages, not finished yet.
fn paged() -> LegacyData {
    let bytes = u32::try_from(MAX_SNAPSHOT_BYTES + 1).unwrap();
    LegacyData {
        complete: false,
        decks: serde_json::json!([]),
        paged: Some(ArchiveManifest {
            version: 1,
            bytes,
            generation: source_generation("account", DIGEST, bytes),
        }),
        ..moved()
    }
}

/// The same, with its decks restored.
fn restored() -> LegacyData {
    LegacyData {
        restored_digest: Some(DIGEST),
        restored_root: Some([4; 32]),
        ..paged()
    }
}

fn library() -> crate::decks::LibraryRef {
    serde_json::from_value(serde_json::json!({
        "generation": 1,
        "blob": {
            "bytes": 1,
            "digest": ([0_u8; 32]),
            "chunks": [{
                "id": uuid::Uuid::now_v7(), "revision": 1, "bytes": 1,
                "digest": ([0_u8; 32]),
            }],
        },
    }))
    .unwrap()
}

fn holds(legacy: LegacyData) -> bool {
    let data = AccountData {
        legacy: Some(legacy),
        ..plain()
    };
    data.write().is_ok()
}

#[test]
fn another_version_or_preferences_that_are_no_object_are_refused() {
    assert!(plain().write().is_ok());
    for version in [0, 2] {
        assert!(AccountData { version, ..plain() }.write().is_err());
    }
    for preferences in [serde_json::json!([]), serde_json::Value::Null] {
        let data = AccountData {
            preferences,
            ..plain()
        };
        assert!(data.write().is_err());
    }
}

#[test]
fn at_most_128_different_gateways_are_saved() {
    let list = |n: usize| (0..n).map(|i| format!("https://{i}.example")).collect();
    assert!(with_gateways(list(128)).is_ok());
    assert!(with_gateways(list(129)).is_err());
    let twice = vec!["https://a.example".to_owned(); 2];
    assert!(with_gateways(twice).is_err());
}

#[test]
fn a_saved_gateway_is_a_web_address_without_credentials_query_or_fragment() {
    assert!(one("https://a.example"));
    assert!(one("http://192.168.0.2:8080/baylee"));
    for bad in [
        "",
        "a.example",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "https://user@a.example",
        "https://:secret@a.example",
        "https://a.example/?next=x",
        "https://a.example/#x",
    ] {
        assert!(!one(bad), "{bad}");
    }
}

#[test]
fn a_saved_gateway_is_at_most_2048_bytes_long() {
    let address = |length: usize| format!("{:x<length$}", "https://a.example/");
    assert!(one(&address(2048)));
    assert!(!one(&address(2049)));
}

#[test]
fn the_legacy_account_is_named_and_its_decks_are_a_list() {
    assert!(holds(moved()));
    let named = |account_id: String| {
        holds(LegacyData {
            account_id,
            ..moved()
        })
    };
    assert!(named("x".repeat(64)));
    assert!(!named("x".repeat(65)));
    assert!(!named(String::new()));
    let called = |username: String| {
        holds(LegacyData {
            username,
            ..moved()
        })
    };
    assert!(called("x".repeat(96)));
    assert!(!called("x".repeat(97)));
    assert!(!holds(LegacyData {
        decks: serde_json::json!({}),
        ..moved()
    }));
}

#[test]
fn a_move_comes_from_the_official_gateway_or_a_fixture_on_this_machine() {
    let from = |gateway: &str| {
        holds(LegacyData {
            gateway: gateway.into(),
            ..moved()
        })
    };
    assert!(from(central_api::LEGACY_ORIGIN));
    assert!(from("http://127.0.0.1:4000"));
    assert!(!from("https://untrusted.example"));
    assert!(!from("http://192.168.0.2:4000"));
}

#[test]
fn a_move_that_was_not_paged_claims_no_restored_pages() {
    assert!(!holds(LegacyData {
        restored_digest: Some(DIGEST),
        ..moved()
    }));
    assert!(!holds(LegacyData {
        restored_root: Some([4; 32]),
        ..moved()
    }));
}

#[test]
fn a_paged_move_keeps_no_decks_of_its_own_and_a_manifest_that_can_be_true() {
    assert!(holds(paged()));
    assert!(!holds(LegacyData {
        decks: serde_json::json!([{"id": "deck"}]),
        ..paged()
    }));
    let mut small = paged();
    if let Some(manifest) = &mut small.paged {
        manifest.bytes -= 1;
    }
    assert!(!holds(small), "a source this small is never paged");
}

#[test]
fn restored_pages_are_bound_to_the_account_and_source_they_came_from() {
    assert!(holds(restored()));
    assert!(!holds(LegacyData {
        restored_digest: Some([8; 32]),
        ..restored()
    }));
    assert!(!holds(LegacyData {
        account_id: "somebody else".into(),
        ..restored()
    }));
    // The digest and the root are one fact and come together.
    assert!(!holds(LegacyData {
        restored_root: None,
        ..restored()
    }));
    assert!(!holds(LegacyData {
        restored_root: Some([4; 32]),
        ..paged()
    }));
}

#[test]
fn a_paged_move_is_complete_only_with_its_decks_in_the_library() {
    let finish = |legacy: LegacyData, deck_library| {
        let data = AccountData {
            legacy: Some(LegacyData {
                complete: true,
                ..legacy
            }),
            deck_library,
            ..plain()
        };
        data.write().is_ok()
    };
    assert!(finish(restored(), Some(library())));
    assert!(!finish(restored(), None));
    assert!(!finish(paged(), Some(library())));
}

#[test]
fn a_document_with_a_field_this_client_does_not_know_is_refused() {
    let written = plain().write().unwrap();
    assert!(AccountData::read(&written).is_ok());
    let mut value: serde_json::Value = serde_json::from_slice(&written).unwrap();
    value["bearer"] = "x".into();
    assert!(AccountData::read(value.to_string().as_bytes()).is_err());
}

#[test]
fn a_document_with_a_damaged_deck_library_is_refused() {
    let data = AccountData {
        deck_library: Some(library()),
        ..plain()
    };
    let written = data.write().unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&written).unwrap();
    assert!(AccountData::read(value.to_string().as_bytes()).is_ok());
    value["deck_library"]["generation"] = 0.into();
    assert!(AccountData::read(value.to_string().as_bytes()).is_err());
}
