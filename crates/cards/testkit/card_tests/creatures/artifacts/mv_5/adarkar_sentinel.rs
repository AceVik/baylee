//! `cards/creatures/artifacts/mv_5/adarkar_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Adarkar Sentinel` prints `{{1}}: This creature gets +0/+1 until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 3/3 artifact creature and two Forests.
/// Floating mana from the lands and activating the ability increases its toughness by 1,
/// which can be activated multiple times to stack toughness increments.
#[test]
fn adarkar_sentinel_pumps_toughness_with_generic_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[adarkar_sentinel(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sentinel = on_battlefield(&engine, p0, adarkar_sentinel()).expect("sentinel is seated");
    assert_eq!(pt(&engine, sentinel), (3, 3), "starts as a 3/3");

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, adarkar_sentinel(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, sentinel), (3, 4), "first pump yields 3/4");

    activate(&mut engine, p0, adarkar_sentinel(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, sentinel), (3, 5), "second pump yields 3/5");
}
