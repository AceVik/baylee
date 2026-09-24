//! Leaving the game after turn 1 (CR 104.3a, CR 800.4a).
//!
//! A concession takes one player out and nobody else's turn to decide with
//! it. Priority moves only if the leaver held it (CR 800.4a); a round of
//! passes waits for every player still in the game (CR 117.4), whether or
//! not the leaver had passed; and a game with one side left is over. The
//! mulligans are `house_rules_tests`' (#267).

use super::testkit::{Duel, RegistryLookup, basic_forest, keep_mulligans};
use super::*;
use crate::win::Victor;

fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

/// A table of `seats` past its mulligans, seat 0 holding priority on turn 1.
fn table(seats: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::table(275, basic_forest(), seats).start();
    keep_mulligans(&mut engine);
    assert_eq!(
        engine.pending().asked(),
        Some(seat(0)),
        "{:?}",
        engine.pending()
    );
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
    engine
}

fn pass(engine: &mut Engine<RegistryLookup>, n: u8) {
    engine.apply(seat(n), PlayerAction::PassPriority).unwrap();
}

/// #275: seat 2 concedes while seat 0 holds priority with nobody passed.
/// Seat 0 still holds it, and the round still waits for seat 1.
#[test]
fn a_bystanders_concession_leaves_priority_where_it_was() {
    let mut engine = table(3);
    let (turn, step) = (engine.state().turn.number, engine.state().turn.step);

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    assert_eq!(
        engine.pending().asked(),
        Some(seat(0)),
        "{:?}",
        engine.pending()
    );
    assert_eq!(engine.passes, 0, "nobody has passed");
    assert_eq!(
        (engine.state().turn.number, engine.state().turn.step),
        (turn, step)
    );

    pass(&mut engine, 0);
    assert_eq!(engine.pending().asked(), Some(seat(1)));
    pass(&mut engine, 1);
    assert_ne!(
        (engine.state().turn.number, engine.state().turn.step),
        (turn, step),
        "both players still in the game passed, so the step ended"
    );
}

/// A pass the leaver made is not one the round can count any more: seat 3
/// has not passed, so seat 2's pass does not end the step (CR 117.4).
#[test]
fn a_leavers_pass_is_not_counted_towards_the_round() {
    let mut engine = table(4);
    let step = engine.state().turn.step;
    pass(&mut engine, 0);
    pass(&mut engine, 1);
    assert_eq!(engine.pending().asked(), Some(seat(2)));

    engine.apply(seat(1), PlayerAction::Concede).unwrap();
    assert_eq!(
        engine.pending().asked(),
        Some(seat(2)),
        "{:?}",
        engine.pending()
    );
    assert_eq!(engine.passes, 1, "seat 0's pass, and not seat 1's");

    pass(&mut engine, 2);
    assert_eq!(
        engine.pending().asked(),
        Some(seat(3)),
        "the step ended before seat 3 passed: {:?}",
        engine.pending()
    );
    assert_eq!(engine.state().turn.step, step);
}

/// The passes are counted back over the players still in the game. Seat 1
/// left before the round, so the two passes ahead of seat 3 are seat 2's
/// and seat 0's, and seat 0 leaving takes its own away: seat 4 is asked.
#[test]
fn a_seat_that_left_earlier_is_not_one_of_the_passes() {
    let mut engine = table(5);
    let step = engine.state().turn.step;
    engine.apply(seat(1), PlayerAction::Concede).unwrap();
    pass(&mut engine, 0);
    pass(&mut engine, 2);
    assert_eq!(engine.pending().asked(), Some(seat(3)));

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert_eq!(engine.passes, 1, "seat 2's pass, and not seat 0's");
    pass(&mut engine, 3);
    assert_eq!(
        engine.pending().asked(),
        Some(seat(4)),
        "the step ended before seat 4 passed: {:?}",
        engine.pending()
    );
    assert_eq!(engine.state().turn.step, step);
}

/// The one case where priority does move: the leaver held it, and it passes
/// to the next player in turn order still in the game (CR 800.4a).
#[test]
fn the_holders_concession_passes_priority_on() {
    let mut engine = table(3);
    pass(&mut engine, 0);
    assert_eq!(engine.pending().asked(), Some(seat(1)));

    engine.apply(seat(1), PlayerAction::Concede).unwrap();
    assert_eq!(
        engine.pending().asked(),
        Some(seat(2)),
        "{:?}",
        engine.pending()
    );
}

/// In a duel the player not being asked can still leave, and the game is
/// over the moment they do (CR 104.2a).
#[test]
fn a_concession_by_the_seat_not_being_asked_ends_a_duel() {
    let mut engine = table(2);

    engine.apply(seat(1), PlayerAction::Concede).unwrap();
    let Pending::GameOver(result) = engine.pending() else {
        panic!("the game went on: {:?}", engine.pending());
    };
    assert_eq!(result.winner, Some(Victor::Player(seat(0))));
}
