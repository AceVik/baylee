//! Loyalty abilities with a real planeswalker (The Theorist, Jace Beleren,
//! loyalty 3: +1 makes a 1/1 Illusion, −2 bounces, −6 draws three). The
//! cost changes loyalty as it is paid (CR 606.4), only one loyalty ability
//! of a permanent per turn and only at sorcery speed (CR 606.3), and a
//! minus cost needs that much loyalty (CR 606.6).

use super::testkit::{
    Duel, basic_forest, card_index, keep_mulligans, on_battlefield, pass_until, stack_is_empty,
};
use super::*;
use crate::object::CounterKind;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0x10a1_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

/// Ability indexes in the card's list: the draw-step trigger is 0.
const PLUS_ONE: u32 = 1;
const MINUS_SIX: u32 = 3;

fn jace() -> CardIndex {
    card_index("89a6e876-0b00-4671-9652-766fd7ef9bf2")
}

fn duel() -> (Game, ObjectId) {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[jace()])
        .start();
    keep_mulligans(&mut engine);
    main_of(&mut engine, P0);
    let walker = on_battlefield(&engine, P0, jace()).expect("Jace");
    (engine, walker)
}

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

fn loyalty(engine: &Game, walker: ObjectId) -> u16 {
    engine
        .state()
        .object(walker)
        .map_or(0, |o| o.counters.get(CounterKind::Loyalty))
}

fn activate(engine: &mut Game, walker: ObjectId, index: u32) -> bool {
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: walker,
                ability_index: index,
            },
        )
        .is_ok()
}

fn illusions(engine: &Game) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == P0 && o.card.is_none())
        })
        .count()
}

#[test]
fn jace_enters_with_his_printed_loyalty() {
    let (engine, walker) = duel();
    assert_eq!(loyalty(&engine, walker), 3);
}

#[test]
fn a_plus_ability_adds_loyalty_as_its_cost_and_resolves_from_the_stack() {
    let (mut engine, walker) = duel();
    assert!(activate(&mut engine, walker, PLUS_ONE));
    assert_eq!(loyalty(&engine, walker), 4, "paid on activation");
    assert_eq!(illusions(&engine), 0, "the effect waits on the stack");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(illusions(&engine), 1);
}

#[test]
fn only_one_loyalty_ability_per_turn() {
    let (mut engine, walker) = duel();
    assert!(activate(&mut engine, walker, PLUS_ONE));
    pass_until(&mut engine, stack_is_empty);
    assert!(!activate(&mut engine, walker, PLUS_ONE));
    assert_eq!(loyalty(&engine, walker), 4);
}

#[test]
fn a_minus_cost_bigger_than_the_loyalty_is_refused() {
    let (mut engine, walker) = duel();
    assert!(!activate(&mut engine, walker, MINUS_SIX));
    assert_eq!(loyalty(&engine, walker), 3);
}

#[test]
fn loyalty_abilities_are_refused_in_the_opponents_turn() {
    let (mut engine, walker) = duel();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(!activate(&mut engine, walker, PLUS_ONE));
}

#[test]
fn the_next_turn_allows_another_activation() {
    let (mut engine, walker) = duel();
    assert!(activate(&mut engine, walker, PLUS_ONE));
    pass_until(&mut engine, stack_is_empty);
    main_of(&mut engine, P0);
    assert!(activate(&mut engine, walker, PLUS_ONE));
    assert_eq!(loyalty(&engine, walker), 5);
}

#[test]
fn a_loyalty_ability_is_refused_while_the_stack_holds_something() {
    let (mut engine, walker) = duel();
    assert!(activate(&mut engine, walker, PLUS_ONE));
    assert!(!engine.state().zones.stack_is_empty());
    assert!(!activate(&mut engine, walker, PLUS_ONE));
}
