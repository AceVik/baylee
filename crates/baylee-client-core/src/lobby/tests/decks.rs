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

/// #270: `/pool` answers a session, so a sign-out forgets the pool. A
/// signed-out lobby then holds none that a language picked on the front
/// door would ask for again without one, and the next session, perhaps at
/// another gateway, asks for its own.
#[test]
fn signing_out_forgets_the_pool() {
    let mut lobby = seated_lobby();
    assert_eq!(lobby.build_deck(), Some(LobbyRequest::LoadPool));
    lobby.apply(LobbyEvent::Pool {
        cards: vec![PoolCard {
            index: 1,
            english_name: "Forest".to_string(),
            name: "Forest".to_string(),
            ..PoolCard::default()
        }],
        has_text: false,
    });
    let drawn = lobby.builder().pool_revision();
    lobby.sign_out();
    assert!(!lobby.builder().loaded(), "signed out, no pool is held");
    lobby.apply(LobbyEvent::LoggedIn {
        token: "another".to_string(),
        username: None,
    });
    assert_eq!(
        lobby.build_deck(),
        Some(LobbyRequest::LoadPool),
        "the next session asks for its own"
    );
    lobby.apply(LobbyEvent::Pool {
        cards: vec![PoolCard {
            index: 2,
            english_name: "Island".to_string(),
            name: "Island".to_string(),
            ..PoolCard::default()
        }],
        has_text: false,
    });
    assert_ne!(
        lobby.builder().pool_revision(),
        drawn,
        "nothing that drew the old pool takes the new one for it"
    );
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
        ..Default::default()
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
        ..Default::default()
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
    // The builder's door: the sheet stands over `Screen::Build`.
    lobby.build_deck();
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
                    sideboard: 1,
                    delta: None,
                    card_count: None,
                }],
                delta: None,
                card_count: None,
            }
        ))),
        Some(LobbyRequest::Library(Request::Version("d1".into(), 3)))
    );
    // The head is selected and compared with the one before it, so that
    // one is read next.
    assert_eq!(
        lobby.apply(LobbyEvent::Library(Reply::Version(
            "d1".into(),
            Snapshot {
                version: 3,
                cards: working_main.clone(),
                sideboard: working_side.clone(),
                commanders: vec![],
            },
        ))),
        Some(LobbyRequest::Library(Request::Version("d1".into(), 1)))
    );
    lobby.apply(LobbyEvent::Library(Reply::Version(
        "d1".into(),
        Snapshot {
            version: 1,
            cards: vec!["4 Forest".into()],
            sideboard: vec!["1 Island".into()],
            commanders: vec![],
        },
    )));
    assert_eq!(lobby.compared(), Some((Some(1), 3)));
    assert_eq!(
        lobby.restore_version(),
        None,
        "current version cannot be restored"
    );
    assert_eq!(lobby.preview_version("another-account", 1), None);
    assert_eq!(
        lobby.preview_version("d1", 1),
        None,
        "both ends are read already"
    );
    assert_eq!(lobby.library().selected, Some(1));
    assert_eq!(
        lobby.compared(),
        Some((None, 1)),
        "the oldest has no before"
    );
    assert_eq!(lobby.builder().rows(Zone::Main), working_main);
    assert_eq!(lobby.builder().rows(Zone::Side), working_side);
    assert_eq!(
        lobby.restore_version(),
        Some(LobbyRequest::Library(Request::Restore("d1".into(), 1)))
    );
    assert_eq!(lobby.restore_version(), None, "double submission refused");
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
        ..Default::default()
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
            commanders: vec![],
            ..Default::default()
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
    use crate::lobby::library::{Change, Snapshot, compare_snapshots};
    let snap = |cards: &[&str], side: &[&str]| Snapshot {
        version: 1,
        cards: cards.iter().map(|r| (*r).to_string()).collect(),
        sideboard: side.iter().map(|r| (*r).to_string()).collect(),
        commanders: vec![],
    };
    let [main, side, commanders] = compare_snapshots(
        &snap(&["4 Forest", "1 Island"], &[]),
        &snap(&["2 Forest", "3 Island"], &["2 Negate"]),
    );
    assert_eq!(main.changes.len(), 2);
    assert!(
        main.changes
            .iter()
            .all(|c| matches!(c, Change::Count { .. }))
    );
    assert!(matches!(
        side.changes.as_slice(),
        [Change::Added { count: 2, .. }]
    ));
    assert!(commanders.changes.is_empty());
    // A foil change is one Finish row — never −4 and +4 of the same card,
    // as the old `row_changes` drew it.
    let [main, ..] = compare_snapshots(
        &snap(&["4 Lightning Bolt (M11) 149"], &[]),
        &snap(&["4 Lightning Bolt (M11) 149 *F*"], &[]),
    );
    assert!(
        matches!(main.changes.as_slice(), [Change::Finish { count: 4, .. }]),
        "{:?}",
        main.changes
    );
    // Another printing is one Printing row.
    let [main, ..] = compare_snapshots(
        &snap(&["1 Forest (SET) 1"], &[]),
        &snap(&["1 Forest (SET) 2"], &[]),
    );
    assert!(
        matches!(main.changes.as_slice(), [Change::Printing { .. }]),
        "{:?}",
        main.changes
    );
}

