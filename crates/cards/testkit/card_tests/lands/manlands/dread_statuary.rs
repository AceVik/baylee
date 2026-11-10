//! `cards/lands/manlands/dread_statuary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dread Statuary: "{T}: Add {C}." / "{4}: This land becomes a 4/2 Golem artifact creature until end of turn. It's still a land."
/// Paid with four Forests, activating ability 1 animates the land without tapping it.
/// Upon resolution, the permanent has types Land, Creature, and Artifact, with 4/2 base P/T.
#[test]
fn dread_statuary_animates_into_golem_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(133, forest())
        .battlefield(
            0,
            &[dread_statuary(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let statuary = on_battlefield(&engine, p0, dread_statuary()).expect("Statuary deployed");
    assert!(!types(&engine, statuary).contains(TypeSet::CREATURE));

    tap_mana_except(&mut engine, p0, statuary);
    activate(&mut engine, p0, dread_statuary(), 1);

    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, statuary);
    assert!(t.contains(TypeSet::LAND));
    assert!(t.contains(TypeSet::CREATURE));
    assert!(t.contains(TypeSet::ARTIFACT));
    assert_eq!(pt(&engine, statuary), (4, 2));
    assert!(!is_tapped(&engine, statuary));
}
