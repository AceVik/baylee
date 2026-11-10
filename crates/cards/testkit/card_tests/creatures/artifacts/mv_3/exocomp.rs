//! `cards/creatures/artifacts/mv_3/exocomp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn exocomp() -> CardIndex {
    card_index("fb208157-67e5-45e2-bf08-7671f65cf520")
}

/// Exocomp — {2}{U}, 3/1 artifact creature with flying.
#[test]
fn exocomp_resolves_as_a_three_one_flying_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[exocomp()], &[island(), island(), island()]);

    cast_and_settle(&mut engine, exocomp());

    let comp = on_battlefield(&engine, p0, exocomp()).expect("it resolved");
    assert_eq!(pt(&engine, comp), (3, 1));
    let t = types(&engine, comp);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(keywords(&engine, comp).contains(KeywordSet::FLYING));
}

/// A ground creature may block a ground attacker and not the Exocomp.
#[test]
fn exocomp_cannot_be_blocked_by_a_ground_creature() {
    let (engine, [comp, ground], options) = evasion_probe(exocomp(), &[quiet_creature()]);
    let blocker = on_battlefield(&engine, PlayerId::new(1), quiet_creature()).expect("seated");
    assert!(may_block(&options, blocker, ground));
    assert!(!may_block(&options, blocker, comp));
}
