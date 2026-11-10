//! `cards/creatures/mv_4/flowstone_giant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Flowstone Giant` is a 3/3 creature costing `{2}{R}{R}` under `Coverage::Implemented`.
/// It prints "{R}: This creature gets +2/-2 until end of turn."
/// When activated off a Mountain for `{R}`, it gets +2/-2 until end of turn,
/// shifting its power and toughness from 3/3 to 5/1.
#[test]
fn flowstone_giant_modifies_power_and_toughness() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[flowstone_giant(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let giant = on_battlefield(&engine, p0, flowstone_giant())
        .expect("Flowstone Giant is on the battlefield");
    assert_eq!(pt(&engine, giant), (3, 3), "base body is 3/3");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, flowstone_giant(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, giant),
        (5, 1),
        "Flowstone Giant gets +2/-2 until end of turn"
    );
    assert!(
        !is_tapped(&engine, giant),
        "activation cost did not require tapping"
    );
}
