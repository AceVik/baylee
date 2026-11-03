//! The replay contract at its plainest: one seed and one sequence of
//! answers give one game, hash for hash; another seed gives another game;
//! and an answer from a player the engine did not ask is refused without
//! touching the state.

use super::testkit::{Duel, basic_forest, keep_mulligans, pass_until};
use super::*;

const SEED: u64 = 0x5eed_0f01;

fn game(seed: u64) -> Engine<super::testkit::RegistryLookup> {
    let mut engine = Duel::new(seed, basic_forest()).start();
    keep_mulligans(&mut engine);
    engine
}

/// Passes once for whoever holds priority, answering an empty attack or
/// block where one is asked instead.
fn step(engine: &mut Engine<super::testkit::RegistryLookup>) {
    let before = engine.state().snapshot_hash();
    let turn = engine.state().turn;
    pass_until(engine, |e| {
        e.state().snapshot_hash() != before || e.state().turn != turn
    });
}

#[test]
fn one_seed_and_one_sequence_of_answers_give_one_game() {
    let mut a = game(SEED);
    let mut b = game(SEED);
    assert_eq!(a.state().snapshot_hash(), b.state().snapshot_hash());
    for i in 0..40 {
        step(&mut a);
        step(&mut b);
        assert_eq!(
            a.state().snapshot_hash(),
            b.state().snapshot_hash(),
            "the two games parted after {i} answers"
        );
    }
    assert!(a.state().turn.number > 1, "the walk crossed a turn");
}

#[test]
fn two_seeds_give_two_games() {
    let a = game(SEED);
    let b = game(SEED ^ 0xffff);
    assert_ne!(a.state().snapshot_hash(), b.state().snapshot_hash());
}

#[test]
fn a_pass_from_a_player_without_priority_is_refused_and_changes_nothing() {
    let mut engine = game(SEED);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. })
    });
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let other = PlayerId::new(1 - player.get());
    let before = engine.state().snapshot_hash();
    assert!(
        engine.apply(other, PlayerAction::PassPriority).is_err(),
        "only the player with priority may pass it"
    );
    assert_eq!(engine.state().snapshot_hash(), before);
    assert!(engine.apply(player, PlayerAction::PassPriority).is_ok());
}

#[test]
fn a_seat_that_is_not_at_the_table_is_refused() {
    let mut engine = game(SEED);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. })
    });
    let before = engine.state().snapshot_hash();
    assert!(
        engine
            .apply(PlayerId::new(7), PlayerAction::PassPriority)
            .is_err()
    );
    assert_eq!(engine.state().snapshot_hash(), before);
}

#[test]
fn the_hash_moves_when_the_game_moves() {
    let mut engine = game(SEED);
    let start = engine.state().snapshot_hash();
    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert_ne!(engine.state().snapshot_hash(), start);
}

#[test]
fn reading_the_state_does_not_move_the_hash() {
    let engine = game(SEED);
    let first = engine.state().snapshot_hash();
    let _ = engine.pending();
    let _ = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield);
    assert_eq!(engine.state().snapshot_hash(), first);
    assert_eq!(engine.state().clone().snapshot_hash(), first);
}
