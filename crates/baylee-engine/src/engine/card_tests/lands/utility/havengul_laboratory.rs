//! `cards/lands/utility/havengul_laboratory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Havengul Laboratory // Havengul Mystery: "{T}: Add {C}." / "{4}, {T}: Investigate." / "At the beginning of your end step, if you sacrificed three or more Clues this turn, transform..."
/// Under `Coverage::Partial`, the transform loop and back-face reanimation trigger are omitted.
/// Paying `{4}` and tapping Havengul Laboratory activates Investigate, creating a Clue token on resolution.
#[test]
fn havengul_laboratory_investigates() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(226, forest())
        .battlefield(
            0,
            &[
                havengul_laboratory(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lab = on_battlefield(&engine, p0, havengul_laboratory()).expect("Lab deployed");
    tap_mana_except(&mut engine, p0, lab);
    activate(&mut engine, p0, havengul_laboratory(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(tokens_of(&engine, p0).len(), 1);
    assert!(is_tapped(&engine, lab));
}
