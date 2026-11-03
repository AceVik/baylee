//! A modal spell with a real card (Healing Salve, {W}: choose one — target
//! player gains 3 life; or prevent the next 3 damage that would be dealt
//! to any target this turn). The mode is chosen as the spell is cast
//! (CR 601.2b), each mode has its own targets, and the prevention mode
//! leaves a shield that lasts the turn.

use super::testkit::{
    Duel, basic_forest, card_index, in_graveyard, in_hand, keep_mulligans, pass_until,
    stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::CardIndex;

const SEED: u64 = 0x30da_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn salve() -> CardIndex {
    card_index("8da8644c-75a1-4fe9-8e94-900d948d631c")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn duel() -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[salve()])
        .battlefield(0, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

/// Casts the Salve with mode `mode` aimed at player `at`, then resolves it.
fn salve_at(engine: &mut Game, mode: usize, at: PlayerId) {
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, salve()).expect("in hand");
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseCastMode { .. }));
    engine.apply(P0, PlayerAction::ChooseMode(mode)).unwrap();
    let answer = match engine.pending() {
        Pending::ChoosePlayer { .. } => PlayerAction::ChoosePlayer(at),
        _ => PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![at],
        },
    };
    engine.apply(P0, answer).unwrap();
    pass_until(engine, stack_is_empty);
}

fn life(engine: &Game, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

#[test]
fn both_modes_are_offered() {
    let mut engine = duel();
    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, salve()).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    let Pending::ChooseCastMode {
        options, player, ..
    } = engine.pending().clone()
    else {
        panic!("{:?}", engine.pending())
    };
    assert_eq!(player, P0);
    assert_eq!(options.len(), 2);
}

#[test]
fn the_life_mode_gains_its_target_three_life() {
    let mut engine = duel();
    salve_at(&mut engine, 0, P0);
    assert_eq!(life(&engine, P0), 23);
    assert_eq!(life(&engine, P1), 20);
    assert!(in_graveyard(&engine, P0, salve()).is_some());
}

#[test]
fn the_life_mode_may_target_an_opponent() {
    let mut engine = duel();
    salve_at(&mut engine, 0, P1);
    assert_eq!(life(&engine, P1), 23);
    assert_eq!(life(&engine, P0), 20);
}

#[test]
fn the_prevention_mode_gains_no_life_and_leaves_a_shield() {
    let mut engine = duel();
    salve_at(&mut engine, 1, P0);
    assert_eq!(life(&engine, P0), 20);
    assert_eq!(engine.state().shields.len(), 1);
}

#[test]
fn the_shield_is_gone_after_the_turn() {
    let mut engine = duel();
    salve_at(&mut engine, 1, P0);
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert!(engine.state().shields.is_empty());
}
