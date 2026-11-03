//! Playing a land with a real Forest (CR 305.1–305.3): it goes straight
//! onto the battlefield without using the stack, the player keeps
//! priority, and it may be played only in its controller's own main phase
//! with an empty stack, once per turn, the count starting over each turn.

use super::testkit::{
    Duel, basic_forest, cast_from_hand, in_hand, keep_mulligans, on_battlefield, pass_until,
    quiet_creature,
};
use super::*;
use crate::zone::ZoneLocation;

const SEED: u64 = 0x1a4d_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn main_of(engine: &mut Game, seat: PlayerId) {
    let start = engine.state().turn.number;
    let here = |e: &Game| {
        e.state().turn.active == seat
            && e.state().turn.phase == Phase::FirstMain
            && e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == seat)
    };
    let already = here(engine);
    pass_until(engine, |e| {
        here(e) && (!already || e.state().turn.number > start)
    });
}

fn duel() -> Game {
    let forest = basic_forest();
    let mut engine = Duel::new(SEED, forest)
        .hand(0, &[forest, forest, forest, quiet_creature()])
        .hand(1, &[forest, forest])
        .battlefield(0, &[forest])
        .start();
    keep_mulligans(&mut engine);
    main_of(&mut engine, P0);
    engine
}

fn play(engine: &mut Game, seat: PlayerId) -> Result<(), String> {
    let card = in_hand(engine, seat, basic_forest()).ok_or("no forest in hand")?;
    engine
        .apply(seat, PlayerAction::PlayLand { card })
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

fn lands(engine: &Game, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == seat && o.card.is_some_and(|c| c.index == basic_forest())
            })
        })
        .count()
}

#[test]
fn a_land_goes_straight_onto_the_battlefield_and_priority_stays() {
    let mut engine = duel();
    let before = lands(&engine, P0);
    play(&mut engine, P0).unwrap();
    assert_eq!(lands(&engine, P0), before + 1);
    assert!(engine.state().zones.stack_is_empty(), "no stack");
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0));
}

#[test]
fn a_second_land_in_one_turn_is_refused() {
    let mut engine = duel();
    play(&mut engine, P0).unwrap();
    let before = engine.state().snapshot_hash();
    assert!(play(&mut engine, P0).is_err());
    assert_eq!(engine.state().snapshot_hash(), before);
}

#[test]
fn the_land_drop_comes_back_next_turn() {
    let mut engine = duel();
    play(&mut engine, P0).unwrap();
    main_of(&mut engine, P0);
    assert!(play(&mut engine, P0).is_ok());
}

#[test]
fn a_land_is_not_played_in_the_opponents_turn() {
    let mut engine = duel();
    main_of(&mut engine, P1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    assert!(play(&mut engine, P0).is_err());
}

#[test]
fn a_land_is_not_played_while_a_spell_is_on_the_stack() {
    let mut engine = duel();
    cast_from_hand(&mut engine, P0, quiet_creature());
    assert!(!engine.state().zones.stack_is_empty());
    assert!(play(&mut engine, P0).is_err());
}

#[test]
fn a_land_is_not_played_outside_a_main_phase() {
    let mut engine = duel();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::Ending
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(play(&mut engine, P0).is_err());
}

#[test]
fn playing_a_land_moves_it_out_of_the_hand() {
    let mut engine = duel();
    let card = in_hand(&engine, P0, basic_forest()).unwrap();
    let hand = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    engine.apply(P0, PlayerAction::PlayLand { card }).unwrap();
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand - 1
    );
    assert!(on_battlefield(&engine, P0, basic_forest()).is_some());
}
