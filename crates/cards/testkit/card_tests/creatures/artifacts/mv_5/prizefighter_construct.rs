//! `cards/creatures/artifacts/mv_5/prizefighter_construct.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn prizefighter_construct() -> CardIndex {
    card_index("6f4cfd69-c739-4398-93a5-e6238b7cd83b")
}

/// Prizefighter Construct — a vanilla 6/2 artifact creature.
#[test]
fn prizefighter_construct_is_a_six_two_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[prizefighter_construct()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let construct = cast_resolved(&mut engine, p0, prizefighter_construct());
    assert_eq!(pt(&engine, construct), (6, 2));
    assert!(is_artifact_creature(&engine, construct));
    assert!(keywords(&engine, construct).is_empty());
}
