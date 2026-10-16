//! `cards/creatures/artifacts/mv_3/spined_thopter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn spined_thopter() -> CardIndex {
    card_index("45f2af87-0235-49f8-a63d-370faff3a097")
}

/// Spined Thopter — {2}{U/P}, 2/1 flier. Paid with Islands, no life.
#[test]
fn spined_thopter_paid_with_blue_costs_no_life() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[spined_thopter()], &[island(), forest(), forest()]);

    cast_from_hand(&mut engine, p0, spined_thopter());
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let id = on_battlefield(&engine, p0, spined_thopter()).expect("it resolved");
    assert_eq!(pt(&engine, id), (2, 1));
    assert!(keywords(&engine, id).contains(KeywordSet::FLYING));
    assert_eq!(life_of(&engine, 0), 20);
}

/// With no blue source the {U/P} is paid with 2 life.
#[test]
fn spined_thopter_paid_with_life_costs_two() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[spined_thopter()], &[forest(), forest()]);

    cast_and_settle(&mut engine, spined_thopter());

    assert!(on_battlefield(&engine, p0, spined_thopter()).is_some());
    assert_eq!(life_of(&engine, 0), 18);
}

/// Flying: a ground creature cannot block it.
#[test]
fn spined_thopter_cannot_be_blocked_by_a_ground_creature() {
    let (engine, [thopter, ground], options) = evasion_probe(spined_thopter(), &[quiet_creature()]);
    let blocker = on_battlefield(&engine, PlayerId::new(1), quiet_creature()).expect("seated");
    assert!(may_block(&options, blocker, ground));
    assert!(!may_block(&options, blocker, thopter));
}
