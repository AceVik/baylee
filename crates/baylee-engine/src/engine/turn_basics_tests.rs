//! The turn's own bookkeeping, played through the engine: whose turn it
//! is, the untap step (CR 502.3, only the active player's permanents), the
//! draw step (CR 504.1, one card for the active player) and the turn
//! counter. Each rule is read on both sides: the player it applies to and
//! the one it does not.

use super::testkit::{Duel, basic_forest, keep_mulligans, on_battlefield, pass_until, reach_main_phase};
use super::*;
use crate::object::Status;
use crate::zone::ZoneLocation;

const SEED: u64 = 0x7a2b_0001;

fn hand_size(engine: &Engine<super::testkit::RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

fn library_size(engine: &Engine<super::testkit::RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Library(seat)).len()
}

fn tapped(engine: &Engine<super::testkit::RegistryLookup>, id: baylee_core::ids::ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .expect("on the battlefield")
        .status
        .contains(Status::TAPPED)
}

fn duel_with_a_forest_each() -> Engine<super::testkit::RegistryLookup> {
    let forest = basic_forest();
    let mut engine = Duel::new(SEED, forest)
        .battlefield(0, &[forest])
        .battlefield(1, &[forest])
        .start();
    keep_mulligans(&mut engine);
    engine
}

fn forests(engine: &Engine<super::testkit::RegistryLookup>) -> [baylee_core::ids::ObjectId; 2] {
    let forest = basic_forest();
    let mine = on_battlefield(engine, PlayerId::new(0), forest);
    let theirs = on_battlefield(engine, PlayerId::new(1), forest);
    [
        mine.expect("seat 0 has a forest"),
        theirs.expect("seat 1 has a forest"),
    ]
}

#[test]
fn the_starting_player_takes_the_first_turn_and_the_other_seat_the_second() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = duel_with_a_forest_each();
    reach_main_phase(&mut engine, p0);
    let first = engine.state().turn.number;
    assert_eq!(engine.state().turn.active, p0);
    reach_main_phase(&mut engine, p1);
    assert_eq!(engine.state().turn.number, first + 1);
    reach_main_phase(&mut engine, p0);
    assert_eq!(engine.state().turn.number, first + 2);
}

#[test]
fn the_draw_step_gives_the_active_player_one_card_and_the_other_none() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = duel_with_a_forest_each();
    reach_main_phase(&mut engine, p0);
    let (hand0, hand1) = (hand_size(&engine, p0), hand_size(&engine, p1));
    let (lib0, lib1) = (library_size(&engine, p0), library_size(&engine, p1));
    reach_main_phase(&mut engine, p1);
    assert_eq!(hand_size(&engine, p1), hand1 + 1, "the active player drew");
    assert_eq!(library_size(&engine, p1), lib1 - 1);
    assert_eq!(hand_size(&engine, p0), hand0, "the other seat drew nothing");
    assert_eq!(library_size(&engine, p0), lib0);
}

#[test]
fn the_untap_step_untaps_only_the_active_players_permanents() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = duel_with_a_forest_each();
    reach_main_phase(&mut engine, p0);
    let [mine, theirs] = forests(&engine);
    {
        let state = engine.dev_state_mut(p0).expect("dev board");
        for id in [mine, theirs] {
            state.object_mut(id).unwrap().status.insert(Status::TAPPED);
        }
    }
    reach_main_phase(&mut engine, p1);
    assert!(!tapped(&engine, theirs), "seat 1 untapped in its own untap step");
    assert!(tapped(&engine, mine), "seat 0's forest waits for seat 0's turn");
    reach_main_phase(&mut engine, p0);
    assert!(!tapped(&engine, mine));
}

#[test]
fn an_untapped_permanent_is_left_as_it_was() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = duel_with_a_forest_each();
    reach_main_phase(&mut engine, p0);
    let [mine, theirs] = forests(&engine);
    reach_main_phase(&mut engine, p1);
    assert!(!tapped(&engine, mine));
    assert!(!tapped(&engine, theirs));
}

#[test]
fn every_step_of_a_turn_happens_once_and_in_order() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = duel_with_a_forest_each();
    reach_main_phase(&mut engine, p0);
    let turn = engine.state().turn.number;
    let mut seen: Vec<(Phase, crate::turn::Step)> = Vec::new();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    // Walk a whole turn of seat 1's, recording the steps that hand out
    // priority.
    for _ in 0..200 {
        let state = engine.state();
        if state.turn.active != p1 || state.turn.number != turn + 1 {
            break;
        }
        let here = (state.turn.phase, state.turn.step);
        if seen.last() != Some(&here) {
            seen.push(here);
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            pass_until(&mut engine, |e| matches!(e.pending(), Pending::Priority { .. }));
            continue;
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    for (i, here) in seen.iter().enumerate() {
        assert!(
            !seen[i + 1..].contains(here),
            "no step came back: {seen:?}"
        );
    }
    let first_main = seen.iter().position(|(p, _)| *p == Phase::FirstMain);
    let second_main = seen.iter().position(|(p, _)| *p == Phase::SecondMain);
    let end = seen.iter().position(|(_, s)| *s == crate::turn::Step::End);
    assert!(first_main.is_some() && end.is_some(), "{seen:?}");
    assert!(first_main < second_main, "{seen:?}");
    assert!(second_main < end, "the end step follows the main phases: {seen:?}");
}

#[test]
fn a_tapped_land_untaps_for_its_controller_every_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = duel_with_a_forest_each();
    reach_main_phase(&mut engine, p0);
    let [mine, _] = forests(&engine);
    for _ in 0..3 {
        engine
            .dev_state_mut(p0)
            .expect("dev board")
            .object_mut(mine)
            .unwrap()
            .status
            .insert(Status::TAPPED);
        reach_main_phase(&mut engine, p1);
        assert!(tapped(&engine, mine));
        reach_main_phase(&mut engine, p0);
        assert!(!tapped(&engine, mine));
    }
}
