//! Which piece of cardboard a row names, and what that makes it show. The picker is a dialog laid over the whole builder, so every control it draws has to be reachable — a carousel with no way to move it is a dialog a player is stuck in — and the pool row that opens it is the only door to the feature at all. The preview is the other half and is decided without an app: a deck row shows the printing it was given and falls back to the card's own id when the choice narrowed only by set, a pool row shows plainly because no finish has been chosen there, and the nil id previews nothing rather than fetching a certain 404.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// The picker is a dialog over the whole builder, and every control it
/// offers has to be reachable — a carousel with no way to move it, or a
/// finish with no way to choose it, is a dialog a player is stuck in.
#[test]
fn the_printing_picker_offers_every_control_it_needs() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        let asked = state.lobby.builder_mut().open_picker(0, Zone::Main);
        assert_eq!(asked, Some(LobbyRequest::LoadPrintings { card: 1 }));
        state.lobby.apply(LobbyEvent::Printings {
            card: 1,
            printings: serde_json::from_value(serde_json::json!([
                {
                    "scryfall_id": "11111111-2222-3333-4444-555555555555",
                    "oracle_id": "o", "lang": "en", "set": "m19",
                    "set_name": "Core Set 2019", "collector_number": "314",
                    "finishes": ["nonfoil", "foil"], "name": "Llanowar Elves"
                },
                {
                    "scryfall_id": "66666666-7777-8888-9999-aaaaaaaaaaaa",
                    "oracle_id": "o", "lang": "de", "set": "dom",
                    "set_name": "Dominaria", "collector_number": "168",
                    "finishes": ["nonfoil"], "name": "Elfen von Llanowar"
                }
            ]))
            .expect("printings decode"),
            from_catalog: true,
        });
    }
    app.update();
    let found = presses(&mut app);
    for wanted in [
        Press::PickerStep(-1),
        Press::PickerStep(1),
        Press::PickerGo(1),
        Press::PickerLang(None),
        Press::PickerLang(Some(0)),
        Press::PickerLang(Some(1)),
        Press::PickerFinish(Finish::Foil),
        Press::PickerRefresh,
        Press::PickerForceFinish,
        Press::PickerSet(Some(1)),
        Press::PickerConfirm,
        Press::PickerClose,
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
}

/// The row the pool draws is the one that opens the picker; without it
/// the whole feature is unreachable from the builder.
#[test]
fn a_pool_row_offers_a_way_to_choose_its_printing() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
    }
    app.update();
    let found = presses(&mut app);
    assert!(found.contains(&Press::PickPrint(0)), "{found:?}");
}

/// A deck row previews the printing it names, not the one the registry
/// happens to point at — that is the whole point of having chosen one.
#[test]
fn a_deck_row_previews_the_printing_it_names() {
    use baylee_client_core::deckbuilder::PoolCard;
    let card = PoolCard {
        scryfall_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
        ..PoolCard::default()
    };
    let chosen = baylee_core::deckrow::PrintChoice {
        scryfall_id: Some("11111111-2222-3333-4444-555555555555".to_string()),
        lang: Some("de".to_string()),
        finish: Some(Finish::Foil),
        ..baylee_core::deckrow::PrintChoice::default()
    };
    let hover = hover_of_entry(&card, &chosen);
    let url = hover.url.expect("a real id has art");
    assert!(
        url.contains("11111111-2222-3333-4444-555555555555"),
        "{url}"
    );
    assert_eq!(hover.finish, FinishTreatment::Foil);

    // A row that only narrowed by set has no id of its own and falls
    // back to the card's, rather than previewing nothing at all.
    let vague = baylee_core::deckrow::PrintChoice {
        set: Some("M11".to_string()),
        ..baylee_core::deckrow::PrintChoice::default()
    };
    let fallback = hover_of_entry(&card, &vague).url.expect("falls back");
    assert!(
        fallback.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
        "{fallback}"
    );
}

/// The pool's own rows preview plainly: a player has not chosen a finish
/// there, and showing one would be inventing a choice.
#[test]
fn a_pool_row_previews_plainly() {
    use baylee_client_core::deckbuilder::PoolCard;
    let card = PoolCard {
        scryfall_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
        ..PoolCard::default()
    };
    assert_eq!(hover_of_card(&card).finish, FinishTreatment::Plain);
    assert!(hover_of_card(&card).url.is_some());
}

/// A card with no usable printing must preview nothing rather than fetch
/// a guaranteed 404 — the nil id is what a preset carries.
#[test]
fn a_card_with_no_printing_previews_nothing() {
    use baylee_client_core::deckbuilder::PoolCard;
    let card = PoolCard {
        scryfall_id: "00000000-0000-0000-0000-000000000000".to_string(),
        ..PoolCard::default()
    };
    assert!(hover_of_card(&card).url.is_none());
}
