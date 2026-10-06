//! `cards/creatures/mv_2/coiling_oracle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "If a triggered ability of a creature you control with power 2 or less
/// triggers, that ability triggers an additional time." Coiling Oracle (1/1)
/// reveals twice and puts two Forests onto the battlefield; Thragtusk (5/3)
/// gains its 5 life once.
#[test]
fn delney_doubles_the_triggers_of_small_creatures_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                delney_streetwise_lookout(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
            ],
        )
        .hand(0, &[coiling_oracle(), thragtusk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lands_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .len();
    tap_mana_where(&mut engine, p0, |_| true);
    cast_with_floating(&mut engine, p0, coiling_oracle());
    pass_until(&mut engine, stack_is_empty);
    let after_oracle = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .len();
    assert_eq!(
        after_oracle,
        lands_before + 3,
        "the Oracle and two revealed Forests"
    );

    cast_with_floating(&mut engine, p0, thragtusk());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 25, "a 5/3 triggers once");
}
