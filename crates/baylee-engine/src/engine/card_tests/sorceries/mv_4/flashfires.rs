//! `cards/sorceries/mv_4/flashfires.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flashfires: "Destroy all Plains."
#[test]
fn flashfires_destroys_only_plains() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                forest(),
                island(),
                swamp(),
            ],
        )
        .hand(0, &[flashfires()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mountains = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| mountains.contains(&id));
    cast_with_floating(&mut engine, p0, flashfires());
    pass_until(&mut engine, stack_is_empty);

    assert!(all_on_battlefield(&engine, p0, plains()).is_empty());
    assert!(
        !all_on_battlefield(&engine, p0, forest()).is_empty(),
        "not other basics"
    );
    assert!(!all_on_battlefield(&engine, p0, island()).is_empty());
    assert!(!all_on_battlefield(&engine, p0, swamp()).is_empty());
}
