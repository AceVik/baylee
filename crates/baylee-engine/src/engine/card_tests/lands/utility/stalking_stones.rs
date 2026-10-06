//! `cards/lands/utility/stalking_stones.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stalking Stones: "{T}: Add {C}." / "{6}: This land becomes a 3/3 Elemental artifact creature that's still a land. (This effect lasts indefinitely.)"
/// Paid with six Forests, activating ability 1 animates the land into an artifact creature without tapping it.
/// On resolution, Stalking Stones permanently gains the Creature and Artifact types with 3/3 base P/T.
#[test]
fn stalking_stones_animates_into_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(137, forest())
        .battlefield(
            0,
            &[
                stalking_stones(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stones = on_battlefield(&engine, p0, stalking_stones()).expect("Stones deployed");
    assert!(!types(&engine, stones).contains(TypeSet::CREATURE));

    tap_mana_except(&mut engine, p0, stones);
    activate(&mut engine, p0, stalking_stones(), 1);

    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, stones);
    assert!(t.contains(TypeSet::LAND));
    assert!(t.contains(TypeSet::ARTIFACT));
    assert!(t.contains(TypeSet::CREATURE));
    assert_eq!(pt(&engine, stones), (3, 3));
    assert!(!is_tapped(&engine, stones));
}
