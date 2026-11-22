use super::Account;
use super::common;
use baylee_account::decks::{DeckDraft, DeckSource, ImportedDeck, ImportedVersion};

fn history(through: u64) -> ImportedDeck {
    let note = "x".repeat(425);
    let versions = (1..=through)
        .map(|revision| ImportedVersion {
            revision,
            saved_at: i64::try_from(revision).unwrap(),
            draft: DeckDraft {
                name: "Large imported private log".into(),
                format: "freeform".into(),
                cards: (0..180)
                    .map(|i| format!("1 Island # {revision}/{i} {note}"))
                    .collect(),
                source: Some(DeckSource {
                    gateway: None,
                    account: Some("local:synthetic-install".into()),
                    deck: "synthetic-offline-deck".into(),
                    revision,
                }),
                ..DeckDraft::default()
            },
        })
        .collect();
    ImportedDeck { versions }
}

pub(super) fn check(account: &mut Account) {
    let source = account.data().legacy.as_ref().unwrap();
    assert!(
        source
            .paged
            .is_some_and(|manifest| manifest.bytes > 256 * 1024)
    );
    assert!(source.complete && source.restored_digest.is_some() && source.restored_root.is_some());
    let restored = account
        .list_decks()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "Paged at cutover")
        .unwrap();
    assert_eq!(restored.revision, 4);
    assert_eq!(
        account.deck_version(restored.id, 1).unwrap().draft.cards,
        gateway_rows(1)
    );
    let first = history(4);
    assert!(
        first
            .versions
            .iter()
            .flat_map(|v| &v.draft.cards)
            .map(String::len)
            .sum::<usize>()
            > 256 * 1024
    );
    account.import_decks([50; 32], vec![first]).unwrap();
    assert!(account.has_deck_import([50; 32]).unwrap());
    account.import_decks([50; 32], vec![history(4)]).unwrap();
    account.import_decks([51; 32], vec![history(5)]).unwrap();
    let entries = account.list_decks().unwrap();
    let rows: Vec<_> = entries
        .iter()
        .filter(|h| h.name == "Large imported private log")
        .collect();
    assert_eq!(rows.len(), 1);
    let id = rows[0].id;
    assert_eq!(rows[0].revision, 5);
    let saves = account.deck_history(id).unwrap();
    assert_eq!(
        saves.iter().map(|s| s.revision).collect::<Vec<_>>(),
        [4, 3, 2, 1]
    );
    assert_eq!(
        account.deck_version(id, 1).unwrap().draft.cards,
        history(1).versions.remove(0).draft.cards
    );
    let mut renamed = history(5);
    renamed.versions.last_mut().unwrap().draft.description = "Changed on the source".into();
    assert!(matches!(
        account.import_decks([53; 32], vec![renamed]),
        Err(baylee_account::Error::Conflict)
    ));
    assert!(!account.has_deck_import([53; 32]).unwrap());
    assert!(account.load_deck(id).unwrap().draft.description.is_empty());
    account.delete_deck(id, 5).unwrap();
    account.import_decks([52; 32], vec![history(5)]).unwrap();
    assert!(account.list_decks().unwrap().iter().all(|s| s.id != id));
}

fn gateway_rows(revision: u64) -> Vec<String> {
    let note = "x".repeat(425);
    (0..180)
        .map(|row| format!("1 Island # {revision}/{row} {note}"))
        .collect()
}

/// Seed an actual over-inline-limit gateway source before the simulated cutover.
pub(super) fn seed_paged_source(port: u16, token: &str) {
    let body = |revision| {
        serde_json::json!({"name":"Paged at cutover","format":"freeform","cards":gateway_rows(revision)}).to_string()
    };
    let (status, reply) = common::http(port, "POST", "/decks", Some(token), &body(1));
    assert_eq!(status, 200);
    let id = common::json_field(&reply, "deck_id");
    for revision in 2..=4 {
        let (status, _) = common::http(
            port,
            "PUT",
            &format!("/decks/{id}"),
            Some(token),
            &body(revision),
        );
        assert!((200..300).contains(&status));
    }
}
