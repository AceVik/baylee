//! `cards/creatures/artifacts/mv_5/glass_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn glass_golem() -> CardIndex {
    card_index("24e48968-bfeb-4965-9e06-8b04397fcb53")
}

/// Glass Golem — a vanilla 6/2 artifact creature: it can be cast for {5} and
/// arrives as exactly that.
#[test]
fn glass_golem_is_a_six_two_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[glass_golem()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let golem = cast_resolved(&mut engine, p0, glass_golem());
    assert_eq!(pt(&engine, golem), (6, 2));
    assert!(is_artifact_creature(&engine, golem));
    assert!(
        keywords(&engine, golem).is_empty(),
        "no rules text, no keywords"
    );
}
