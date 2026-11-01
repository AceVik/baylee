//! `cards/creatures/mv_4/ib_halfheart_goblin_tactician.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::event::DamageTarget;
use baylee_core::generated::index;

fn ib() -> CardIndex {
    card_index("6742c4b9-8fa6-439f-844d-04a09fa20c45")
}

#[test]
fn ib_sacrifices_a_blocked_goblin_once_and_burns_each_blocker() {
    let mut e = lk::game(
        SEED,
        &[ib(), index::RAGING_GOBLIN],
        &[grizzly_bears(), grizzly_bears()],
        &[],
        &[],
    );
    let attacker = lk::obj(&e, lk::P0, index::RAGING_GOBLIN);
    let blockers = all_on_battlefield(&e, lk::P1, grizzly_bears());
    lk::attack(&mut e, &[attacker]);
    let from = e.journal().entries().len();
    lk::block(&mut e, &[(blockers[0], attacker), (blockers[1], attacker)]);
    assert_eq!(e.state().zones.list(ZoneLocation::Stack).len(), 1);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, lk::P0, index::RAGING_GOBLIN).is_some());
    let damage = damage_events(&e, from);
    assert_eq!(
        damage.len(),
        2,
        "one event per blocker, not one per trigger"
    );
    assert!(
        damage
            .iter()
            .all(|(source, _, amount, combat)| { *source == attacker && *amount == 4 && !combat })
    );
    assert!(all_on_battlefield(&e, lk::P1, grizzly_bears()).is_empty());
    assert!(on_battlefield(&e, lk::P0, ib()).is_some());
}

#[test]
fn ib_does_not_trigger_for_itself_and_skips_a_blocker_that_left() {
    let mut e = lk::game(
        SEED,
        &[ib(), index::RAGING_GOBLIN],
        &[grizzly_bears(), grizzly_bears()],
        &[],
        &[],
    );
    let source = lk::obj(&e, lk::P0, ib());
    let blocker = lk::obj(&e, lk::P1, grizzly_bears());
    lk::attack(&mut e, &[source]);
    lk::block(&mut e, &[(blocker, source)]);
    assert!(stack_is_empty(&e));
    assert!(on_battlefield(&e, lk::P0, ib()).is_some());
    lk::through_combat(&mut e);

    let mut e = lk::game(
        SEED,
        &[ib(), index::RAGING_GOBLIN],
        &[grizzly_bears(), grizzly_bears()],
        &[],
        &[],
    );
    let attacker = lk::obj(&e, lk::P0, index::RAGING_GOBLIN);
    lk::attack(&mut e, &[attacker]);
    let blockers = all_on_battlefield(&e, lk::P1, grizzly_bears());
    let from = e.journal().entries().len();
    lk::block(&mut e, &[(blockers[0], attacker), (blockers[1], attacker)]);
    kill(&mut e, blockers[0]);
    lk::settle(&mut e);
    let damage = damage_events(&e, from);
    assert_eq!(damage.len(), 1, "the departed blocker is not damaged");
    assert_eq!(damage[0].0, attacker);
    assert_eq!(damage[0].1, DamageTarget::Object(blockers[1]));
    assert_eq!(damage[0].2, 4);
}

#[test]
fn ibs_ability_sacrifices_two_distinct_mountains_for_two_goblins() {
    let mut e = lk::game(SEED, &[ib(), mountain(), mountain()], &[], &[], &[]);
    let mountains = all_on_battlefield(&e, lk::P0, mountain());
    activate(&mut e, lk::P0, ib(), 1);
    for expected in [1, 0] {
        let Pending::ChooseCards {
            options, min, max, ..
        } = e.pending().clone()
        else {
            panic!("a Mountain is paid, got {:?}", e.pending());
        };
        assert_eq!((min, max), (1, 1));
        assert_eq!(options.len(), expected + 1);
        e.apply(
            lk::P0,
            PlayerAction::ChooseObjects {
                objects: vec![mountains[expected]],
            },
        )
        .unwrap();
    }
    lk::settle(&mut e);
    assert!(all_on_battlefield(&e, lk::P0, mountain()).is_empty());
    assert_eq!(tokens_of(&e, lk::P0).len(), 2);
}

#[test]
fn ibs_ability_needs_two_mountains() {
    let e = lk::game(SEED, &[ib(), mountain()], &[], &[], &[]);
    assert!(!lk::offered(&e, ib(), lk::P0, 1));
}

/// "Sacrifice it. If you do": Ib's controller sacrifices, and only a
/// permanent they control (CR 701.21a). A Goblin an opponent took while the
/// trigger waited is not sacrificed and deals no damage; the old code made
/// its new controller sacrifice it and burned the blockers anyway.
#[test]
fn ib_does_not_sacrifice_a_goblin_that_changed_control_before_resolution() {
    let mut e = lk::game(
        SEED,
        &[ib(), index::RAGING_GOBLIN],
        &[grizzly_bears()],
        &[],
        &[],
    );
    let attacker = lk::obj(&e, lk::P0, index::RAGING_GOBLIN);
    let blocker = lk::obj(&e, lk::P1, grizzly_bears());
    lk::attack(&mut e, &[attacker]);
    let from = e.journal().entries().len();
    lk::block(&mut e, &[(blocker, attacker)]);
    assert_eq!(e.state().zones.list(ZoneLocation::Stack).len(), 1);
    e.state.object_mut(attacker).unwrap().set_controller(lk::P1);
    e.state.invalidate_projections();
    lk::settle(&mut e);
    assert!(
        e.state()
            .object(attacker)
            .is_some_and(|o| o.zone == Zone::Battlefield),
        "the Goblin was not Ib's controller's to sacrifice"
    );
    assert!(
        damage_events(&e, from)
            .iter()
            .all(|(source, target, _, combat)| *combat
                || !(*source == attacker && *target == DamageTarget::Object(blocker)))
    );
}
