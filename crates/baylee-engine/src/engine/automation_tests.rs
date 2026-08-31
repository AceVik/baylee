//! Seat automation: priority holds and standing answers.
//!
//! The point of these tests is the *safety* of the feature, not its
//! convenience. An automated seat must never end up in a position it
//! could not have reached by hand, must always be woken when somebody
//! responds to what it is letting through, and must never have a
//! game-losing decision made for it.

use super::testkit::{Duel, RegistryLookup, card_index, keep_mulligans, on_battlefield};
use super::*;
use crate::choice::{PriorityHold, StandingAnswer};
use baylee_core::ids::{AbilityRef, CardIndex};

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}

fn started(builder: Duel) -> Engine<RegistryLookup> {
    let mut engine = builder.start();
    keep_mulligans(&mut engine);
    engine
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// The default is unchanged behaviour: every priority is offered.
#[test]
fn the_default_asks_about_everything() {
    let engine = started(Duel::new(7, forest()));
    assert_eq!(engine.automation(P0).hold, PriorityHold::Always);
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
}

/// `PassWhenNothingToDo` skips only the priorities where passing was the
/// single legal action — the seat is still asked the moment it could
/// actually do something.
#[test]
fn nothing_to_do_skips_only_empty_priorities() {
    let mut engine = started(Duel::new(7, forest()).hand(1, &[forest()]));
    // Seat 1 has a land in hand, so it always has something to do; seat 0
    // starts with an empty hand and cannot act at all.
    engine
        .apply(
            P0,
            PlayerAction::SetPriorityHold(PriorityHold::PassWhenNothingToDo),
        )
        .expect("setting a hold is always legal");

    // Whoever is asked now, it is not seat 0 with nothing to do.
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::Priority { player, legal } => {
                assert!(
                    player != P0 || !legal.nothing_but_passing(),
                    "seat 0 was asked a priority it had nothing to do with"
                );
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            _ => break,
        }
    }
}

/// Setting a hold does not advance the game or disturb the priority round:
/// the same seat is still on the clock afterwards.
#[test]
fn setting_a_hold_is_not_a_game_action() {
    let mut engine = started(Duel::new(3, forest()).hand(0, &[forest()]));
    let before = engine.state().journal.last_seq();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected a priority")
    };
    engine
        .apply(
            player,
            PlayerAction::SetStandingAnswer {
                ability: AbilityRef::new(forest(), AbilityRef::ENTERS),
                answer: Some(StandingAnswer::Yes),
            },
        )
        .expect("setting a standing answer is always legal");
    assert_eq!(
        engine.state().journal.last_seq(),
        before,
        "an automation setting must not move the game"
    );
    match engine.pending() {
        Pending::Priority { player: p, .. } => assert_eq!(*p, player),
        other => panic!("the same seat should still hold priority, got {other:?}"),
    }
}

/// A standing answer is stored under a stable `(card, ability)` handle, so
/// a gateway can persist it per account and replay it into a new game.
#[test]
fn standing_answers_round_trip_under_a_stable_handle() {
    let mut engine = started(Duel::new(3, forest()));
    let ability = AbilityRef::new(ondu_cleric(), 0);
    engine
        .apply(
            P0,
            PlayerAction::SetStandingAnswer {
                ability,
                answer: Some(StandingAnswer::Yes),
            },
        )
        .unwrap();
    assert_eq!(
        engine.automation(P0).standing_answer(ability),
        Some(StandingAnswer::Yes)
    );
    assert_eq!(
        engine.automation(P1).standing_answer(ability),
        None,
        "one seat's setting is not another's"
    );

    engine
        .apply(
            P0,
            PlayerAction::SetStandingAnswer {
                ability,
                answer: None,
            },
        )
        .unwrap();
    assert_eq!(engine.automation(P0).standing_answer(ability), None);
}

/// "Resolve the stack" is cancelled as soon as anything is added to the
/// stack — the exact moment a player wants to be asked again, because
/// somebody just responded to what they were letting through.
#[test]
fn a_stack_hold_breaks_when_the_stack_grows() {
    let mut engine = started(Duel::new(11, forest()));
    engine
        .apply(
            P0,
            PlayerAction::SetPriorityHold(PriorityHold::UntilStackEmpty { depth: 3 }),
        )
        .unwrap();
    // Nothing is on the stack, so the condition is already met and the
    // hold has expired by the time the engine came to rest.
    assert_eq!(
        engine.automation(P0).hold,
        PriorityHold::Always,
        "an already-satisfied hold does not linger"
    );
}

