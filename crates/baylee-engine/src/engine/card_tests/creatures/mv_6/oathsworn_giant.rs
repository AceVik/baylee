#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("d3828907-1321-40ca-9531-30401e82a6b8")
}

// Scenario: other own creature gains +0/+2 and vigilance, including a
// non-Giant; source keeps printed vigilance but no toughness bonus;
// opposing creature is unaffected.
#[test]
fn oathsworn_giant_toughens_and_steadies_only_other_own_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
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
    assert_eq!(pt(&e, ours), (1, 1));
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    let giant = on_battlefield(&e, p0, subject()).unwrap();
    assert_eq!(pt(&e, giant), (3, 4));
    assert!(keywords(&e, giant).contains(KeywordSet::VIGILANCE));
    assert_eq!(pt(&e, ours), (1, 3));
    assert!(keywords(&e, ours).contains(KeywordSet::VIGILANCE));
    assert_eq!(pt(&e, theirs), (1, 1));
    assert!(!keywords(&e, theirs).contains(KeywordSet::VIGILANCE));
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut e, p1, unsummon());
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![giant],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, p0, subject()).is_some());
    assert_eq!(pt(&e, ours), (1, 1));
    assert!(!keywords(&e, ours).contains(KeywordSet::VIGILANCE));
}
