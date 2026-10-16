//! `cards/artifacts/mv_4/mishra_s_groundbreaker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn mishra_s_groundbreaker() -> CardIndex {
    card_index("84fecd54-c08a-4f5a-b2fa-b254069ccadb")
}

/// "{T}, Sacrifice this artifact: Target land becomes a 3/3 artifact creature
/// that's still a land. (This effect lasts indefinitely.)" The target land,
/// not the sacrificed artifact, is the one that changes.
#[test]
fn the_target_land_becomes_a_3_3_artifact_creature_that_is_still_a_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mishra_s_groundbreaker(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = on_battlefield(&engine, p1, forest()).unwrap();
    let mine = on_battlefield(&engine, p0, forest()).unwrap();

    activate(&mut engine, p0, mishra_s_groundbreaker(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a land target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&theirs) && options.contains(&mine),
        "{options:?}"
    );
    aim_at(&mut engine, p0, theirs);
    assert!(
        in_graveyard(&engine, p0, mishra_s_groundbreaker()).is_some(),
        "sacrificed as a cost"
    );
    pass_until(&mut engine, stack_is_empty);

    let c = engine
        .state()
        .object(theirs)
        .unwrap()
        .characteristics()
        .clone();
    assert!(c.types.contains(TypeSet::LAND), "still a land");
    assert!(c.types.contains(TypeSet::CREATURE), "and a creature");
    assert!(c.types.contains(TypeSet::ARTIFACT), "and an artifact");
    assert_eq!(pt(&engine, theirs), (3, 3));
    let other = engine
        .state()
        .object(mine)
        .unwrap()
        .characteristics()
        .clone();
    assert!(!other.types.contains(TypeSet::CREATURE), "only the target");

    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine
            .state()
            .object(theirs)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "indefinitely, not until end of turn"
    );
}
