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
        Press::Build(BuildPress::PickerStep(-1)),
        Press::Build(BuildPress::PickerStep(1)),
        Press::Build(BuildPress::PickerGo(1)),
        Press::Build(BuildPress::PickerLang(Some(0))),
        Press::Build(BuildPress::PickerLang(Some(1))),
        Press::Build(BuildPress::PickerFinish(Finish::Foil)),
        Press::Build(BuildPress::PickerForceFinish),
        Press::Build(BuildPress::FocusBuild(BuildField::PickerSet)),
        // The card window's primary through a picture door on a pool card:
        // Add to deck, the chosen print riding along (windows-b6 §A.3).
        Press::Build(BuildPress::WindowAdd(Zone::Main)),
        Press::Build(BuildPress::WindowStep(Zone::Main, true)),
        Press::Build(BuildPress::PickerClose),
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    assert!(!found.contains(&Press::Build(BuildPress::PickerSet(Some(1)))));
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .builder_mut()
        .focus_on(BuildField::PickerSet);
    app.update();
    assert!(presses(&mut app).contains(&Press::Build(BuildPress::PickerSet(Some(1)))));
}

/// Empty space in the card window's printing strip must not carry an add
/// `Press`, while Enter on the strip's focus stop still activates its primary
/// action and thumbnails still select their printing.
#[test]
fn printing_strip_keeps_enter_primary_without_click_to_add() {
    use crate::buildui::cardwindow::CARD;
    use crate::shellkit::focus::Stop;

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        state.lobby.builder_mut().open_picker(0, Zone::Main);
        state.lobby.apply(LobbyEvent::Printings {
            card: 1,
            printings: serde_json::from_value(serde_json::json!([
                {
                    "scryfall_id": "11111111-2222-3333-4444-555555555555",
                    "oracle_id": "o", "lang": "en", "set": "m19",
                    "set_name": "Core Set 2019", "collector_number": "314",
                    "finishes": ["nonfoil"], "name": "Llanowar Elves"
                },
                {
                    "scryfall_id": "66666666-7777-8888-9999-aaaaaaaaaaaa",
                    "oracle_id": "o", "lang": "en", "set": "dom",
                    "set_name": "Dominaria", "collector_number": "168",
                    "finishes": ["nonfoil"], "name": "Llanowar Elves"
                }
            ]))
            .expect("printings decode"),
            from_catalog: true,
        });
    }
    app.update();

    let strip = {
        let world = app.world_mut();
        world
            .query::<(Entity, &Stop)>()
            .iter(world)
            .find(|(_, s)| s.table == CARD && s.id == "printings")
            .map(|(e, _)| e)
            .expect("the printings strip stop")
    };
    assert!(
        app.world().get::<Press>(strip).is_none(),
        "empty space in the strip container must not carry a clickable Press"
    );
    assert_eq!(
        app.world().get::<Primary>(strip).copied(),
        Some(Primary(Press::Build(BuildPress::WindowAdd(Zone::Main)))),
        "Enter on the strip still runs the window's primary action"
    );
    let children: Vec<Entity> = app
        .world()
        .get::<Children>(strip)
        .expect("thumbnail children")
        .iter()
        .collect();
    assert_eq!(children.len(), 2);
    assert_eq!(
        app.world().get::<Press>(children[0]).copied(),
        Some(Press::Build(BuildPress::PickerGo(0)))
    );
    assert_eq!(
        app.world().get::<Press>(children[1]).copied(),
        Some(Press::Build(BuildPress::PickerGo(1)))
    );
}

