//! `cards/sorceries/mv_4/tsunami.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tsunami: "Destroy all Islands."
#[test]
fn tsunami_destroys_only_islands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                plains(),
                mountain(),
                swamp(),
            ],
        )
        .hand(0, &[tsunami()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    cast_with_floating(&mut engine, p0, tsunami());
    pass_until(&mut engine, stack_is_empty);

    assert!(all_on_battlefield(&engine, p0, island()).is_empty());
    assert!(
        !all_on_battlefield(&engine, p0, plains()).is_empty(),
        "not other basics"
    );
    assert!(!all_on_battlefield(&engine, p0, mountain()).is_empty());
    assert!(!all_on_battlefield(&engine, p0, swamp()).is_empty());
}
