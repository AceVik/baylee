//! `cards/creatures/artifacts/mv_5/stone_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn stone_golem() -> CardIndex {
    card_index("7cfb32af-ac59-4459-ac9e-2762488e9180")
}

/// Stone Golem — a vanilla 4/4 artifact creature.
#[test]
fn stone_golem_is_a_four_four_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[stone_golem()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let golem = cast_resolved(&mut engine, p0, stone_golem());
    assert_eq!(pt(&engine, golem), (4, 4));
    assert!(is_artifact_creature(&engine, golem));
    assert!(keywords(&engine, golem).is_empty());
}
