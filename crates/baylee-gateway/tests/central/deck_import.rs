//! Real signed gateway admission, private history over the old 192-KiB backup
//! ceiling, interrupted source generation, and idempotent Central publication.
use super::common;
use baylee_account::{Account, Error, GatewayDeckImport};
use serde_json::{Value, json};

pub(super) fn run(owner: &mut Account, other: &Account, port: u16, origin: &str) {
    let admission = owner.admit(origin).unwrap();
    let token = Some(admission.token.as_str());
    let foreign = other.admit(origin).unwrap();
    let cards = |rev| {
        (0..110)
            .map(|i| format!("1 Forest # {rev}/{i} {}", "z".repeat(470)))
            .collect::<Vec<_>>()
    };
    let body = |rev, name| {
        json!({"name":name,"format":"freeform","cards":cards(rev),"summary":format!("save {rev}")})
            .to_string()
    };
    let (status, reply) = common::http(port, "POST", "/decks", token, &body(1, "Import test"));
    assert_eq!(status, 200);
    let id = common::json_field(&reply, "deck_id");
    let path = format!("/decks/{id}");
    for n in 2..=5 {
        let (status, _) = common::http(port, "PUT", &path, token, &body(n, "Import test"));
        assert_eq!(status, 200);
    }
    let (status, reply) = common::http(
        port,
        "GET",
        &format!("/decks/private-export?deck={id}"),
        Some(&foreign.token),
        "",
    );
    assert_eq!(status, 409);
    assert!(!reply.contains("Import test"));
    let mut progress = GatewayDeckImport::default();
    assert!(
        !owner
            .gateway_deck_import_step(origin, &admission.token, 7, &mut progress)
            .unwrap()
    );
    let (status, _) = common::http(port, "PUT", &path, token, &body(5, "Renamed before import"));
    assert_eq!(status, 200);
    assert!(matches!(
        owner.gateway_deck_import_step(origin, &admission.token, 7, &mut progress),
        Err(Error::Conflict)
    ));
    for _ in 0..10 {
        if owner
            .gateway_deck_import_step(origin, &admission.token, 7, &mut progress)
            .unwrap()
        {
            break;
        }
    }
    assert_eq!(progress.scanned_decks(), 1);
    assert_eq!(progress.imported_decks(), 1);
    let owned = owner.list_decks().unwrap();
    let imported = owned
        .iter()
        .find(|d| d.name == "Renamed before import")
        .unwrap();
    assert_eq!(imported.revision, 5);
    for rev in 1..=5 {
        assert_eq!(
            owner.deck_version(imported.id, rev).unwrap().draft.cards,
            cards(rev)
        );
    }
    let mut again = GatewayDeckImport::default();
    assert!(
        !owner
            .gateway_deck_import_step(origin, &admission.token, 7, &mut again)
            .unwrap()
    );
    assert!(
        owner
            .gateway_deck_import_step(origin, &admission.token, 7, &mut again)
            .unwrap()
    );
    assert_eq!(again.imported_decks(), 0);
    let (status, original) = common::http(port, "GET", &path, token, "");
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_str::<Value>(&original).unwrap()["version"],
        5
    );
}
