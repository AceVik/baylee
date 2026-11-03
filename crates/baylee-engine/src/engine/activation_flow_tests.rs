//! Activated abilities with real cards: Prodigal Sorcerer ({T}: 1 damage
//! to any target) and Shivan Dragon ({R}: +1/+0 until end of turn). A tap
//! ability needs an untapped creature that has been under its controller's
//! control since the turn began (CR 302.6), may be activated whenever its
//! controller has priority (CR 602.2), and uses the stack (CR 602.2a); a
//! pump may be activated as often as it is paid for.

use super::testkit::{
    Duel, basic_forest, card_index, keep_mulligans, on_battlefield, pass_until, pt, stack_is_empty,
    tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0xac71_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

fn dragon() -> CardIndex {
    card_index("711eea87-0fa3-46e0-a42b-fa5a86455f04")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn duel(mine: &[CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, basic_forest()).battlefield(0, mine).start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

fn ping(engine: &mut Game, source: ObjectId, at: PlayerId) -> bool {
    if engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .is_err()
    {
        return false;
    }
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![at],
            },
        )
        .unwrap();
    true
}

fn life(engine: &Game, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

fn tapped(engine: &Game, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(crate::object::Status::TAPPED)
}

#[test]
fn a_tap_ability_taps_its_source_and_resolves_from_the_stack() {
    let mut engine = duel(&[sorcerer()]);
    let wizard = on_battlefield(&engine, P0, sorcerer()).unwrap();
    assert!(ping(&mut engine, wizard, P1));
    assert!(tapped(&engine, wizard));
    assert_eq!(life(&engine, P1), 20, "on the stack, not resolved");
    assert!(!stack_is_empty(&engine));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, P1), 19);
}

#[test]
fn a_tapped_source_cannot_pay_a_tap_cost_again() {
    let mut engine = duel(&[sorcerer()]);
    let wizard = on_battlefield(&engine, P0, sorcerer()).unwrap();
    assert!(ping(&mut engine, wizard, P1));
    pass_until(&mut engine, stack_is_empty);
    assert!(!ping(&mut engine, wizard, P1));
    assert_eq!(life(&engine, P1), 19);
}

#[test]
fn a_tap_ability_works_in_the_opponents_turn() {
    let mut engine = duel(&[sorcerer()]);
    let wizard = on_battlefield(&engine, P0, sorcerer()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(ping(&mut engine, wizard, P1));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, P1), 19);
}

#[test]
fn a_creature_that_just_arrived_cannot_use_a_tap_ability() {
    // A library of Sorcerers: one is drawn and put straight onto the
    // battlefield in seat 0's main phase, so it has not been there since
    // the turn began.
    let mut engine = Duel::new(SEED, sorcerer()).start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let wizard = {
        let state = engine.dev_state_mut(P0).expect("dev board");
        let card = state.draw_cards(P0, 1)[0];
        state
            .move_object(
                card,
                crate::zone::ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap()
    };
    // The offer was made before the Sorcerer arrived; let the game ask
    // again in the next step of the same turn, so the refusal below is
    // the rule's and not a stale offer's.
    let step = engine.state().turn.step;
    pass_until(&mut engine, |e| {
        e.state().turn.step != step
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert_eq!(engine.state().turn.active, P0, "still the turn it arrived");
    assert!(!ping(&mut engine, wizard, P1));
    assert!(!tapped(&engine, wizard));
    // And on its controller's next turn it may.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && e.state().turn.number > 1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(ping(&mut engine, wizard, P1));
}

#[test]
fn a_pump_can_be_activated_once_per_payment() {
    let mut engine = duel(&[dragon(), mountain(), mountain()]);
    let shivan = on_battlefield(&engine, P0, dragon()).unwrap();
    assert_eq!(pt(&engine, shivan), (5, 5));
    tap_all_mana(&mut engine, P0);
    for _ in 0..2 {
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: shivan,
                    ability_index: 0,
                },
            )
            .unwrap();
    }
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: shivan,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the pool is empty"
    );
    pass_until(&mut engine, stack_is_empty);
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .refresh_characteristics();
    assert_eq!(pt(&engine, shivan), (7, 5));
}

#[test]
fn a_pump_without_mana_is_refused() {
    let mut engine = duel(&[dragon()]);
    let shivan = on_battlefield(&engine, P0, dragon()).unwrap();
    let before = engine.state().snapshot_hash();
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: shivan,
                    ability_index: 0,
                },
            )
            .is_err()
    );
    assert_eq!(engine.state().snapshot_hash(), before);
}

#[test]
fn an_opponent_cannot_activate_my_creatures_ability() {
    let mut engine = duel(&[sorcerer()]);
    let wizard = on_battlefield(&engine, P0, sorcerer()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::ActivateAbility {
                    source: wizard,
                    ability_index: 0,
                },
            )
            .is_err()
    );
}

/// An ability on the stack is independent of its source (CR 113.7a): the
/// Sorcerer dying before its ping resolves does not stop the ping.
#[test]
fn an_ability_resolves_after_its_source_is_gone() {
    let mut engine = duel(&[sorcerer()]);
    let wizard = on_battlefield(&engine, P0, sorcerer()).unwrap();
    assert!(ping(&mut engine, wizard, P1));
    crate::sba::destroy(engine.dev_state_mut(P0).expect("dev board"), wizard);
    assert!(on_battlefield(&engine, P0, sorcerer()).is_none());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, P1), 19);
}
