//! `cards/sorceries/mv_4/armageddon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Armageddon: "Destroy all lands." Every land on both sides goes; a
/// creature beside them does not.
#[test]
fn armageddon_destroys_every_land_on_both_sides() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), quiet_creature()],
        )
        .hand(0, &[armageddon()])
        .battlefield(1, &[forest(), island(), mountain(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, armageddon());
    pass_until(&mut engine, stack_is_empty);

    assert!(all_on_battlefield(&engine, p0, plains()).is_empty());
    assert!(all_on_battlefield(&engine, p1, forest()).is_empty());
    assert!(all_on_battlefield(&engine, p1, island()).is_empty());
    assert!(all_on_battlefield(&engine, p1, mountain()).is_empty());
    assert!(all_on_battlefield(&engine, p1, swamp()).is_empty());
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "\"all lands\" — not creatures"
    );
}
