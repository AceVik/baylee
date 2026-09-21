//! The deck list and the door to the builder: a selection that can point neither past a refreshed list nor at a deck that was never there, saving, editing and deleting each re-reading the list rather than leaving a stale row on screen, and the pool fetched once so that coming back to the builder costs no round trip. The race between the two answers is asserted here too — rows that arrive before the pool are held by name and become entries when it lands — as is the builder's refusal to send a deck the gateway would reject. The deck a table is opened with is named in `rooms`; the builder's own rules, counts and zones are the `deckbuilder` module's tests and not these.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn a_selection_never_points_past_the_end_of_a_refreshed_list() {
    let mut lobby = seated_lobby();
    lobby.select_deck(0);
    lobby.refresh();
    lobby.apply(LobbyEvent::Decks(vec![]));
    assert_eq!(lobby.selected(), None);
}

#[test]
fn a_deck_that_does_not_exist_cannot_be_selected() {
    let mut lobby = seated_lobby();
    lobby.select_deck(9);
    assert_eq!(lobby.selected(), Some(0));
}

#[test]
fn saving_a_deck_re_reads_the_list() {
    let mut lobby = seated_lobby();
    assert_eq!(lobby.create_deck("Starter", vec![]), None, "no empty decks");
    let rows = vec!["40 Island".to_string(), "20 Forest".to_string()];
    assert_eq!(
        lobby.create_deck("Starter", rows.clone()),
        Some(LobbyRequest::SaveDeck {
            deck_id: None,
            name: "Starter".to_string(),
            cards: rows,
            sideboard: vec![],
            commanders: Vec::new(),
        })
    );
    assert_eq!(
        lobby.apply(LobbyEvent::DeckSaved {
            deck_id: Some("d9".to_string())
        }),
        Some(LobbyRequest::ListDecks)
    );
}

/// Opening the builder asks for the pool once. Coming back must not ask
/// again: it is the same few hundred cards, and the round trip would be
/// paid on every visit.
#[test]
fn the_card_pool_is_fetched_once() {
    let mut lobby = seated_lobby();
    assert_eq!(lobby.build_deck(), Some(LobbyRequest::LoadPool));
    assert_eq!(lobby.screen(), &Screen::Build);
    lobby.apply(LobbyEvent::Pool {
        cards: vec![PoolCard {
            index: 1,
            english_name: "Forest".to_string(),
            name: "Forest".to_string(),
            kinds: vec!["Land".to_string()],
            type_line: "Basic Land — Forest".to_string(),
            basic_land: true,
            coverage: Coverage::Implemented,
            ..PoolCard::default()
        }],
        has_text: false,
    });
    assert!(lobby.builder().loaded());
    lobby.close_builder();
    assert_eq!(lobby.build_deck(), None, "the pool is already here");
    assert_eq!(lobby.screen(), &Screen::Build);
}

/// Editing a saved deck asks for its rows — `GET /decks` lists counts, not
/// contents, so the builder cannot fill itself from the list.
#[test]
fn editing_a_deck_asks_for_its_rows() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
        id: "deck-1".to_string(),
        name: "Burn".to_string(),
        cards: 2,
        sideboard: 0,
        commanders: Vec::new(),
    }]));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(
        lobby.edit_deck(0),
        Some(LobbyRequest::LoadDeck {
            deck_id: "deck-1".to_string()
        })
    );
    assert_eq!(lobby.edit_deck(9), None, "no such deck");
}

/// A deck that arrives before the pool is held by name and resolves when
/// the pool lands — the two answers race, and neither order may lose rows.
#[test]
fn a_deck_loaded_before_the_pool_still_resolves() {
    let mut lobby = seated_lobby();
    assert_eq!(
        lobby.apply(LobbyEvent::DeckLoaded {
            id: "deck-1".to_string(),
            name: "Trees".to_string(),
            cards: vec!["3 Forest".to_string()],
            sideboard: vec![],
            commanders: Vec::new(),
        }),
        Some(LobbyRequest::LoadPool),
        "the rows arrived first; the pool is still needed"
    );
    assert!(
        lobby.builder().missing().is_empty(),
        "nothing is missing yet — the pool has not had its say"
    );
    lobby.apply(LobbyEvent::Pool {
        cards: vec![PoolCard {
            index: 1,
            english_name: "Forest".to_string(),
            name: "Forest".to_string(),
            kinds: vec!["Land".to_string()],
            type_line: "Basic Land — Forest".to_string(),
            basic_land: true,
            ..PoolCard::default()
        }],
        has_text: false,
    });
    assert_eq!(lobby.builder().name(), "Trees");
    assert_eq!(
        lobby.builder().counts().main,
        3,
        "the held row became a real entry once the pool arrived"
    );
    assert!(lobby.builder().missing().is_empty());
}

