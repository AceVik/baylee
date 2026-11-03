//! Who may block whom and who may attack, one restriction per test and
//! each read against a pairing it does not touch: flying and reach
//! (CR 702.9b, 702.17b), a tapped blocker (CR 509.1a), "can't block",
//! unblockable, the blocker's controller (CR 509.1a, 802.4a), menace's
//! count (CR 702.111b) and the attack-side rules of CR 508.1a.

use super::*;
use crate::combat::can_attack;
use crate::combat::menace_satisfiable;
use crate::object::Status;

fn tap(state: &mut GameState, id: ObjectId) {
    state.object_mut(id).unwrap().status.insert(Status::TAPPED);
}

#[test]
fn a_flier_is_blocked_by_a_flier_or_a_reacher_and_not_by_a_ground_creature() {
    let mut state = empty_state();
    let bird = creature(&mut state, P0, 1, 1, KeywordSet::FLYING);
    let ground = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    let flier = creature(&mut state, P1, 2, 2, KeywordSet::FLYING);
    let spider = creature(&mut state, P1, 2, 2, KeywordSet::REACH);
    assert!(!can_block(&state, P1, ground, bird));
    assert!(can_block(&state, P1, flier, bird));
    assert!(can_block(&state, P1, spider, bird));
}

#[test]
fn a_flier_may_block_a_ground_creature() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let flier = creature(&mut state, P1, 1, 1, KeywordSet::FLYING);
    assert!(
        can_block(&state, P1, flier, bear),
        "flying limits the attacker's blockers only"
    );
}

#[test]
fn a_tapped_creature_cannot_block() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    assert!(can_block(&state, P1, blocker, bear));
    tap(&mut state, blocker);
    assert!(!can_block(&state, P1, blocker, bear));
}

#[test]
fn a_tapped_attacker_can_still_be_blocked() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    attack(&mut state, bear, P1);
    tap(&mut state, bear);
    assert!(can_block(&state, P1, blocker, bear));
}

#[test]
fn a_creature_that_cant_block_blocks_nothing() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let coward = creature(&mut state, P1, 4, 4, KeywordSet::CANT_BLOCK);
    assert!(!can_block(&state, P1, coward, bear));
}

#[test]
fn an_unblockable_attacker_is_blocked_by_nothing() {
    let mut state = empty_state();
    let rogue = creature(&mut state, P0, 1, 1, KeywordSet::UNBLOCKABLE);
    for keywords in [KeywordSet::EMPTY, KeywordSet::FLYING, KeywordSet::REACH] {
        let blocker = creature(&mut state, P1, 5, 5, keywords);
        assert!(!can_block(&state, P1, blocker, rogue));
    }
}

#[test]
fn a_player_blocks_only_with_creatures_they_control() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let theirs = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    let mine = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    attack(&mut state, bear, P1);
    assert!(can_block(&state, P1, theirs, bear));
    assert!(
        !can_block(&state, P1, mine, bear),
        "a creature of the attacking player's is not P1's to block with"
    );
}

#[test]
fn a_player_who_is_not_attacked_blocks_nothing() {
    let mut state = seated(3);
    let p2 = PlayerId::new(2);
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let bystander = creature(&mut state, p2, 2, 2, KeywordSet::EMPTY);
    attack(&mut state, bear, P1);
    assert!(!can_block(&state, p2, bystander, bear));
}

#[test]
fn a_noncreature_cannot_block() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let rock = creature(&mut state, P1, 0, 0, KeywordSet::EMPTY);
    {
        let base = state.object_mut(rock).unwrap().base_mut();
        base.types = TypeSet::ARTIFACT;
        base.power = None;
        base.toughness = None;
    }
    state.invalidate_projections();
    assert!(!can_block(&state, P1, rock, bear));
}

#[test]
fn menace_needs_two_possible_blockers() {
    let mut state = empty_state();
    let brute = creature(&mut state, P0, 3, 3, KeywordSet::MENACE);
    let one = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
    attack(&mut state, brute, P1);
    assert!(!menace_satisfiable(&state, P1, brute, &[one]));
    let two = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
    assert!(menace_satisfiable(&state, P1, brute, &[one, two]));
}

#[test]
fn menace_counts_only_creatures_that_could_block_it() {
    let mut state = empty_state();
    let brute = creature(&mut state, P0, 3, 3, KeywordSet::MENACE);
    let one = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
    let tapped = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
    tap(&mut state, tapped);
    attack(&mut state, brute, P1);
    assert!(!menace_satisfiable(&state, P1, brute, &[one, tapped]));
}

#[test]
fn an_attacker_without_menace_is_always_satisfiable() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    attack(&mut state, bear, P1);
    assert!(menace_satisfiable(&state, P1, bear, &[]));
}

#[test]
fn a_tapped_creature_cannot_attack() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::HASTE);
    assert!(can_attack(&state, P0, bear));
    tap(&mut state, bear);
    assert!(!can_attack(&state, P0, bear));
}

#[test]
fn a_creature_with_cant_attack_stays_home() {
    let mut state = empty_state();
    let bear = creature(
        &mut state,
        P0,
        2,
        2,
        KeywordSet::HASTE.union(KeywordSet::CANT_ATTACK),
    );
    assert!(!can_attack(&state, P0, bear));
}

#[test]
fn a_player_attacks_only_with_their_own_creatures() {
    let mut state = empty_state();
    let theirs = creature(&mut state, P1, 2, 2, KeywordSet::HASTE);
    assert!(!can_attack(&state, P0, theirs));
    assert!(can_attack(&state, P1, theirs));
}

#[test]
fn a_noncreature_permanent_cannot_attack() {
    let mut state = empty_state();
    let rock = creature(&mut state, P0, 0, 0, KeywordSet::HASTE);
    state.object_mut(rock).unwrap().base_mut().types = TypeSet::ARTIFACT;
    state.invalidate_projections();
    assert!(!can_attack(&state, P0, rock));
}

#[test]
fn a_phased_out_creature_cannot_attack() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::HASTE);
    state
        .object_mut(bear)
        .unwrap()
        .status
        .insert(Status::PHASED_OUT);
    assert!(!can_attack(&state, P0, bear));
}
