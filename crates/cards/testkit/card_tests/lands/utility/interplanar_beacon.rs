//! `cards/lands/utility/interplanar_beacon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Interplanar Beacon prints `Whenever you cast a planeswalker spell, you gain 1 life`, `{T}: Add {C}`,
/// and `{1}, {T}: Add two mana of different colors. Spend this mana only to cast planeswalker spells.`
/// The card is marked `Coverage::Partial` because the two picks cannot be constrained to differ in the `DSL`.
/// With one floating mana from a `forest`, activating ability index 2 requires two successive color choices,
/// producing two restricted mana entries in `pool.restricted()` rather than general available mana.
#[test]
fn interplanar_beacon_adds_two_restricted_mana_for_planeswalkers() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[interplanar_beacon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let beacon = play_land(&mut engine, p0, interplanar_beacon());
    assert!(!is_tapped(&engine, beacon));

    tap_all_mana_but(&mut engine, p0, Some(interplanar_beacon()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );

    activate(&mut engine, p0, interplanar_beacon(), 2);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected first color choice");
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected second color choice");
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 0);
    assert_eq!(pool.available(ManaColor::White), 0);
    assert_eq!(pool.restricted().len(), 2);
    assert_eq!(pool.restricted().iter().map(|m| m.amount).sum::<u16>(), 2);
    assert!(is_tapped(&engine, beacon));
}