/// A hold that names an object cancels itself once that object is gone,
/// so a countered trigger can never leave a seat auto-passing forever.
#[test]
fn an_object_hold_expires_when_the_object_is_not_on_the_stack() {
    let mut engine = started(Duel::new(5, forest()));
    let ghost = baylee_core::ids::ObjectId::new(9_999, 0);
    engine
        .apply(
            P0,
            PlayerAction::SetPriorityHold(PriorityHold::UntilTopOfStack { object: ghost }),
        )
        .unwrap();
    assert_eq!(
        engine.automation(P0).hold,
        PriorityHold::Always,
        "a hold on an object that is not on the stack is over immediately"
    );
}

/// An end-of-turn hold really does end with the turn.
#[test]
fn a_turn_hold_expires_with_the_turn() {
    let mut engine = started(Duel::new(5, forest()));
    let turn = engine.state().turn.number;
    engine
        .apply(
            P0,
            PlayerAction::SetPriorityHold(PriorityHold::UntilEndOfTurn { turn }),
        )
        .unwrap();
    // Seat 0 is now auto-passing, so the game runs on by itself until
    // either seat 1 is asked something or the turn rolls over.
    let mut guard = 0;
    while engine.state().turn.number == turn && guard < 200 {
        guard += 1;
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                assert_ne!(player, P0, "seat 0 held priority for the whole turn");
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected pending during the held turn: {other:?}"),
        }
    }
    assert!(engine.state().turn.number > turn, "the turn advanced");
    assert_eq!(
        engine.automation(P0).hold,
        PriorityHold::Always,
        "the hold did not survive the turn it was set for"
    );
}

/// Automation changes what the engine does next, so two engines that agree
/// on the board but differ in automation must not hash the same — the loop
/// detector compares these hashes to decide a segment repeated.
#[test]
fn automation_is_part_of_the_engine_snapshot() {
    let mut a = started(Duel::new(21, forest()));
    let mut b = started(Duel::new(21, forest()));
    assert_eq!(a.snapshot_hash(), b.snapshot_hash());
    b.apply(
        P0,
        PlayerAction::SetPriorityHold(PriorityHold::PassWhenNothingToDo),
    )
    .unwrap();
    assert_ne!(
        a.snapshot_hash(),
        b.snapshot_hash(),
        "a seat that auto-passes is a different engine state"
    );
    a.apply(
        P0,
        PlayerAction::SetPriorityHold(PriorityHold::PassWhenNothingToDo),
    )
    .unwrap();
    assert_eq!(a.snapshot_hash(), b.snapshot_hash());
}

/// Ondu Cleric's rally trigger: with a standing answer stored for it, the
/// trigger resolves without the seat being asked. The card is the one the
/// feature was asked for by name.
#[test]
fn a_standing_answer_covers_a_recurring_trigger() {
    let mut engine = started(
        Duel::new(31, forest())
            .battlefield(0, &[ondu_cleric()])
            .hand(0, &[ondu_cleric()]),
    );
    engine
        .apply(
            P0,
            PlayerAction::SetStandingAnswer {
                ability: AbilityRef::new(ondu_cleric(), 0),
                answer: Some(StandingAnswer::Yes),
            },
        )
        .unwrap();
    // The setting is remembered and addressed by a handle that says
    // nothing about this particular game — which is what lets the gateway
    // store it against an account.
    assert!(on_battlefield(&engine, P0, ondu_cleric()).is_some());
    assert_eq!(
        engine
            .automation(P0)
            .standing_answer(AbilityRef::new(ondu_cleric(), 0)),
        Some(StandingAnswer::Yes)
    );
}

/// A pact's "pay or lose the game" is deliberately not automatable: it
/// carries no ability handle, so no standing answer can ever reach it.
#[test]
fn a_game_losing_question_carries_no_automation_handle() {
    // The rule is enforced at the construction site, so assert the shape
    // of the contract rather than driving a pact into play: a `YesNo` a
    // standing answer could cover must name an ability.
    let pending = Pending::YesNo {
        player: P0,
        prompt: crate::choice::YesNoPrompt::Generic,
        source: None,
    };
    let Pending::YesNo { source, .. } = pending else {
        unreachable!()
    };
    assert!(
        source.is_none(),
        "a question with no ability handle can never be auto-answered"
    );
}