/// The compare bar (`DESIGN` §C.3): Previous by default, Current against
/// the head, Pick… waits for a second version from the list, and Esc (or a
/// second Pick…) leaves the mode for Previous again.
#[test]
fn the_history_compare_bar_follows_previous_current_and_pick() {
    use crate::lobby::library::{Compare, History, Reply, Request, Revision};
    let mut lobby = seated_lobby();
    lobby.browse_deck_history("d1");
    let past = |version| Revision {
        version,
        summary: None,
        superseded_at: i64::from(version) * 10,
        cards: 0,
        sideboard: 0,
        delta: None,
        card_count: None,
    };
    lobby.apply(LobbyEvent::Library(Reply::History(
        "d1".into(),
        History {
            version: 12,
            updated_at: 200,
            past: vec![past(11), past(10), past(9)],
            delta: None,
            card_count: None,
        },
    )));
    assert_eq!(lobby.compared(), Some((Some(11), 12)));
    // ↓ ×2 selects v10: compared with v9.
    lobby.library.loading = false;
    lobby.step_version(1);
    lobby.library.loading = false;
    lobby.step_version(1);
    assert_eq!(lobby.library().selected, Some(10));
    assert_eq!(lobby.compared(), Some((Some(9), 10)));
    lobby.library.loading = false;
    assert_eq!(
        lobby.compare_with(Compare::Current),
        Some(LobbyRequest::Library(Request::Version("d1".into(), 12)))
    );
    assert_eq!(lobby.compared(), Some((Some(10), 12)));
    lobby.library.loading = false;
    lobby.compare_with(Compare::Pick(None));
    assert_eq!(lobby.library().compare, Compare::Pick(None));
    // While the list waits, a press picks the other end, not the selection.
    lobby.library.loading = false;
    lobby.preview_version("d1", 9);
    assert_eq!(lobby.library().selected, Some(10));
    assert_eq!(lobby.compared(), Some((Some(9), 10)));
    assert!(lobby.leave_pick(), "Esc leaves the mode");
    assert_eq!(lobby.library().compare, Compare::Previous);
    assert!(!lobby.leave_pick(), "a second Esc is the sheet's");
    // A second Pick… leaves the mode too.
    lobby.compare_with(Compare::Pick(None));
    lobby.compare_with(Compare::Pick(None));
    assert_eq!(lobby.library().compare, Compare::Previous);
    // The oldest version reads `first save`: nothing before it.
    let history = lobby.library().history.clone().unwrap();
    assert_eq!(history.started(9), None);
    assert_eq!(history.started(10), Some(90));
    assert_eq!(history.started(12), Some(110));
}

fn two_decks(lobby: &mut Lobby, first: &str, second: &str) {
    lobby.apply(LobbyEvent::Decks(
        [first, second]
            .into_iter()
            .map(|id| DeckSummary {
                id: id.into(),
                name: id.into(),
                ..Default::default()
            })
            .collect(),
    ));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
}

/// The next game's deck follows its deck when a save reorders the list:
/// the list is newest save first, so an index would point at another deck.
#[test]
fn the_next_game_deck_follows_its_deck_through_a_reordered_list() {
    let mut lobby = seated_lobby();
    two_decks(&mut lobby, "a", "b");
    lobby.select_deck(1);
    assert_eq!(lobby.next_deck().map(|d| d.id.as_str()), Some("b"));
    two_decks(&mut lobby, "b", "a");
    assert_eq!(lobby.next_deck().map(|d| d.id.as_str()), Some("b"));
}

