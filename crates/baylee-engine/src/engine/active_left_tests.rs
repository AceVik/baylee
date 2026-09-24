//! The turn of a player who has left (CR 800.4j, #276).
//!
//! "If a player leaves the game during their turn, that turn continues to
//! its completion without an active player. If the active player would
//! receive priority, instead the next player in turn order receives
//! priority, or the top object on the stack resolves, or the phase or step
//! ends, whichever is appropriate." Nobody declares attackers, draws or
//! discards in their place, and nobody is asked on their behalf what they
//! owe at their upkeep. Each table here is three or four seats, where a game
//! outlives a player; a duel ends instead.

use super::testkit::{Duel, RegistryLookup, answer_one, basic_forest, card_index, keep_mulligans};
use super::*;
use crate::state::DelayedAction;
use baylee_core::ids::CardIndex;

fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

fn caves_of_koilos() -> CardIndex {
    card_index("33de01e9-ce5a-42d4-afcb-343cd54a6d80")
}

/// A table of `seats` past its mulligans, seat 0 holding priority in the
/// upkeep of its own turn 1, with a painland in play and one life to lose.
fn table(seats: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::table(276, basic_forest(), seats)
        .battlefield(0, &[caves_of_koilos()])
        .life(0, 1)
        .start();
    keep_mulligans(&mut engine);
    assert_eq!(engine.state().turn.active, seat(0));
    assert_eq!(
        engine.pending().asked(),
        Some(seat(0)),
        "{:?}",
        engine.pending()
    );
    engine
}

/// Seat 0 leaves by losing: its painland's colored line deals it its last
/// life, and a state-based action takes it out (CR 704.5a) in the middle of
/// its own action.
fn lose_to_the_painland(engine: &mut Engine<RegistryLookup>) {
    let caves = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == caves_of_koilos())
        })
        .expect("the painland is in play");
    engine
        .apply(
            seat(0),
            PlayerAction::ActivateAbility {
                source: caves,
                ability_index: 1,
            },
        )
        .unwrap();
    while !engine.state().players[0].has_lost() {
        let (player, action) = answer_one(engine).expect("the colour is asked");
        assert_eq!(player, seat(0));
        engine.apply(player, action).unwrap();
    }
}

/// Answers everything until the next turn begins, and returns every step
/// the turn went through. Fails the moment seat 0 is asked anything.
fn finish_the_turn(engine: &mut Engine<RegistryLookup>) -> Vec<Step> {
    let turn = engine.state().turn.number;
    let mut steps = vec![engine.state().turn.step];
    for _ in 0..200 {
        if engine.state().turn.number != turn {
            return steps;
        }
        if steps.last() != Some(&engine.state().turn.step) {
            steps.push(engine.state().turn.step);
        }
        assert_ne!(
            engine.pending().asked(),
            Some(seat(0)),
            "a player who has left was asked, in {:?}: {:?}",
            engine.state().turn.step,
            engine.pending()
        );
        let (player, action) = answer_one(engine).expect("the game goes on");
        engine.apply(player, action).unwrap();
    }
    panic!("the turn never ended; it reached {steps:?}");
}

/// The rest of the turn, after seat 0 has left in its own upkeep: every
/// step to its end, nobody asking seat 0 anything, seat 0 drawing nothing,
/// and the next turn seat 1's.
fn goes_on_without_seat_0(engine: &mut Engine<RegistryLookup>, seats: usize) {
    assert!(engine.state().players[0].has_lost());
    let steps = finish_the_turn(engine);
    for step in [Step::Draw, Step::Main, Step::DeclareAttackers, Step::End] {
        assert!(
            steps.contains(&step),
            "{seats} seats: no {step:?} in {steps:?}"
        );
    }
    assert!(
        !steps.contains(&Step::DeclareBlockers),
        "{seats} seats: nobody attacked, so there is no blockers step (CR 508.8)"
    );
    // Its library left the game with it, so a draw would have been an
    // attempt at an empty one: the flag CR 704.5b reads.
    assert!(
        !engine.state().players[0].tried_empty_draw,
        "{seats} seats: seat 0 was made to draw"
    );
    assert_eq!(engine.state().turn.active, seat(1));
    assert_eq!(engine.state().turn.number, 2);
}

#[test]
fn the_turn_of_a_player_who_conceded_goes_on_without_them() {
    for seats in [3, 4] {
        let mut engine = table(seats);
        engine.apply(seat(0), PlayerAction::Concede).unwrap();
        goes_on_without_seat_0(&mut engine, seats);
    }
}

/// Losing in the middle of one's own action is the case where priority
/// would come straight back to the player who acted (CR 117.3c). It goes to
/// the next player instead (CR 800.4a).
#[test]
fn the_turn_of_a_player_who_lost_goes_on_without_them() {
    for seats in [3, 4] {
        let mut engine = table(seats);
        lose_to_the_painland(&mut engine);
        assert_eq!(
            engine.pending().asked(),
            Some(seat(1)),
            "{seats} seats: {:?}",
            engine.pending()
        );
        goes_on_without_seat_0(&mut engine, seats);
    }
}

/// A pact's "pay at your next upkeep, or lose the game" is the active
/// player's to answer once the upkeep's priority round is over. Seat 0
/// leaves before that, and nobody is asked on its behalf. The payment is put
/// straight into the queue a pact's delayed trigger fills, with a cost of
/// nothing, so that a seat still in the game would be asked it.
#[test]
fn a_player_who_has_left_is_not_asked_for_their_upkeep_payment() {
    for seats in [3, 4] {
        let mut engine = table(seats);
        engine
            .upkeep_payments
            .push_back(DelayedAction::PayCostOrLose {
                cost: baylee_core::mana::ManaCost::ZERO,
            });
        engine.apply(seat(0), PlayerAction::Concede).unwrap();
        goes_on_without_seat_0(&mut engine, seats);
    }
}
