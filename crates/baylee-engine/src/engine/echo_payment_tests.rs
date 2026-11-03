//! Echo come due beside untapped lands (CR 702.30a): the player is asked
//! with an empty pool, and a yes opens a window to make the mana in
//! (CR 605.3a). Bug hunt of 3.11.2026, turn 33: Mogg War Marshal was
//! sacrificed in its controller's upkeep without the question, beside six
//! untapped lands, because the question was put only to a pool that
//! already paid the echo.

use super::testkit::{
    Duel, basic_forest, card_index, cast_from_hand, in_graveyard, keep_mulligans, on_battlefield,
    pass_until, reach_main_phase, stack_is_empty, tap_all_mana,
};
use super::*;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::mana::{ManaCost, ManaPayment};

const SEED: u64 = 0xec40_0001;
const P0: PlayerId = PlayerId::new(0);

type Game = Engine<super::testkit::RegistryLookup>;

/// Pouncing Jaguar, {G}, a 2/2 with echo {G}.
fn jaguar() -> baylee_core::ids::CardIndex {
    card_index("70a2b5ea-dae4-46e8-87eb-8c8d8dfb242b")
}

/// Seat 0 casts the Jaguar from two Forests on its first turn and walks to
/// its next upkeep, where the echo is due; `before` runs on the way, in
/// seat 1's turn.
fn echo_due(before: impl Fn(&mut Game)) -> Game {
    let forest = basic_forest();
    let mut engine = Duel::new(SEED, forest)
        .hand(0, &[jaguar()])
        .battlefield(0, &[forest, forest])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, jaguar());
    pass_until(&mut engine, stack_is_empty);
    pass_until(&mut engine, |e| e.state().turn.active != P0);
    before(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
            || e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    engine
}

/// Asked with the pool empty and both Forests untapped.
fn asked() -> Game {
    let engine = echo_due(|_| {});
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, .. } if *player == P0),
        "the echo is asked: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool is empty"
    );
    engine
}

fn jaguar_stays(engine: &Game) -> bool {
    on_battlefield(engine, P0, jaguar()).is_some()
}

#[test]
fn a_yes_opens_a_window_and_the_lands_pay_the_echo() {
    let mut engine = asked();
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((P0, ManaPayment::Fixed(ManaCost::parse("{G}"))))
    );
    assert!(tap_all_mana(&mut engine, P0) > 0, "the Forests make mana");
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.payment_window(), None);
    reach_main_phase(&mut engine, P0);
    assert!(jaguar_stays(&engine), "the echo was paid");
}

#[test]
fn a_window_closed_without_the_mana_sacrifices_it() {
    let mut engine = asked();
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, P0);
    assert!(!jaguar_stays(&engine));
    assert!(in_graveyard(&engine, P0, jaguar()).is_some());
}

#[test]
fn a_no_sacrifices_it() {
    let mut engine = asked();
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(engine.payment_window(), None);
    reach_main_phase(&mut engine, P0);
    assert!(in_graveyard(&engine, P0, jaguar()).is_some());
}

/// With no land left to make mana, nothing is asked: the question could
/// only be answered by sacrificing.
#[test]
fn with_nothing_to_make_mana_it_is_sacrificed_unasked() {
    let engine = echo_due(|engine| {
        let state = engine.dev_state_mut(P0).expect("dev board");
        while let Some(forest) = state
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .find(|&id| {
                state.object(id).is_some_and(|o| {
                    o.controller == P0 && o.card.is_some_and(|c| c.index == basic_forest())
                })
            })
        {
            state
                .move_object(
                    forest,
                    ZoneLocation::Graveyard(P0),
                    ZonePosition::Top,
                    crate::event::Cause::Setup,
                )
                .expect("the Forest moves");
        }
    });
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "nothing is asked: {:?}",
        engine.pending()
    );
    assert!(in_graveyard(&engine, P0, jaguar()).is_some());
}