/// Saving from the builder goes through the builder's own rules, so a deck
/// the gateway would refuse never leaves the client.
#[test]
fn the_builder_refuses_to_save_what_the_gateway_would_reject() {
    let mut lobby = seated_lobby();
    lobby.build_deck();
    lobby.apply(LobbyEvent::Pool {
        cards: vec![PoolCard {
            index: 1,
            english_name: "Forest".to_string(),
            name: "Forest".to_string(),
            kinds: vec!["Land".to_string()],
            type_line: "Basic Land — Forest".to_string(),
            basic_land: true,
            ..PoolCard::default()
        }],
        has_text: false,
    });
    assert_eq!(lobby.save_deck(), None, "nameless and empty");
    lobby.builder_mut().set_name("Trees");
    lobby.builder_mut().add(0, Zone::Main);
    assert_eq!(
        lobby.save_deck(),
        Some(LobbyRequest::SaveDeck {
            deck_id: None,
            name: "Trees".to_string(),
            cards: vec!["1 Forest".to_string()],
            sideboard: vec![],
            commanders: Vec::new(),
        })
    );
    assert_eq!(
        lobby.apply(LobbyEvent::DeckSaved {
            deck_id: Some("d9".to_string())
        }),
        Some(LobbyRequest::ListDecks)
    );
    assert!(!lobby.builder().dirty(), "saving settles the deck");
    assert_eq!(
        lobby.builder().editing(),
        Some("d9"),
        "and it is now the deck being edited"
    );
    // So a second save edits that deck rather than filing a copy of it.
    // (Through the list refresh the save kicked off, which is what frees
    // the lobby to send anything at all.)
    lobby.apply(LobbyEvent::Decks(vec![]));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    lobby.builder_mut().set_name("Trees II");
    assert_eq!(
        lobby.save_deck(),
        Some(LobbyRequest::SaveDeck {
            deck_id: Some("d9".to_string()),
            name: "Trees II".to_string(),
            cards: vec!["1 Forest".to_string()],
            sideboard: vec![],
            commanders: Vec::new(),
        })
    );
}

/// Deleting a deck re-reads the list, or the one that is gone stays on
/// screen until something else happens to refresh it.
#[test]
fn deleting_a_deck_re_reads_the_list() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
        id: "deck-1".to_string(),
        name: "Burn".to_string(),
        cards: 2,
        sideboard: 0,
        commanders: Vec::new(),
    }]));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(
        lobby.delete_deck(0),
        Some(LobbyRequest::DeleteDeck {
            deck_id: "deck-1".to_string()
        })
    );
    assert_eq!(
        lobby.apply(LobbyEvent::DeckDeleted),
        Some(LobbyRequest::ListDecks)
    );
}

