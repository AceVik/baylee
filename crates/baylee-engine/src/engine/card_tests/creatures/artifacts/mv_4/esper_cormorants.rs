//! `cards/creatures/artifacts/mv_4/esper_cormorants.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn esper_cormorants() -> CardIndex {
    card_index("aeb6cb00-e8bd-4610-aa39-29663d9c9160")
}

/// Esper Cormorants — {2}{W}{U} 3/3 artifact creature with flying. Cast off
/// a Plains, an Island and two Forests it arrives as a 3/3 artifact creature
/// with flying, and a Grizzly Bears may not block it.
#[test]
fn esper_cormorants_is_a_three_three_flier() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), island(), forest(), forest()])
        .hand(0, &[esper_cormorants()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, esper_cormorants());
    pass_until(&mut engine, stack_is_empty);
    let birds = on_battlefield(&engine, p0, esper_cormorants()).expect("it resolved");
    assert_eq!(pt(&engine, birds), (3, 3));
    assert!(keywords(&engine, birds).contains(KeywordSet::FLYING));
    assert!(
        engine
            .state()
            .object(birds)
            .expect("object")
            .characteristics()
            .types
            .contains(TypeSet::ARTIFACT)
    );

    assert!(!bears_may_block(esper_cormorants()), "flying");
    assert!(bears_may_block(quiet_creature()), "the control");
}
