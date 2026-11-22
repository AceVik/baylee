use super::Account;
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
    account.delete_deck(id, 5).unwrap();
    account.import_decks([52; 32], vec![history(5)]).unwrap();
    assert!(account.list_decks().unwrap().iter().all(|s| s.id != id));
}
