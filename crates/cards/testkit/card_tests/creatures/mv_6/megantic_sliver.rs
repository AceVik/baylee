#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("52e8ec03-23ed-43a0-bce2-ad0cd5c4d737")
}

// Scenario: own Slivers (including source) gain exactly +3/+3; a non-Sliver
// and an opponent's Sliver do not. Two sources stack, then one source leaves.
#[test]
fn megantic_sliver_pumps_only_our_slivers_including_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                subject(),
                llanowar_elves(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[subject(), island()])
        .hand(0, &[subject()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let ours = on_battlefield(&e, p0, subject()).unwrap();
    let theirs = on_battlefield(&e, p1, subject()).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    assert_eq!(pt(&e, ours), (6, 6));
    assert_eq!(pt(&e, theirs), (6, 6));
    assert_eq!(pt(&e, elf), (1, 1));
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    for id in all_on_battlefield(&e, p0, subject()) {
        assert_eq!(pt(&e, id), (9, 9));
    }
    assert_eq!(pt(&e, theirs), (6, 6));
    assert_eq!(pt(&e, elf), (1, 1));
    let cast_source = all_on_battlefield(&e, p0, subject())
        .into_iter()
        .find(|id| *id != ours)
        .unwrap();
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut e, p1, unsummon());
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![cast_source],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, ours), (6, 6));
    assert_eq!(pt(&e, theirs), (6, 6));
    assert!(in_hand(&e, p0, subject()).is_some());
}