/// Delete is an Undo toast, not a confirm (S-10): the deck leaves the shelf
/// at once, nothing is sent until the Undo runs out, and Undo sends
/// nothing at all.
#[test]
fn a_staged_delete_sends_nothing_until_it_is_flushed_and_undo_sends_nothing() {
    let mut lobby = seated_lobby();
    two_decks(&mut lobby, "a", "b");
    assert_eq!(lobby.stage_delete(0), None);
    assert_eq!(lobby.staged_delete(), Some("a"));
    assert_eq!(lobby.next_deck().map(|d| d.id.as_str()), Some("b"));
    lobby.undo_delete();
    assert_eq!(lobby.flush_delete(), None, "undone: nothing to send");
    lobby.stage_delete(0);
    // A second delete flushes the first.
    assert_eq!(
        lobby.stage_delete(1),
        Some(LobbyRequest::DeleteDeck {
            deck_id: "a".into()
        })
    );
    assert_eq!(
        lobby.flush_delete(),
        Some(LobbyRequest::DeleteDeck {
            deck_id: "b".into()
        })
    );
    assert_eq!(lobby.flush_delete(), None, "sent once");
}

/// **Add and use** copies a house deck and makes the copy the next game's
/// deck, without opening the builder (S-3).
#[test]
fn add_and_use_selects_the_copy_and_stays_off_the_builder() {
    use crate::lobby::library::{AfterCopy, HouseDeck, Reply, Request};
    let mut lobby = seated_lobby();
    lobby.browse_house();
    lobby.apply(LobbyEvent::Library(Reply::House(vec![HouseDeck {
        id: "shared".into(),
        name: "House".into(),
        version: 1,
        ..Default::default()
    }])));
    assert_eq!(
        lobby.copy_house_then(0, AfterCopy::Use),
        Some(LobbyRequest::Library(Request::Copy("shared".into())))
    );
    assert_eq!(
        lobby.apply(LobbyEvent::Library(Reply::Copied("copy".into()))),
        Some(LobbyRequest::ListDecks)
    );
    assert_eq!(lobby.screen(), &Screen::Table, "no builder");
    two_decks(&mut lobby, "d1", "copy");
    assert_eq!(lobby.next_deck().map(|d| d.id.as_str()), Some("copy"));
}

/// Duplicate copies one of the player's own decks and stays on the shelf.
#[test]
fn a_duplicate_is_a_copy_that_stays_on_the_shelf() {
    use crate::lobby::library::{Reply, Request};
    let mut lobby = seated_lobby();
    assert_eq!(
        lobby.duplicate_deck(0),
        Some(LobbyRequest::Library(Request::Copy("d1".into())))
    );
    assert_eq!(
        lobby.apply(LobbyEvent::Library(Reply::Copied("d2".into()))),
        Some(LobbyRequest::ListDecks)
    );
    assert_eq!(lobby.screen(), &Screen::Table);
    assert!(lobby.library().page.is_none());
}

/// Restoring from the Decks screen's history stays on the shelf and offers
/// the head it replaced for Undo.
#[test]
fn a_restore_from_the_shelf_stays_there_and_can_be_undone() {
    use crate::lobby::library::{History, Reply, Request, Revision, Snapshot};
    let mut lobby = seated_lobby();
    assert!(lobby.browse_deck_history("d1").is_some());
    let snapshot = |version| Snapshot {
        version,
        cards: vec![],
        sideboard: vec![],
        commanders: vec![],
    };
    lobby.apply(LobbyEvent::Library(Reply::History(
        "d1".into(),
        History {
            version: 3,
            updated_at: 0,
            past: vec![Revision {
                version: 2,
                summary: None,
                superseded_at: 0,
                cards: 0,
                sideboard: 0,
                delta: None,
                card_count: None,
            }],
            delta: None,
            card_count: None,
        },
    )));
    lobby.apply(LobbyEvent::Library(Reply::Version(
        "d1".into(),
        snapshot(3),
    )));
    lobby.apply(LobbyEvent::Library(Reply::Version(
        "d1".into(),
        snapshot(2),
    )));
    assert_eq!(lobby.preview_version("d1", 2), None);
    assert_eq!(
        lobby.restore_version(),
        Some(LobbyRequest::Library(Request::Restore("d1".into(), 2)))
    );
    assert_eq!(
        lobby.apply(LobbyEvent::Library(Reply::Restored("d1".into()))),
        Some(LobbyRequest::ListDecks)
    );
    assert_eq!(lobby.screen(), &Screen::Table);
    assert_eq!(lobby.library().restored, Some(("d1".into(), 3)));
    assert_eq!(
        lobby.undo_restore(),
        Some(LobbyRequest::Library(Request::Restore("d1".into(), 3)))
    );
    assert_eq!(lobby.library().restored, None, "an Undo is not undone");
}