/// The card window's Set dropdown must be mounted outside the scroll-clipped
/// body and above the modal footer so clicking a matched set never hits Close.
#[test]
fn set_dropdown_stands_outside_clipped_body_and_above_footer() {
    use crate::buildui::print_picker::PickerSetPopup;

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        state.lobby.builder_mut().open_picker(0, Zone::Main);
        state.lobby.apply(LobbyEvent::Printings {
            card: 1,
            printings: serde_json::from_value(serde_json::json!([
                {
                    "scryfall_id": "11111111-2222-3333-4444-555555555555",
                    "oracle_id": "o", "lang": "en", "set": "m19",
                    "set_name": "Core Set 2019", "collector_number": "314",
                    "finishes": ["nonfoil"], "name": "Llanowar Elves"
                },
                {
                    "scryfall_id": "66666666-7777-8888-9999-aaaaaaaaaaaa",
                    "oracle_id": "o", "lang": "en", "set": "m10",
                    "set_name": "Magic 2010", "collector_number": "168",
                    "finishes": ["nonfoil"], "name": "Llanowar Elves"
                }
            ]))
            .expect("printings decode"),
            from_catalog: true,
        });
        state.lobby.builder_mut().focus_on(BuildField::PickerSet);
    }
    app.update();

    let (popup, z) = {
        let world = app.world_mut();
        world
            .query_filtered::<(Entity, &GlobalZIndex), With<PickerSetPopup>>()
            .iter(world)
            .next()
            .map(|(e, z)| (e, z.0))
            .expect("the open set popup")
    };
    assert!(
        z > crate::shellkit::tokens::z::SHEET,
        "set popup must sit above the modal sheet and its footer: {z}"
    );
    let parent = app
        .world()
        .get::<ChildOf>(popup)
        .map(ChildOf::parent)
        .expect("set popup has a parent");
    assert!(
        app.world()
            .get::<crate::shellkit::surfaces::ModalScrim>(parent)
            .is_some(),
        "set popup must mount directly on the full-window ModalScrim, outside the clipped sheet and footer"
    );
    let mut cur = Some(parent);
    while let Some(ancestor) = cur {
        if app.world().get::<LobbyRoot>(ancestor).is_some() {
            break;
        }
        if let Some(node) = app.world().get::<Node>(ancestor) {
            assert_eq!(
                node.overflow.x,
                OverflowAxis::Visible,
                "no modal or overlay ancestor may clip the set popup horizontally"
            );
            assert_eq!(
                node.overflow.y,
                OverflowAxis::Visible,
                "no modal or overlay ancestor may clip the set popup vertically"
            );
        }
        cur = app.world().get::<ChildOf>(ancestor).map(ChildOf::parent);
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
    assert!(
        found.contains(&Press::Build(BuildPress::PickPrint(0))),
        "{found:?}"
    );
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

/// Draw seven on the Stats tab must render each drawn card with the printing
/// chosen for it in the deck rather than the pool card's default printing.
#[test]
fn draw_seven_uses_the_chosen_printing_from_the_deck() {
    use crate::buildui::DeckTab;
    use crate::buildui::sample_hand::SampleHand;

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    let chosen = baylee_core::deckrow::PrintChoice {
        scryfall_id: Some("11111111-2222-3333-4444-555555555555".to_string()),
        finish: Some(Finish::Foil),
        ..baylee_core::deckrow::PrintChoice::default()
    };
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        assert!(state.lobby.builder_mut().add_print(0, Zone::Main, chosen));
        state.build.tab = DeckTab::Stats;
    }
    app.update();
    press(&mut app, Press::Build(BuildPress::DrawSeven));
    let hand = {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<SampleHand>>()
            .iter(world)
            .next()
            .expect("the sample hand row")
    };
    let children: Vec<Entity> = app
        .world()
        .get::<Children>(hand)
        .expect("drawn thumbnails")
        .iter()
        .collect();
    assert_eq!(children.len(), 1);
    let hover = app
        .world()
        .get::<HoverCard>(children[0])
        .expect("thumbnail hover");
    assert!(
        hover
            .url
            .as_deref()
            .is_some_and(|u| u.contains("11111111-2222-3333-4444-555555555555")),
        "expected chosen printing url, got {:?}",
        hover.url
    );
    assert_eq!(hover.finish, FinishTreatment::Foil);
}
