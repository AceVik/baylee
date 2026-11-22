//! A real gateway export, Central's encrypted objects and two successive native logins.
use super::{Account, common};
use uuid::Uuid;
mod large;

pub(super) fn seed_gateway(port: u16, token: &str) {
    let draft = |cards: &str| {
        serde_json::json!({
            "name":"Before Central","format":"freeform","cards":[cards],"sideboard":["1 Plains"],
            "description":"Preserve this description","summary":"Add one Island"
        })
        .to_string()
    };
    let (status, body) = common::http(port, "POST", "/decks", Some(token), &draft("1 Island"));
    assert_eq!(status, 200, "create migration source");
    let id = common::json_field(&body, "deck_id");
    let (status, _) = common::http(
        port,
        "PUT",
        &format!("/decks/{id}"),
        Some(token),
        &draft("2 Island"),
    );
    assert!(
        (200..300).contains(&status),
        "save a second historical revision"
    );
}

pub(super) fn first_device(account: &mut Account) -> Uuid {
    large::check(account);
    let entries = account.list_decks().unwrap();
    let imported = entries
        .iter()
        .find(|deck| deck.name == "Before Central")
        .unwrap();
    let snapshot = account.load_deck(imported.id).unwrap();
    assert_eq!(snapshot.revision, 2);
    assert_eq!(snapshot.draft.cards, ["2 Island"]);
    assert_eq!(snapshot.draft.sideboard, ["1 Plains"]);
    let old = account.deck_version(imported.id, 1).unwrap();
    assert_eq!(old.draft.cards, ["1 Island"]);
    assert_eq!(old.draft.summary.as_deref(), Some("Add one Island"));
    let copied = account
        .copy_deck(snapshot, "Across devices".into())
        .unwrap();
    assert_ne!(copied.id, imported.id);
    copied.id
}

pub(super) fn second_device(first: &mut Account, second: &mut Account, id: Uuid) {
    let entries = second.list_decks().unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|deck| deck.name == "Before Central")
            .count(),
        1
    );
    let initial = second.load_deck(id).unwrap();
    let mut changed = initial.draft.clone();
    changed.cards = vec!["3 Island".into()];
    let updated = second
        .save_deck(Some(id), Some(initial.revision), changed.clone())
        .unwrap();
    assert!(
        first.delete_deck(id, updated.revision).is_err(),
        "revoked login cannot delete"
    );
    changed.name = "stale edit".into();
    assert!(
        second
            .save_deck(Some(id), Some(initial.revision), changed)
            .is_err()
    );
    let restored = second
        .revert_deck(id, updated.revision, initial.revision)
        .unwrap();
    assert!(restored.revision > updated.revision);
    let latest = second.load_deck(id).unwrap();
    assert_eq!(latest.draft.cards, initial.draft.cards);
    assert_eq!(latest.draft.description, "Preserve this description");
    second.delete_deck(id, latest.revision).unwrap();
    assert!(second.load_deck(id).is_err());
    assert!(
        second
            .list_decks()
            .unwrap()
            .iter()
            .all(|deck| deck.id != id)
    );
}
