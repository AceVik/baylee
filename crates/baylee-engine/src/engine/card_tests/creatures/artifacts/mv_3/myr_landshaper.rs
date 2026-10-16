//! `cards/creatures/artifacts/mv_3/myr_landshaper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn myr_landshaper() -> CardIndex {
    card_index("7432fbe1-dce6-4b3c-a6b4-764f4612a6b6")
}

/// Myr Landshaper — "{T}: Target land becomes an artifact in addition to its
/// other types until end of turn." The target land, not the Myr, takes the
/// type; it stays a land; it stops being an artifact at end of turn.
#[test]
fn myr_landshaper_makes_the_target_land_an_artifact_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[myr_landshaper(), forest()]);
    let myr = on_battlefield(&engine, p0, myr_landshaper()).expect("seated");
    let land = on_battlefield(&engine, p0, forest()).expect("seated");
    assert!(!types(&engine, land).contains(TypeSet::ARTIFACT));

    activate(&mut engine, p0, myr_landshaper(), 0);
    let options = aim_at(&mut engine, p0, land);
    assert!(options.contains(&land));
    assert!(!options.contains(&myr), "only lands");
    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, land);
    assert!(t.contains(TypeSet::ARTIFACT), "the land became an artifact");
    assert!(t.contains(TypeSet::LAND), "in addition to its other types");

    pass_until(&mut engine, |e| e.state().turn.number > 1);
    assert!(
        !types(&engine, land).contains(TypeSet::ARTIFACT),
        "until end of turn"
    );
}
