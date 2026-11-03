//! "At the beginning of your upkeep" with real cards: Nyx-Fleece Ram
//! (gain 1 life) and Moroii (lose 1 life). The trigger fires in its
//! controller's upkeep only (CR 503.1a), once per upkeep, for each
//! permanent that has it, and not in the turn the permanent arrived after
//! the upkeep had passed.

use super::testkit::{Duel, basic_forest, card_index, keep_mulligans, pass_until};
use super::*;
use baylee_core::ids::CardIndex;

const SEED: u64 = 0x0b4e_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn ram() -> CardIndex {
    card_index("5a20113c-7ea8-4edf-af9f-148ccd326a88")
}

fn moroii() -> CardIndex {
    card_index("418be12f-b40a-4018-86c1-98c9000a999a")
}

fn duel(mine: &[CardIndex], theirs: &[CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    engine
}

/// Lets the game run to `seat`'s main phase of turn `number`.
fn to_main(engine: &mut Game, seat: PlayerId, number: u32) {
    pass_until(engine, |e| {
        e.state().turn.active == seat
            && e.state().turn.number == number
            && e.state().turn.phase == Phase::FirstMain
            && e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { .. })
    });
}

fn life(engine: &Game, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

#[test]
fn the_ram_gains_a_life_in_its_controllers_upkeep() {
    let mut engine = duel(&[ram()], &[]);
    let first = engine.state().turn.number;
    to_main(&mut engine, P0, first);
    assert_eq!(life(&engine, P0), 21);
}

#[test]
fn the_ram_does_nothing_in_the_opponents_upkeep() {
    let mut engine = duel(&[ram()], &[]);
    let first = engine.state().turn.number;
    to_main(&mut engine, P0, first);
    to_main(&mut engine, P1, first + 1);
    assert_eq!(life(&engine, P0), 21, "no second gain in seat 1's turn");
}

#[test]
fn every_upkeep_of_its_controller_gains_again() {
    let mut engine = duel(&[ram()], &[]);
    let first = engine.state().turn.number;
    to_main(&mut engine, P0, first + 2);
    assert_eq!(life(&engine, P0), 22);
    to_main(&mut engine, P0, first + 4);
    assert_eq!(life(&engine, P0), 23);
}

#[test]
fn two_rams_trigger_once_each() {
    let mut engine = duel(&[ram(), ram()], &[]);
    let first = engine.state().turn.number;
    to_main(&mut engine, P0, first);
    assert_eq!(life(&engine, P0), 22);
}

#[test]
fn a_loss_trigger_costs_its_controller() {
    let mut engine = duel(&[], &[moroii()]);
    let first = engine.state().turn.number;
    to_main(&mut engine, P0, first);
    assert_eq!(life(&engine, P1), 20, "not in seat 0's upkeep");
    to_main(&mut engine, P1, first + 1);
    assert_eq!(life(&engine, P1), 19);
    assert_eq!(life(&engine, P0), 20);
}

#[test]
fn a_permanent_that_arrives_after_the_upkeep_waits_for_the_next() {
    // A library of Rams; one is put onto the battlefield in the main phase.
    let mut with_ram = Duel::new(SEED, ram()).start();
    keep_mulligans(&mut with_ram);
    let first = with_ram.state().turn.number;
    to_main(&mut with_ram, P0, first);
    {
        let state = with_ram.dev_state_mut(P0).expect("dev board");
        let card = state.draw_cards(P0, 1)[0];
        state
            .move_object(
                card,
                crate::zone::ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    to_main(&mut with_ram, P1, first + 1);
    assert_eq!(life(&with_ram, P0), 20, "this turn's upkeep had passed");
    to_main(&mut with_ram, P0, first + 2);
    assert_eq!(life(&with_ram, P0), 21);
}
