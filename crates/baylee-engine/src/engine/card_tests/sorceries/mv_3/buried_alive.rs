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
