//! `cards/creatures/mv_4/fetid_horror.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Fetid Horror` is a 1/2 creature costing `{3}{B}` under `Coverage::Implemented`.
/// It prints "{B}: This creature gets +1/+1 until end of turn."
/// When activated off a Swamp for `{B}`, it grows from 1/2 to 2/3 until end of turn without tapping.
#[test]
fn fetid_horror_pumps_power_and_toughness() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fetid_horror(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let horror =
        on_battlefield(&engine, p0, fetid_horror()).expect("Fetid Horror is on the battlefield");
    assert_eq!(pt(&engine, horror), (1, 2), "base body is 1/2");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, fetid_horror(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, horror),
        (2, 3),
        "Fetid Horror gets +1/+1 until end of turn"
    );
    assert!(
        !is_tapped(&engine, horror),
        "activation cost did not require tapping"
    );
}
