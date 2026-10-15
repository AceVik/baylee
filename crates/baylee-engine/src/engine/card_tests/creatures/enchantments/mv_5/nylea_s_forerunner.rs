#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("bf0948b7-f7af-4d99-a8aa-ea7e6a69232d")
}

// Scenario: own other creature gains trample; source retains printed trample;
// opposing creature never gains it. No P/T is changed by this ability.
#[test]
fn nyleas_forerunner_grants_trample_to_other_own_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), island()])
        .hand(0, &[subject()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let ours = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let theirs = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    assert!(!keywords(&e, ours).contains(KeywordSet::TRAMPLE));
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    let source = on_battlefield(&e, p0, subject()).unwrap();
    assert!(keywords(&e, source).contains(KeywordSet::TRAMPLE));
    assert!(keywords(&e, ours).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&e, theirs).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&e, ours), (1, 1));
    assert_eq!(pt(&e, source), (5, 3));
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut e, p1, unsummon());
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![source],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, p0, subject()).is_some());
    assert!(!keywords(&e, ours).contains(KeywordSet::TRAMPLE));
}
