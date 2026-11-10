//! `cards/sorceries/mv_3/tranquility.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tranquility: "Destroy all enchantments."
#[test]
fn tranquility_destroys_all_enchantments_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                sterling_grove(),
                quiet_creature(),
            ],
        )
        .hand(0, &[tranquility()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, tranquility());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, sterling_grove()).is_none());
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "\"all enchantments\" — not creatures"
    );
}