/// The deck list's way into a deck opens the builder on the deck, never on
/// a page the Decks screen left open (the owner's beta.6 review: "deck list
/// → edit deck shows an old page"). The house list stays open behind its
/// tab, and a first run with no decks asks for it unprompted; a deck's
/// history is a sheet there. The shell draws the builder's own history
/// page over `Screen::Build` whenever any page is open.
#[test]
fn the_builder_opens_on_the_deck_not_on_a_page_the_decks_screen_left_open() {
    use crate::lobby::library::{HouseDeck, Page, Reply};
    type Open = fn(&mut Lobby) -> Option<LobbyRequest>;
    let doors: [(&str, Open); 2] = [
        ("edit", |lobby| lobby.edit_deck(0)),
        ("new", Lobby::build_deck),
    ];
    for (door, open) in doors {
        for page in ["house", "history"] {
            let mut lobby = seated_lobby();
            if page == "house" {
                assert!(lobby.browse_house().is_some());
                lobby.apply(LobbyEvent::Library(Reply::House(vec![HouseDeck {
                    id: "shared".into(),
                    name: "House".into(),
                    version: 1,
                    ..Default::default()
                }])));
                assert_eq!(lobby.library().page, Some(Page::House));
            } else {
                assert!(lobby.browse_deck_history("d1").is_some());
                assert!(lobby.library().page.is_some());
            }
            open(&mut lobby);
            assert_eq!(lobby.screen(), &Screen::Build, "{door} from {page}");
            assert_eq!(lobby.library().page, None, "{door} from {page}");
            assert!(!lobby.library().loading, "{door} from {page}");
            // And the house tab, come back to, asks again.
            lobby.close_builder();
            assert!(lobby.browse_house().is_some(), "{door} from {page}");
        }
    }
}

/// The store writes a save's summary onto the row it replaced (`put_deck`),
/// so "restored v1" for v3 stands on v2's row. Each version reads its words
/// off its predecessor's row: the live walk found every restore named one
/// row early and the head (v4, an Undo) never named.
#[test]
fn a_versions_summary_is_the_one_its_predecessors_row_keeps() {
    use crate::lobby::library::{History, Revision};
    let row = |version, summary: Option<&str>| Revision {
        version,
        summary: summary.map(str::to_string),
        superseded_at: i64::from(version) * 10,
        cards: 0,
        sideboard: 0,
        delta: None,
        card_count: None,
    };
    // v1 saved; v2 saved; v3 restored v1; v4 restored v2 (an Undo).
    let history = History {
        version: 4,
        updated_at: 40,
        past: vec![
            row(3, Some("back to version 2")),
            row(2, Some("back to version 1")),
            row(1, None),
        ],
        delta: None,
        card_count: None,
    };
    assert_eq!(history.summary(4), Some("back to version 2"), "the head");
    assert_eq!(history.summary(3), Some("back to version 1"));
    assert_eq!(history.summary(2), None, "an ordinary save");
    assert_eq!(history.summary(1), None, "the oldest kept");
}

/// A house deck's history opens from the House tab, read-only (Q-C4): the
/// list is kept under the sheet, restore is refused, and closing the sheet
/// stands the list up again rather than an empty tab.
#[test]
fn a_house_decks_history_is_read_only_and_closes_back_to_the_list() {
    use crate::lobby::library::{HouseDeck, Page, Reply, Request};
    let mut lobby = seated_lobby();
    lobby.browse_house();
    lobby.apply(LobbyEvent::Library(Reply::House(vec![HouseDeck {
        id: "shared".into(),
        name: "House".into(),
        version: 2,
        ..Default::default()
    }])));
    assert_eq!(
        lobby.browse_deck_history("shared"),
        Some(LobbyRequest::Library(Request::History("shared".into())))
    );
    assert!(
        lobby.house_deck("shared"),
        "the sheet knows it for a house deck"
    );
    assert_eq!(lobby.library().house.len(), 1);
    assert_eq!(lobby.restore_version(), None, "read-only");
    lobby.close_library();
    assert_eq!(lobby.library().page, Some(Page::House));
    assert_eq!(lobby.library().house.len(), 1, "the list stands again");
    // Nobody's deck and no house deck: no history.
    assert_eq!(lobby.browse_deck_history("elsewhere"), None);
}
