//! `cards/artifacts/mv_1/feldon_s_cane.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Feldon's Cane: "{T}, Exile this artifact: Shuffle your graveyard into
/// your library."
///
/// Both costs land before the effect: the Cane taps and is exiled, and the
/// graveyard the three seeded cards filled is moved into the library and
/// shuffled — which is `GameEvent::Shuffled`, not the same cards merely
/// stacked back on top.
#[test]
fn feldon_s_cane_exiles_itself_to_shuffle_the_graveyard_back_in() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[feldon_s_cane()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cane = on_battlefield(&engine, p0, feldon_s_cane()).expect("the Cane is out");
    let library_before = library_size(&engine, p0);
    seed_graveyard(&mut engine, p0, 3);
    assert_eq!(library_size(&engine, p0), library_before - 3);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        3
    );

    let from = engine.journal().entries().len();
    activate(&mut engine, p0, feldon_s_cane(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty(),
        "the graveyard is shuffled into the library"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "and every card of it is back"
    );
    assert_eq!(
        engine.state().object(cane).map(|o| o.zone),
        Some(Zone::Exile),
        "exiling the Cane paid its own cost"
    );
    assert!(
        engine.journal().entries()[from..]
            .iter()
            .any(|entry| matches!(
                entry.event,
                crate::event::GameEvent::Shuffled {
                    player,
                    zone: Zone::Library
                } if player == p0
            )),
        "the effect shuffles rather than stacking the cards on top"
    );
}
