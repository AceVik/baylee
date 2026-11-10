//! `cards/creatures/artifacts/mv_5/venser_s_sliver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn venser_s_sliver() -> CardIndex {
    card_index("54d65915-3833-4e4d-a066-37ae9507846f")
}

/// Venser's Sliver — a vanilla 3/3 artifact creature.
#[test]
fn venser_s_sliver_is_a_three_three_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[venser_s_sliver()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let sliver = cast_resolved(&mut engine, p0, venser_s_sliver());
    assert_eq!(pt(&engine, sliver), (3, 3));
    assert!(is_artifact_creature(&engine, sliver));
    assert!(keywords(&engine, sliver).is_empty());
}
