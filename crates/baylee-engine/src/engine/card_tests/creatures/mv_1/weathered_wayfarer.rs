//! `cards/creatures/mv_1/weathered_wayfarer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{LIGHTNING_BOLT, RIPTIDE_LABORATORY};

fn subject() -> CardIndex {
    card_index("1093fdb6-bd4f-46a8-92e3-5aa46b52bb4f")
}

fn activate_wayfarer(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> bool {
    let source = on_battlefield(engine, seat, subject()).expect("Wayfarer on the battlefield");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .is_ok()
}

/// {W}, {T} searches for any land, not just a basic one. Only the chosen card
/// moves to hand; the search reveals it and shuffles the library afterwards.
#[test]
fn weathered_wayfarer_finds_and_reveals_any_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), plains()])
        .hand(0, &[RIPTIDE_LABORATORY, island(), LIGHTNING_BOLT])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let nonbasic = hand_to_library_top(&mut engine, p0, RIPTIDE_LABORATORY);
    let basic = hand_to_library_top(&mut engine, p0, island());
    let nonland = hand_to_library_top(&mut engine, p0, LIGHTNING_BOLT);
    let source = on_battlefield(&engine, p0, subject()).unwrap();
    let library_before = library_size(&engine, p0);
    tap_mana_except(&mut engine, p0, source);
    activate(&mut engine, p0, subject(), 0);
    pass_until(&mut engine, |game| {
        matches!(
            game.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the land search, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    // A search for a stated quality may fail to find (CR 701.23b).
    assert_eq!((min, max, prompt), (0, 1, ChoicePrompt::SearchLibrary));
    assert!(options.contains(&nonbasic));
    assert!(options.contains(&basic));
    assert!(!options.contains(&nonland));
    assert!(options.iter().all(|id| {
        engine
            .state()
            .object(*id)
            .is_some_and(|object| object.characteristics().types.contains(TypeSet::LAND))
    }));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![nonbasic],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, source));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(
        engine
            .state()
            .zones
            .contains(nonbasic, ZoneLocation::Hand(p0))
    );
    assert!(
        engine
            .state()
            .zones
            .contains(basic, ZoneLocation::Library(p0))
    );
    assert!(
        engine
            .state()
            .zones
            .contains(nonland, ZoneLocation::Library(p0))
    );
    assert_eq!(library_size(&engine, p0), library_before - 1);
    assert!(engine.state().journal.entries().iter().any(|entry| {
        matches!(&entry.event, crate::event::GameEvent::Revealed { player, cards }
            if *player == p0 && cards == &vec![nonbasic])
    }));
    assert!(engine.state().journal.entries().iter().any(|entry| {
        matches!(entry.event, crate::event::GameEvent::Shuffled { player, .. } if player == p0)
    }));

    // Isolate {T} from the mana cost: even with white mana floating, the
    // already tapped Wayfarer cannot activate a second time.
    engine.dev_state_mut(p0).expect("dev board").players[0]
        .mana_pool
        .add(ManaColor::White, 1);
    engine.refresh_offer();
    assert!(!activate_wayfarer(&mut engine, p0));
    assert_eq!(library_size(&engine, p0), library_before - 1);
}

/// "More" is strict: one land each is not enough, even when the activation's
/// white mana is already floating.
#[test]
fn weathered_wayfarer_needs_an_opponent_ahead_on_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), plains()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let source = on_battlefield(&engine, p0, subject()).unwrap();
    let library_before = library_size(&engine, p0);
    tap_mana_except(&mut engine, p0, source);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.abilities.contains(&(source, 0)));
    assert!(!activate_wayfarer(&mut engine, p0));
    assert!(!is_tapped(&engine, source));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );
    assert_eq!(library_size(&engine, p0), library_before);
}

/// The opponent's land advantage does not replace the printed white pip.
#[test]
fn weathered_wayfarer_needs_white_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), island()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let source = on_battlefield(&engine, p0, subject()).unwrap();
    tap_mana_except(&mut engine, p0, source);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.abilities.contains(&(source, 0)));
    assert!(!activate_wayfarer(&mut engine, p0));
    assert!(!is_tapped(&engine, source));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
}
