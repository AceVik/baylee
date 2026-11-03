//! A player's numbers through the state's own doors: `change_life`,
//! `damage_player`, `change_player_counters`, `pay_energy`, life payment
//! and the hand-size limit. Each is read for the seat it touches and for
//! the seat beside it, and the per-turn tallies are read with the total.

use super::{RegistryLookup, make_preset};
use crate::event::Cause;
use crate::object::CounterKind;
use crate::state::GameState;
use baylee_core::ids::PlayerId;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn state(seed: u64) -> GameState {
    GameState::from_preset(&make_preset(seed), &RegistryLookup).expect("game starts")
}

#[test]
fn a_loss_and_a_gain_move_the_total_and_their_own_tallies() {
    let mut state = state(301);
    let start = state.players[0].life;
    state.change_life(P0, -3, Cause::Effect);
    state.change_life(P0, 5, Cause::Effect);
    assert_eq!(state.players[0].life, start + 2);
    assert!(state.per_turn.life_lost[0]);
    assert_eq!(state.per_turn.life_lost_amount[0], 3);
    assert_eq!(state.per_turn.life_gained_amount[0], 5);
    assert!(!state.per_turn.life_lost[1], "the other seat lost nothing");
    assert_eq!(state.players[1].life, start);
}

#[test]
fn a_change_of_zero_records_neither_a_loss_nor_a_gain() {
    let mut state = state(302);
    let start = state.players[0].life;
    state.change_life(P0, 0, Cause::Effect);
    assert_eq!(state.players[0].life, start);
    assert!(!state.per_turn.life_lost[0]);
    assert_eq!(state.per_turn.life_lost_amount[0], 0);
    assert_eq!(state.per_turn.life_gained_amount[0], 0);
}

#[test]
fn a_life_total_may_go_below_zero() {
    let mut state = state(303);
    let start = state.players[1].life;
    state.change_life(P1, -(start + 4), Cause::Effect);
    assert_eq!(state.players[1].life, -4);
    assert_eq!(
        state.per_turn.life_lost_amount[1],
        u32::try_from(start + 4).unwrap()
    );
}

#[test]
fn losses_add_up_across_one_turn() {
    let mut state = state(304);
    for _ in 0..4 {
        state.change_life(P1, -2, Cause::Effect);
    }
    assert_eq!(state.per_turn.life_lost_amount[1], 8);
}

#[test]
fn zero_damage_to_a_player_is_no_damage() {
    let mut state = state(305);
    let source = state.draw_cards(P0, 1)[0];
    let before = state.players[1].life;
    state.damage_player(source, P1, 0, false, Cause::Effect);
    assert_eq!(state.players[1].life, before);
    assert!(!state.per_turn.life_lost[1]);
}

#[test]
fn damage_to_a_player_is_a_loss_of_that_much_life() {
    let mut state = state(306);
    let source = state.draw_cards(P0, 1)[0];
    let before = state.players[1].life;
    state.damage_player(source, P1, 6, true, Cause::Effect);
    assert_eq!(state.players[1].life, before - 6);
    assert_eq!(state.per_turn.life_lost_amount[1], 6);
    assert_eq!(
        state.players[0].life, before,
        "the source's owner is untouched"
    );
}

#[test]
fn a_player_can_pay_up_to_their_life_total_and_not_more() {
    let mut state = state(307);
    state.players[0].life = 5;
    assert!(state.can_pay_life(P0, 5));
    assert!(!state.can_pay_life(P0, 6));
    assert!(
        state.can_pay_life(P0, 0),
        "paying nothing is always possible"
    );
    assert_eq!(state.life_payable(P0), 5);
}

/// CR 119.4: a player at zero or less life can pay only 0 life.
#[test]
fn a_player_below_one_life_can_pay_only_nothing() {
    let mut state = state(308);
    state.players[0].life = 0;
    assert!(state.can_pay_life(P0, 0));
    assert!(!state.can_pay_life(P0, 1));
    state.players[0].life = -3;
    assert!(!state.can_pay_life(P0, 1));
}

#[test]
fn poison_counters_are_added_and_removed_but_never_below_zero() {
    let mut state = state(309);
    state.change_player_counters(P0, CounterKind::Poison, 3);
    assert_eq!(state.players[0].poison, 3);
    state.change_player_counters(P0, CounterKind::Poison, -1);
    assert_eq!(state.players[0].poison, 2);
    state.change_player_counters(P0, CounterKind::Poison, -10);
    assert_eq!(state.players[0].poison, 0);
    assert_eq!(state.players[1].poison, 0);
}

#[test]
fn energy_is_paid_exactly_or_not_at_all() {
    let mut state = state(310);
    state.change_player_counters(P0, CounterKind::Energy, 3);
    assert!(!state.pay_energy(P0, 4), "three energy do not pay four");
    assert_eq!(
        state.players[0].energy, 3,
        "a refused payment takes nothing"
    );
    assert!(state.pay_energy(P0, 2));
    assert_eq!(state.players[0].energy, 1);
    assert!(state.pay_energy(P0, 1));
    assert_eq!(state.players[0].energy, 0);
    assert!(state.pay_energy(P0, 0), "nothing is always payable");
}

#[test]
fn rad_counters_are_a_player_counter_of_their_own() {
    let mut state = state(311);
    state.change_player_counters(P1, CounterKind::Rad, 4);
    assert_eq!(state.players[1].rad, 4);
    assert_eq!(state.players[1].poison, 0);
    assert_eq!(state.players[1].energy, 0);
}

#[test]
fn a_counter_a_player_cannot_have_is_refused_and_named() {
    let mut state = state(312);
    state.change_player_counters(P0, CounterKind::P1P1, 1);
    assert!(state.numeric_failure.is_some());
    assert_eq!(state.players[0].poison, 0);
}

#[test]
fn the_hand_size_is_seven_and_moves_with_the_modifier() {
    let mut state = state(313);
    assert_eq!(state.maximum_hand_size(P0), Some(7));
    assert!(!state.no_max_hand_size(P0));
    state.players[0].hand_modifier = -2;
    assert_eq!(state.maximum_hand_size(P0), Some(5));
    assert_eq!(state.maximum_hand_size(P1), Some(7));
    state.players[0].hand_modifier = -9;
    assert_eq!(
        state.maximum_hand_size(P0),
        Some(0),
        "a limit below zero is no hand at all"
    );
}

#[test]
fn in_a_duel_each_seat_is_the_others_opponent_and_not_its_own() {
    let state = state(314);
    assert!(state.is_opponent(P0, P1));
    assert!(state.is_opponent(P1, P0));
    assert!(!state.is_opponent(P0, P0));
    assert!(!state.is_opponent(P1, P1));
}