#[test]
fn issue_186_history_preview_preserves_edits_and_restore_is_explicit() {
    use crate::lobby::library::{History, Reply, Request, Revision, Snapshot};
    let mut lobby = seated_lobby();
    lobby.builder_mut().load(
        "d1",
        "test",
        &["10 Forest".into()],
        &["2 Island".into()],
        &[],
    );
    let working_main = lobby.builder().rows(Zone::Main);
    let working_side = lobby.builder().rows(Zone::Side);
    assert_eq!(
        lobby.browse_history(),
        Some(LobbyRequest::Library(Request::History("d1".into())))
    );
    assert_eq!(
        lobby.apply(LobbyEvent::Library(Reply::History(
            "d1".into(),
            History {
                version: 3,
                updated_at: 100,
                past: vec![Revision {
                    version: 1,
                    summary: Some("first".into()),
                    superseded_at: 90,
                    cards: 1,
                    sideboard: 1
                }],
            }
        ))),
        Some(LobbyRequest::Library(Request::Version("d1".into(), 3)))
    );
    lobby.apply(LobbyEvent::Library(Reply::Version(
        "d1".into(),
        Snapshot {
            version: 3,
            cards: working_main.clone(),
            sideboard: working_side.clone(),
            commanders: vec![],
        },
    )));
    assert_eq!(
        lobby.restore_preview(),
        None,
        "current version cannot be restored"
    );
    assert_eq!(lobby.preview_version("another-account", 1), None);
    assert!(lobby.preview_version("d1", 1).is_some());
    lobby.apply(LobbyEvent::Library(Reply::Version(
        "d1".into(),
        Snapshot {
            version: 1,
            cards: vec!["4 Forest".into()],
            sideboard: vec!["1 Island".into()],
            commanders: vec![],
        },
    )));
    assert_eq!(lobby.builder().rows(Zone::Main), working_main);
    assert_eq!(lobby.builder().rows(Zone::Side), working_side);
    assert_eq!(lobby.restore_preview(), None, "first click only confirms");
    assert!(lobby.library().confirm_restore);
    assert_eq!(
        lobby.restore_preview(),
        Some(LobbyRequest::Library(Request::Restore("d1".into(), 1)))
    );
    assert_eq!(lobby.restore_preview(), None, "double submission refused");
    assert_eq!(
        lobby.apply(LobbyEvent::Library(Reply::Restored("d1".into()))),
        Some(LobbyRequest::LoadDeck {
            deck_id: "d1".into()
        })
    );
    assert!(lobby.library().page.is_none());
}

#[test]
fn issue_186_house_copy_gets_its_own_identity_and_history_membership() {
    use crate::lobby::library::{HouseDeck, Reply, Request};
    let mut lobby = seated_lobby();
    assert!(lobby.browse_house().is_some());
    lobby.apply(LobbyEvent::Library(Reply::House(vec![HouseDeck {
        id: "shared".into(),
        name: "House".into(),
        format: "commander".into(),
        description: String::new(),
        version: 2,
        cards: 1,
        sideboard: 1,
        commanders: vec![],
    }])));
    assert_eq!(
        lobby.copy_house(0),
        Some(LobbyRequest::Library(Request::Copy("shared".into())))
    );
    assert_eq!(lobby.copy_house(0), None);
    assert_eq!(
        lobby.apply(LobbyEvent::Library(Reply::Copied("private-copy".into()))),
        Some(LobbyRequest::ListDecks)
    );
    assert_eq!(
        lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "private-copy".into(),
            name: "House".into(),
            cards: 1,
            sideboard: 1,
            commanders: vec![]
        }])),
        Some(LobbyRequest::LoadDeck {
            deck_id: "private-copy".into()
        })
    );
    lobby
        .builder_mut()
        .load("private-copy", "House", &["10 Forest".into()], &[], &[]);
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert!(lobby.browse_history().is_some());
    lobby.sign_out();
    lobby.apply(LobbyEvent::Library(Reply::House(vec![])));
    assert!(
        lobby.library().page.is_none(),
        "late response cannot reopen an account screen"
    );
    assert!(offline_lobby().browse_house().is_some());
    assert_eq!(offline_lobby().browse_history(), None);
}

#[test]
fn issue_186_diff_counts_quantities_and_preserves_print_and_sideboard_changes() {
    use crate::lobby::library::row_changes;
    assert_eq!(
        row_changes(
            &["4 Forest".into(), "1 Island".into()],
            &["2 Forest".into(), "3 Island".into()]
        ),
        vec![("Forest".into(), -2), ("Island".into(), 2)]
    );
    assert!(
        row_changes(
            &["2 Forest".into(), "2 Forest".into()],
            &["4 Forest".into()]
        )
        .is_empty()
    );
    assert_eq!(
        row_changes(&["1 Forest [SET:1]".into()], &["1 Forest [SET:2]".into()]).len(),
        2
    );
    assert_eq!(
        row_changes(&[], &["2 Negate".into()]),
        vec![("Negate".into(), 2)]
    );
}
