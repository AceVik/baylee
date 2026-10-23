//! `cards/sorceries/mv_3/buried_alive.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn buried_alive() -> CardIndex {
    card_index("8203c621-a1a0-4865-8c9a-0d4064c86107")
}

/// `Buried Alive` ({2}{B}, Sorcery) searches your library for up to three
/// creature cards and puts them into your graveyard.
#[test]
fn buried_alive_searches_library_for_up_to_three_creatures_to_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, GRIZZLY_BEARS)
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[buried_alive()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, buried_alive());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards");
    };
    let picks: Vec<_> = options.into_iter().take(3).collect();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: picks })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        4
    );
}

/// "Up to three": the question allows at most three, refuses four, and takes
/// two; the library then loses exactly the two it gave up and is shuffled.
#[test]
fn buried_alive_takes_up_to_three_and_shuffles() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, GRIZZLY_BEARS)
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[buried_alive()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library = library_size(&engine, p0);
    let since = engine.state().journal.last_seq();

    cast_from_hand(&mut engine, p0, buried_alive());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, max, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched");
    };
    assert_eq!(max, 3, "at most three");
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: options.iter().copied().take(4).collect(),
                },
            )
            .is_err(),
        "four is one too many"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options.iter().copied().take(2).collect(),
            },
        )
        .expect("up to three includes two");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        3,
        "Buried Alive and the two creatures"
    );
    assert_eq!(library_size(&engine, p0), library - 2);
    assert!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .skip(since as usize)
            .any(|e| matches!(e.event, crate::event::GameEvent::Shuffled { player, .. } if player == p0)),
        "then shuffle"
    );
}

/// "Creature cards": a library of Forests holds none, so nothing reaches the
/// graveyard but the spell, whatever the search is asked.
#[test]
fn buried_alive_finds_nothing_in_a_library_without_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[buried_alive()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, buried_alive());
    assert!(matches!(drive_to_rest(&mut engine, p0), Rest::Reached));

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "only Buried Alive itself"
    );
    assert_eq!(library_size(&engine, p0), library, "no Forest was taken");
}
