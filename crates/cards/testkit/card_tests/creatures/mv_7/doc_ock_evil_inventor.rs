//! `cards/creatures/mv_7/doc_ock_evil_inventor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::subtypes;

fn doc_ock_evil_inventor() -> CardIndex {
    card_index("3e0738af-00ec-4707-a56d-85b5a5ce4529")
}

/// Doc Ock, Evil Inventor — an 8/8 legendary Human Scientist Villain for
/// {5}{U}{B}.
#[test]
fn doc_ock_costs_seven_and_is_an_eight_eight() {
    let board = [vec![island(), swamp()], lots(forest(), 5)].concat();
    let mut engine = table7(&board, &[doc_ock_evil_inventor()], &[]);
    let ock = cast_and_resolve(&mut engine, doc_ock_evil_inventor());
    assert_body(&engine, ock, (8, 8), KeywordSet::default());
}

/// "At the beginning of combat on your turn, target noncreature artifact you
/// control becomes an 8/8 Robot Villain artifact creature in addition to its
/// other types." Doc Ock is not an artifact, so the Sol Ring is the one
/// target; it comes out an 8/8 creature that is still an artifact, with the
/// two subtypes.
#[test]
fn doc_ock_turns_an_artifact_into_an_eight_eight_robot() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&[doc_ock_evil_inventor(), sol_ring()], &[], &[]);
    let ring = on_battlefield(&engine, p0, sol_ring()).expect("out");
    let ock = on_battlefield(&engine, p0, doc_ock_evil_inventor()).expect("out");
    assert!(
        !types(&engine, ring).contains(TypeSet::CREATURE),
        "an artifact and nothing else, to start with"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let (options, _) = target_menu(&engine);
    assert_eq!(options, vec![ring], "only the noncreature artifact");
    point_at(&mut engine, p0, &[ring], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ring), (8, 8));
    let now = types(&engine, ring);
    assert!(now.contains(TypeSet::CREATURE), "a creature now");
    assert!(now.contains(TypeSet::ARTIFACT), "in addition to artifact");
    let subs = engine
        .state()
        .object(ring)
        .expect("out")
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::ROBOT));
    assert!(subs.contains(subtypes::creature::VILLAIN));
    assert_eq!(pt(&engine, ock), (8, 8), "Doc Ock himself is unchanged");
    assert!(
        !types(&engine, ock).contains(TypeSet::ARTIFACT),
        "and no artifact"
    );
}
