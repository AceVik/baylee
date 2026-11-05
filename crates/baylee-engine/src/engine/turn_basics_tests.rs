//! The turn's own bookkeeping, played through the engine: whose turn it
//! is, the untap step (CR 502.3, only the active player's permanents), the
//! draw step (CR 504.1, one card for the active player) and the turn
//! counter. Each rule is read on both sides: the player it applies to and
//! the one it does not.

use super::testkit::{Duel, basic_forest, keep_mulligans, on_battlefield, pass_until};
use super::*;
use crate::object::Status;
use crate::zone::ZoneLocation;

const SEED: u64 = 0x7a2b_0001;

/// Lets the game run to `seat`'s next first main phase: the one it is in
/// does not count, so calling it twice for the same seat walks a full
/// round.
fn main_of(engine: &mut Engine<super::testkit::RegistryLookup>, seat: PlayerId) {
    let start = engine.state().turn.number;
    let here = |e: &Engine<super::testkit::RegistryLookup>| {
        e.state().turn.active == seat
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
    };
    let already = here(engine);
    pass_until(engine, |e| {
        here(e) && (!already || e.state().turn.number > start)
    });
}

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
    main_of(&mut engine, p0);
    let first = engine.state().turn.number;
    assert_eq!(engine.state().turn.active, p0);
    main_of(&mut engine, p1);
    assert_eq!(engine.state().turn.number, first + 1);
    main_of(&mut engine, p0);
    assert_eq!(engine.state().turn.number, first + 2);
}

#[test]
fn the_draw_step_gives_the_active_player_one_card_and_the_other_none() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = duel_with_a_forest_each();
    main_of(&mut engine, p0);
    let (hand0, hand1) = (hand_size(&engine, p0), hand_size(&engine, p1));
    let (lib0, lib1) = (library_size(&engine, p0), library_size(&engine, p1));
    main_of(&mut engine, p1);
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
    main_of(&mut engine, p0);
    let [mine, theirs] = forests(&engine);
    {
        let state = engine.dev_state_mut(p0).expect("dev board");
        for id in [mine, theirs] {
            state.set_tapped(id, true);
        }
    }
    main_of(&mut engine, p1);
    assert!(
        !tapped(&engine, theirs),
        "seat 1 untapped in its own untap step"
    );
    assert!(
        tapped(&engine, mine),
        "seat 0's forest waits for seat 0's turn"
    );
    main_of(&mut engine, p0);
    assert!(!tapped(&engine, mine));
}

/// Seat 1's whole turn, read at every question it asks: each step that
/// gives priority comes once, in the order of CR 500.1, 501.1, 506.1 and
/// 512.1. The untap and cleanup steps give no priority (CR 502.4, 514.3),
/// and with no attacker declared the blockers and damage steps are skipped
/// (CR 508.8).
#[test]
fn every_step_of_a_turn_happens_once_and_in_order() {
    use crate::turn::Step;
    let p1 = PlayerId::new(1);
    let mut engine = duel_with_a_forest_each();
    main_of(&mut engine, PlayerId::new(0));
    let turn = engine.state().turn.number + 1;
    pass_until(&mut engine, |e| e.state().turn.number == turn);
    let mut seen: Vec<(Phase, Step)> = Vec::new();
    while engine.state().turn.number == turn {
        let state = engine.state();
        assert_eq!(state.turn.active, p1);
        let here = (state.turn.phase, state.turn.step);
        if seen.last() != Some(&here) {
            seen.push(here);
        }
        let before = engine.state().turn.step;
        pass_until(&mut engine, |e| {
            e.state().turn.step != before || e.state().turn.number != turn
        });
    }
    assert_eq!(
        seen,
        [
            (Phase::Beginning, Step::Upkeep),
            (Phase::Beginning, Step::Draw),
            (Phase::FirstMain, Step::Main),
            (Phase::Combat, Step::CombatBegin),
            (Phase::Combat, Step::DeclareAttackers),
            (Phase::Combat, Step::CombatEnd),
            (Phase::SecondMain, Step::Main),
            (Phase::Ending, Step::End),
        ]
    );
}

#[test]
fn a_tapped_land_untaps_for_its_controller_every_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = duel_with_a_forest_each();
    main_of(&mut engine, p0);
    let [mine, _] = forests(&engine);
    for _ in 0..3 {
        engine
            .dev_state_mut(p0)
            .expect("dev board")
            .set_tapped(mine, true);
        main_of(&mut engine, p1);
        assert!(tapped(&engine, mine));
        main_of(&mut engine, p0);
        assert!(!tapped(&engine, mine));
    }
}
