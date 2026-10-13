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
use baylee_core::generated::index;

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

/// Gives the seat's Divine Intervention exactly `n` intervention counters.
///
/// It is placed rather than cast, so it never entered and never got its two;
/// the harness sets the count the board is meant to have.
#[track_caller]
fn seed_intervention_counters(engine: &mut Engine<RegistryLookup>, seat: PlayerId, n: u16) {
    let card = on_battlefield(engine, seat, divine_intervention())
        .expect("Divine Intervention is on the battlefield");
    let state = engine
        .dev_state_mut(seat)
        .expect("the harness may set boards up");
    state
        .object_mut(card)
        .expect("Divine Intervention")
        .counters
        .set(counters::INTERVENTION, n);
    state.invalidate_projections();
    engine.refresh_offer();
}

/// "It doesn't matter how that happens." (ruling, 2009-10-01.) Nesting Grounds
/// -- `{1}, {T}: Move a counter from target permanent you control onto a
/// second target permanent` -- takes the last intervention counter off
/// Divine Intervention and puts it on the Bears. Its controller removed it, so
/// the third ability triggers and the game is a draw.
#[test]
fn divine_intervention_draws_the_game_when_nesting_grounds_moves_its_last_counter_away() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                nesting_grounds(),
                forest(),
                divine_intervention(),
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_intervention_counters(&mut engine, p0, 1);
    let intervention = on_battlefield(&engine, p0, divine_intervention())
        .expect("Divine Intervention is on the battlefield");
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("the Bears are seated");
    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));

    activate(&mut engine, p0, nesting_grounds(), 1);
    for target in [intervention, bears] {
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!("expected a target choice, got {:?}", engine.pending())
        };
        assert!(
            options.contains(&target),
            "{target:?} is a legal target of the move: {options:?}"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![target],
                },
            )
            .unwrap();
    }
    // With only one kind of counter on the source the move may or may not
    // ask which; if it does, the intervention counter is the answer.
    for _ in 0..5 {
        pass_until(&mut engine, |e| {
            stack_is_empty(e)
                || matches!(
                    e.pending(),
                    Pending::GameOver(_) | Pending::ChooseCastMode { .. }
                )
        });
        let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
            break;
        };
        let slot = options
            .iter()
            .position(|o| {
                matches!(o.kind, crate::choice::CastModeKind::Counter(kind)
                    if kind == counters::INTERVENTION)
            })
            .expect("the intervention counter is the kind on offer");
        engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    }

    let Pending::GameOver(result) = engine.pending().clone() else {
        panic!(
            "its controller took the last counter off: the game is a draw, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(result.winner, None, "neither player wins");
    assert_eq!(result.reason, EndReason::Draw);
    assert_eq!(
        counters_on(&engine, intervention, counters::INTERVENTION),
        0,
        "the counter left Divine Intervention"
    );
    assert_eq!(
        counters_on(&engine, bears, counters::INTERVENTION),
        1,
        "and is on the Bears"
    );
}

/// A table where `seat` -- 0 for the controller of Divine Intervention, 1 for
/// its opponent -- holds Thief of Blood ({4}{B}{B}) and six Swamps, and
/// Divine Intervention has one intervention counter left. Thief of Blood's
/// "as this creature enters, remove all counters from all permanents" takes
/// that counter off whoever casts it.
///
/// Answers the engine standing in the caster's first main phase.
#[track_caller]
fn thief_of_blood_in_hand(seat: usize) -> Engine<RegistryLookup> {
    let (p0, caster) = (PlayerId::new(0), PlayerId::new(seat.try_into().unwrap()));
    let mut theirs = vec![swamp(); 6];
    let mut mine = vec![divine_intervention()];
    if seat == 0 {
        mine.append(&mut theirs);
    }
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &mine)
        .battlefield(1, &theirs)
        .hand(seat, &[index::THIEF_OF_BLOOD])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_intervention_counters(&mut engine, p0, 1);
    if caster != p0 {
        reach_their_main_phase(&mut engine, caster);
    }
    engine
}

/// "Divine Intervention's third ability triggers only if its controller
/// removes the last intervention counter from it." (ruling, 2009-10-01.) Its
/// controller casts Thief of Blood, whose enters effect removes every counter
/// from every permanent, and the game is a draw.
#[test]
fn divine_intervention_draws_when_its_controller_removes_the_last_counter_with_thief_of_blood() {
    let p0 = PlayerId::new(0);
    let mut engine = thief_of_blood_in_hand(0);

    cast_from_hand(&mut engine, p0, index::THIEF_OF_BLOOD);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::GameOver(_)) || stack_is_empty(e)
    });

    let Pending::GameOver(result) = engine.pending().clone() else {
        panic!(
            "its controller removed the last counter: draw, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(result.winner, None);
    assert_eq!(result.reason, EndReason::Draw);
}

/// "On the other hand, if the last intervention counter is removed as a result
/// of a Clockspinning another player controls, the ability won't trigger (and
/// won't ever be able to)." (ruling, 2009-10-01.) The opponent casts Thief of
/// Blood and the counter goes, but the game is not a draw -- not then, and not
/// when Divine Intervention's controller reaches their next upkeep with no
/// counter left to remove.
#[test]
fn divine_intervention_does_not_draw_when_the_opponent_removes_its_last_counter() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = thief_of_blood_in_hand(1);
    let intervention = on_battlefield(&engine, p0, divine_intervention())
        .expect("Divine Intervention is on the battlefield");

    cast_from_hand(&mut engine, p1, index::THIEF_OF_BLOOD);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::GameOver(_)) || stack_is_empty(e)
    });

    assert!(
        !matches!(engine.pending(), Pending::GameOver(_)),
        "the opponent removed it, not Divine Intervention's controller: no draw"
    );
    assert_eq!(
        counters_on(&engine, intervention, counters::INTERVENTION),
        0,
        "the Thief did take the counter off"
    );
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !matches!(engine.pending(), Pending::GameOver(_)),
        "and it never will draw: there is nothing left for the upkeep to remove"
    );
}
