//! Spells with {X} in their cost, with real cards: Stream of Life ({X}{G},
//! target player gains X life) and Mind Spring ({X}{U}{U}, draw X cards).
//! X is announced as the spell is cast (CR 601.2b), paid as part of the
//! total cost (CR 601.2f), and refused when the mana for it is not there.

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, pass_until, stack_is_empty,
    tap_all_mana,
};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;

const SEED: u64 = 0x0a0a_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn stream() -> CardIndex {
    card_index("9eb2912d-2130-49f2-9529-b58fa5a97a15")
}

fn mind_spring() -> CardIndex {
    card_index("3428287b-cfd5-45bc-b5f9-1e3f5a425a68")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn duel(spell: CardIndex, lands: &[CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[spell])
        .battlefield(0, lands)
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

/// Casts `spell` with X = `x`, aiming any player question at `at`.
/// Returns whether every step was accepted.
fn cast_x(engine: &mut Game, spell: CardIndex, x: u32, at: PlayerId) -> bool {
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, spell).expect("in hand");
    if engine.apply(P0, PlayerAction::CastSpell { card }).is_err() {
        return false;
    }
    for _ in 0..10 {
        let action = match engine.pending() {
            Pending::Priority { .. } => return true,
            Pending::ChooseNumber { .. } => PlayerAction::ChooseNumber(x),
            Pending::ChoosePlayer { .. } => PlayerAction::ChoosePlayer(at),
            Pending::ChooseTargets { .. } => PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![at],
            },
            other => panic!("unexpected question {other:?}"),
        };
        if engine.apply(P0, action).is_err() {
            return false;
        }
    }
    panic!("the cast never finished");
}

#[test]
fn stream_of_life_gains_x_life() {
    let mut engine = duel(stream(), &vec![basic_forest(); 4]);
    assert!(cast_x(&mut engine, stream(), 3, P0));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 23);
}

#[test]
fn stream_of_life_may_target_the_opponent() {
    let mut engine = duel(stream(), &vec![basic_forest(); 3]);
    assert!(cast_x(&mut engine, stream(), 2, P1));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 22);
    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn x_of_zero_is_legal_and_does_nothing() {
    let mut engine = duel(stream(), &[basic_forest()]);
    assert!(cast_x(&mut engine, stream(), 0, P0));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn an_x_beyond_the_mana_is_refused() {
    let mut engine = duel(stream(), &vec![basic_forest(); 3]);
    assert!(!cast_x(&mut engine, stream(), 5, P0));
    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn mind_spring_draws_x_cards() {
    let mut engine = duel(
        mind_spring(),
        &[island(), island(), basic_forest(), basic_forest()],
    );
    let library = engine.state().zones.list(ZoneLocation::Library(P0)).len();
    assert!(cast_x(&mut engine, mind_spring(), 2, P0));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        library - 2
    );
}

fn blaze() -> CardIndex {
    card_index("0596920f-9946-42f4-a03b-24aab67f9f1b")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

#[test]
fn blaze_deals_x_damage_to_a_player() {
    let mut engine = duel(blaze(), &vec![mountain(); 5]);
    assert!(cast_x(&mut engine, blaze(), 4, P1));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 16);
}

#[test]
fn the_x_announced_is_the_x_paid() {
    // Five Mountains: X = 4 uses them all, so nothing is left floating.
    let mut engine = duel(blaze(), &vec![mountain(); 5]);
    assert!(cast_x(&mut engine, blaze(), 4, P1));
    assert!(engine.state().players[0].mana_pool.is_empty());
}
