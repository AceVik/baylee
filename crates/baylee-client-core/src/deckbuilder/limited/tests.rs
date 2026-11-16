use super::*;
use crate::deckbuilder::{Coverage, PoolCard};

fn editor() -> DeckBuilder {
    let names = ["General Tazri", "Plains", "Sol Ring", "Island"];
    let pool: Vec<_> = names
        .iter()
        .map(|name| {
            let index = baylee_cards::decks::by_name(name).unwrap();
            PoolCard {
                index: index.get(),
                name: (*name).into(),
                english_name: (*name).into(),
                commander: *name == "General Tazri",
                basic_land: matches!(*name, "Plains" | "Island"),
                coverage: Coverage::Implemented,
                ..PoolCard::default()
            }
        })
        .collect();
    let stock = pool
        .iter()
        .map(|card| {
            (
                card.index,
                Holding {
                    copies: if card.basic_land { u16::MAX } else { 1 },
                    print: PrintChoice {
                        finish: Some(baylee_core::preset::Finish::Foil),
                        ..PrintChoice::default()
                    },
                },
            )
        })
        .collect();
    let mut builder = DeckBuilder::new();
    builder.set_pool(pool, false);
    builder.load(
        "run-only",
        "Tower",
        &[
            "1 General Tazri".into(),
            "38 Plains".into(),
            "1 Sol Ring".into(),
        ],
        &[],
        &["General Tazri".into()],
    );
    builder.limit_to(Commander {
        total: 40,
        side: 5,
        stock,
    });
    builder
}

#[test]
fn limited_editor_uses_forty_card_identity_checks_and_never_emits_library_saves() {
    let mut builder = editor();
    assert_eq!(builder.standing(), baylee_cardtext::Standing::Legal);
    assert!(builder.saveable());
    assert!(builder.save().is_none());
    let ring = builder.slot_of("Sol Ring").unwrap();
    let plains = builder.slot_of("Plains").unwrap();
    assert!(!builder.add(ring, Zone::Side), "ownership spans both zones");
    assert!(
        !builder.add(plains, Zone::Main),
        "exactly forty including commander"
    );
    assert!(builder.remove(plains, Zone::Main));
    assert!(!builder.saveable(), "thirty-nine cannot be submitted");
    assert!(builder.add(plains, Zone::Main));
    for _ in 0..5 {
        assert!(builder.add(plains, Zone::Side));
    }
    assert!(!builder.add(plains, Zone::Side));
    let composition = builder.composition();
    assert!(!builder.move_entry(0, Zone::Main, Zone::Side));
    assert!(!builder.move_entry(0, Zone::Side, Zone::Main));
    assert_eq!(
        builder.composition(),
        composition,
        "a full destination loses no card"
    );
    assert!(builder.saveable());
    assert!(
        builder
            .entries(Zone::Main)
            .iter()
            .chain(builder.entries(Zone::Side))
            .all(|row| row.print.finish == Some(baylee_core::preset::Finish::Foil))
    );
    let (main, side, commanders) = builder.composition();
    assert_eq!((main.len(), side.len(), commanders.len()), (39, 5, 1));
    builder.set_pool(Vec::new(), false);
    assert_eq!(
        builder.pool().len(),
        4,
        "late pool reply cannot replace collection"
    );
    assert!(builder.open_picker(ring, Zone::Main).is_none());
    let picker = builder.picker().unwrap();
    assert!(!picker.loading());
    assert_eq!(picker.finish(), baylee_core::preset::Finish::Foil);
    builder.picker_set_finish(baylee_core::preset::Finish::Normal);
    assert_eq!(
        builder.picker().unwrap().finish(),
        baylee_core::preset::Finish::Foil
    );
    assert!(builder.refresh_printings().is_none());
    let held = builder.picker().unwrap().clone();
    builder.set_printings(
        builder.card(ring).unwrap().index,
        vec![super::super::Printing::default()],
        true,
    );
    assert_eq!(
        builder.picker(),
        Some(&held),
        "late library metadata cannot replace a run printing"
    );
}

#[test]
fn scoped_lobby_preserves_normal_editor_and_late_gateway_pool_targets_the_library() {
    let mut lobby = crate::lobby::Lobby::default();
    lobby.builder_mut().set_name("Ordinary unsaved deck");
    lobby.open_scoped_builder(editor());
    assert_eq!(lobby.builder().name(), "Tower");
    let _ = lobby.apply(crate::lobby::LobbyEvent::Pool {
        cards: Vec::new(),
        has_text: false,
    });
    assert_eq!(lobby.builder().pool().len(), 4);
    assert!(lobby.close_scoped_builder());
    assert_eq!(lobby.builder().name(), "Ordinary unsaved deck");
    assert!(lobby.builder().dirty());
    assert!(!lobby.close_scoped_builder());
}

#[test]
fn limited_check_refuses_off_color_and_unowned_cards_even_in_a_loaded_composition() {
    let original = editor();
    let mut limits = original.limited().unwrap().clone();
    let mut pool = original.pool().to_vec();
    let leader = baylee_cards::decks::by_name("Isamaru, Hound of Konda").unwrap();
    pool[0].index = leader.get();
    pool[0].name = "Isamaru, Hound of Konda".into();
    pool[0].english_name = pool[0].name.clone();
    limits.stock.insert(
        leader.get(),
        Holding {
            copies: 1,
            print: PrintChoice::default(),
        },
    );
    let mut builder = DeckBuilder::new();
    builder.set_pool(pool, false);
    builder.load(
        "run",
        "Tower",
        &[
            "1 Isamaru, Hound of Konda".into(),
            "37 Plains".into(),
            "1 Island".into(),
            "1 Sol Ring".into(),
        ],
        &[],
        &["Isamaru, Hound of Konda".into()],
    );
    builder.limit_to(limits.clone());
    assert!(!builder.saveable());
    assert!(
        builder
            .report()
            .problems
            .iter()
            .any(|problem| matches!(problem, Problem::OutsideIdentity { .. }))
    );
    let island = builder.slot_of("Island").unwrap();
    let plains = builder.slot_of("Plains").unwrap();
    assert!(builder.remove(island, Zone::Main));
    assert!(builder.add(plains, Zone::Main));
    assert!(builder.saveable());
    limits
        .stock
        .remove(&baylee_cards::decks::by_name("Sol Ring").unwrap().get());
    builder.limit_to(limits);
    assert!(!builder.saveable());
    assert!(
        builder
            .report()
            .problems
            .iter()
            .any(|problem| matches!(problem, Problem::TooManyCopies { limit: 0, .. }))
    );
}
