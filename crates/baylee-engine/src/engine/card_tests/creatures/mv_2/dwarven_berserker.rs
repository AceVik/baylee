//! `cards/creatures/mv_2/dwarven_berserker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dwarven_berserker() -> CardIndex {
    card_index("cc43507a-ca61-45ae-ac0a-4ddb851c27ef")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn wall_of_ice() -> CardIndex {
    card_index("1945e165-7636-4727-89ba-3811251ef175")
}

/// The Berserker has attacked P1 and P1's board is at the block question.
fn berserker_attacks(blockers: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = lk::game(SEED, &[dwarven_berserker()], blockers, &[], &[]);
    let berserker = lk::obj(&e, P0, dwarven_berserker());
    lk::attack(&mut e, &[berserker]);
    (e, berserker)
}

/// Dwarven Berserker (cc43507a-ca61-45ae-ac0a-4ddb851c27ef) — "Whenever this
/// creature becomes blocked, it gets +3/+0 and gains trample until end of
/// turn." Blocked by a 2/2, the 4/1 tramples over for two.
#[test]
fn dwarven_berserker_blocked_gets_plus_three_and_tramples_over() {
    let (mut e, berserker) = berserker_attacks(&[grizzly_bears()]);
    assert_eq!(pt(&e, berserker), (1, 1), "no bonus before the block");
    assert!(!keywords(&e, berserker).contains(KeywordSet::TRAMPLE));
    let bears = lk::obj(&e, P1, grizzly_bears());

    lk::block(&mut e, &[(bears, berserker)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, berserker), (4, 1));
    assert!(keywords(&e, berserker).contains(KeywordSet::TRAMPLE));

    lk::to_step(&mut e, P0, Step::CombatEnd);
    assert!(
        in_graveyard(&e, P1, grizzly_bears()).is_some(),
        "two of the four damage killed the Bears"
    );
    assert_eq!(lk::life(&e, P1), 18, "and the other two trampled over");
}

/// Dwarven Berserker — the bonus is "until end of turn": blocked by a wall
/// that cannot hurt it, it is a 1/1 without trample again on P1's turn.
#[test]
fn dwarven_berserker_loses_the_bonus_at_end_of_turn() {
    let (mut e, berserker) = berserker_attacks(&[wall_of_ice()]);
    let wall = lk::obj(&e, P1, wall_of_ice());

    lk::block(&mut e, &[(wall, berserker)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, berserker), (4, 1));

    lk::to_step(&mut e, P0, Step::CombatEnd);
    assert_eq!(lk::life(&e, P1), 20, "all four went into the 0/7");
    assert!(on_battlefield(&e, P0, dwarven_berserker()).is_some());

    reach_their_main_phase(&mut e, P1);
    assert_eq!(pt(&e, berserker), (1, 1), "the bonus ended with the turn");
    assert!(!keywords(&e, berserker).contains(KeywordSet::TRAMPLE));
}

/// Dwarven Berserker — unblocked, it never "becomes blocked": a 1/1 for one.
#[test]
fn dwarven_berserker_unblocked_gets_no_bonus() {
    let (mut e, berserker) = berserker_attacks(&[grizzly_bears()]);
    lk::block(&mut e, &[]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, berserker), (1, 1));
    assert!(!keywords(&e, berserker).contains(KeywordSet::TRAMPLE));

    lk::to_step(&mut e, P0, Step::CombatEnd);
    assert_eq!(lk::life(&e, P1), 19, "one damage, no more");
}
