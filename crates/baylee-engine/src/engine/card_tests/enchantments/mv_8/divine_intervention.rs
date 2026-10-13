//! `cards/enchantments/mv_8/divine_intervention.rs`, played.
//!
//! "This enchantment enters with two intervention counters on it. At the
//! beginning of your upkeep, remove an intervention counter from this
//! enchantment. When you remove the last intervention counter from this
//! enchantment, the game is a draw."
//!
//! Three sentences, three things to see: the counters it arrives with, *whose*
//! upkeep takes one off, and what happens when the last one goes. The ruling
//! (2009-10-01) says the game ends when the third ability resolves and that it
//! is a draw -- neither player wins and neither loses.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::win::EndReason;

fn divine_intervention() -> CardIndex {
    card_index("0158a440-6573-4f12-958b-a5f0cf190d5b")
}

/// How many intervention counters the seat's Divine Intervention carries.
#[track_caller]
fn intervention_counters(engine: &Engine<RegistryLookup>, seat: PlayerId) -> u16 {
    let card = on_battlefield(engine, seat, divine_intervention())
        .expect("Divine Intervention is on the battlefield");
    counters_on(engine, card, counters::INTERVENTION)
}

/// A table of `seats` with Divine Intervention resolved on the battlefield of
/// seat 0, which has just cast it from eight Plains in its first main phase.
#[track_caller]
fn with_divine_intervention_resolved(seats: usize) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::table(SEED, plains(), seats)
        .battlefield(0, &[plains(); 8])
        .hand(0, &[divine_intervention()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, divine_intervention());
    pass_until(&mut engine, stack_is_empty);
    engine
}

/// Passes priority until the game is over and answers the result.
#[track_caller]
fn play_to_the_end(engine: &mut Engine<RegistryLookup>) -> crate::win::GameResult {
    pass_until(engine, |e| matches!(e.pending(), Pending::GameOver(_)));
    let Pending::GameOver(result) = engine.pending().clone() else {
        unreachable!("pass_until stopped on the end of the game")
    };
    result
}

/// "This enchantment enters with two intervention counters on it." Cast for
/// {6}{W}{W}, it resolves onto the battlefield with exactly two, and the game
/// goes on.
#[test]
fn divine_intervention_enters_with_two_intervention_counters() {
    let p0 = PlayerId::new(0);
    let engine = with_divine_intervention_resolved(2);

    assert!(
        on_battlefield(&engine, p0, divine_intervention()).is_some(),
        "an enchantment spell resolves onto the battlefield"
    );
    assert_eq!(
        intervention_counters(&engine, p0),
        2,
        "it enters with two intervention counters"
    );
    assert!(
        !matches!(engine.pending(), Pending::GameOver(_)),
        "casting it does not end the game"
    );
}

/// "At the beginning of **your** upkeep, remove an intervention counter": the
/// opponent's upkeep takes none off. A whole turn of the opponent's goes by
/// and the enchantment still carries both counters.
#[test]
fn divine_intervention_does_not_lose_a_counter_in_the_opponents_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = with_divine_intervention_resolved(2);

    reach_their_main_phase(&mut engine, p1);

    assert_eq!(
        engine.state().turn.active,
        p1,
        "it is the opponent's turn, their upkeep has come and gone"
    );
    assert_eq!(
        intervention_counters(&engine, p0),
        2,
        "the opponent's upkeep removed nothing"
    );
}

/// "At the beginning of your upkeep, remove an intervention counter" -- one,
/// not both. After the controller's first upkeep one counter is left, the
/// last one has not been removed, and the game is still being played.
#[test]
fn divine_intervention_removes_one_counter_in_its_controllers_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = with_divine_intervention_resolved(2);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().turn.active,
        p0,
        "back in the controller's own turn, after its upkeep"
    );
    assert_eq!(
        intervention_counters(&engine, p0),
        1,
        "its upkeep removed one of the two counters"
    );
    assert!(
        !matches!(engine.pending(), Pending::GameOver(_)),
        "that was not the last counter: the game goes on"
    );
}

/// "When you remove the last intervention counter from this enchantment, the
/// game is a draw": the second upkeep of its controller takes the last one,
/// and the game ends with no winner -- `GameResult::winner` is `None`, and
/// the reason is the draw, not a player standing alone.
#[test]
fn divine_intervention_draws_the_game_when_the_last_counter_is_removed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = with_divine_intervention_resolved(2);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(intervention_counters(&engine, p0), 1);

    let result = play_to_the_end(&mut engine);

    assert_eq!(
        result.winner, None,
        "\"the game is a draw\": neither player wins"
    );
    assert_eq!(result.reason, EndReason::Draw, "and it was a draw");
    assert_eq!(
        engine.state().turn.active,
        p0,
        "it ended in its controller's turn, on the upkeep that removed the last counter"
    );
    assert_eq!(
        intervention_counters(&engine, p0),
        0,
        "the last intervention counter was the one removed"
    );
    assert_eq!(
        (life_of(&engine, p0), life_of(&engine, p1)),
        (20, 20),
        "neither player lost anything: it is a draw, not a loss"
    );
}

/// "The game is a draw" is the table's: at a three-player table every seat
/// takes part in the draw, and the other two players' upkeeps on the way
/// remove nothing. Two full rounds of upkeeps by the other seats leave it at
/// one counter, and the third visit to its controller's upkeep ends the game.
#[test]
fn divine_intervention_draws_a_three_player_game_for_everyone() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = with_divine_intervention_resolved(3);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p2);
    assert_eq!(
        intervention_counters(&engine, p0),
        2,
        "neither of the other players' upkeeps removed a counter"
    );
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(intervention_counters(&engine, p0), 1);

    let result = play_to_the_end(&mut engine);

    assert_eq!(result.winner, None, "nobody wins a three-player draw");
    assert_eq!(result.reason, EndReason::Draw);
}
