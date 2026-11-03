//! "When this creature enters" with real cards (Healer of the Glade, {G}:
//! gain 3 life; Kindly Customer, {1}{W}: draw a card): the trigger goes
//! on the stack the next time a player would receive priority (CR 603.3)
//! and resolves like a spell, it belongs to the creature's controller,
//! and each creature that enters triggers once.

use super::testkit::{
    Duel, card_index, cast_from_hand, keep_mulligans, on_battlefield, pass_until, stack_is_empty,
};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;

const SEED: u64 = 0xe7b0_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn healer() -> CardIndex {
    card_index("2701b912-b672-4764-97f7-51c444f754f2")
}

fn customer() -> CardIndex {
    card_index("3faab84f-7055-4554-b948-181928d8d4e2")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn forest() -> CardIndex {
    super::testkit::basic_forest()
}

fn duel(hand: &[CardIndex], lands: &[CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, forest())
        .hand(0, hand)
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

fn life(engine: &Game, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

fn stack_len(engine: &Game) -> usize {
    engine.state().zones.list(ZoneLocation::Stack).len()
}

#[test]
fn the_creature_resolves_first_and_its_trigger_waits_on_the_stack() {
    let mut engine = duel(&[healer()], &[forest()]);
    cast_from_hand(&mut engine, P0, healer());
    assert_eq!(stack_len(&engine), 1);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert!(
        on_battlefield(&engine, P0, healer()).is_some(),
        "the creature is in"
    );
    assert_eq!(stack_len(&engine), 1, "its trigger is on the stack");
    assert_eq!(life(&engine, P0), 20, "and has not resolved yet");
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0));
}

#[test]
fn the_trigger_resolves_and_its_controller_gains_the_life() {
    let mut engine = duel(&[healer()], &[forest()]);
    cast_from_hand(&mut engine, P0, healer());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, P0), 23);
    assert_eq!(life(&engine, P1), 20);
}

#[test]
fn a_draw_trigger_draws_its_controller_one_card() {
    let mut engine = duel(&[customer()], &[plains(), plains()]);
    let hand = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    let library = engine.state().zones.list(ZoneLocation::Library(P0)).len();
    cast_from_hand(&mut engine, P0, customer());
    pass_until(&mut engine, stack_is_empty);
    // The cast took one card out of the hand and the trigger put one in.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        library - 1
    );
    assert!(on_battlefield(&engine, P0, customer()).is_some());
}

#[test]
fn a_creature_that_cannot_be_cast_never_triggers() {
    let mut engine = duel(&[healer()], &[]);
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert_eq!(life(&engine, P0), 20);
}

#[test]
fn two_creatures_trigger_once_each() {
    let mut engine = duel(&[healer(), healer()], &[forest(), forest()]);
    super::testkit::tap_all_mana(&mut engine, P0);
    for _ in 0..2 {
        super::testkit::cast_with_floating(&mut engine, P0, healer());
        pass_until(&mut engine, stack_is_empty);
    }
    assert_eq!(life(&engine, P0), 26);
}
