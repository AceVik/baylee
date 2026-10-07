//! `cards/lands/utility/drownyard_temple.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Drownyard Temple: "{T}: Add {C}." / "{3}: Return this card from your graveyard to the battlefield tapped."
/// Under `Coverage::Partial`, activating from the graveyard to return tapped is unsupported and omitted.
/// The land taps on the battlefield to add {C} to the mana pool.
#[test]
fn drownyard_temple_taps_for_colorless_and_omits_graveyard_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(124, forest())
        .battlefield(0, &[drownyard_temple()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let temple = on_battlefield(&engine, p0, drownyard_temple()).expect("Temple deployed");

    activate(&mut engine, p0, drownyard_temple(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, temple));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == temple && *ai == 1),
        "no second ability is offered"
    );
}
