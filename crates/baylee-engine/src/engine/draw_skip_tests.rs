//! Island Sanctuary's skip reaches every draw of its controller's draw step
//! (`state/draw_offer.rs`).
//!
//! "If you would draw a card during your draw step, instead you may skip
//! that draw." The turn-based draw (CR 504.1) is one such draw; a Howling
//! Mine's additional card and an Ancestral Recall cast in that step are
//! others, and each card of a multiple draw is a draw of its own (CR 121.2),
//! replaced on its own before the sequence goes on (CR 614.11a). Only the
//! turn-based draw used to be asked.

use super::testkit::*;
use super::*;
use crate::choice::YesNoPrompt;
use crate::turn::Step;
use baylee_cards_dsl::Modifier;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

/// One skip question as it was asked: the turn, the step, and whether
/// something was resolving (a draw inside a resolution) or not.
#[derive(Debug, PartialEq, Eq)]
struct Asked {
    turn: u32,
    step: Step,
    resolving: bool,
}

fn hand(engine: &Engine<RegistryLookup>) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(P0)).len()
}

fn restricted(engine: &Engine<RegistryLookup>) -> bool {
    engine.state().effects.iter().any(|fx| {
        fx.controller == P0 && matches!(fx.modifier, Modifier::CantBeAttackedExceptBy { .. })
    })
}

fn skip_offered(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::MayDo,
            source: Some(source),
        } if *player == P0 && source.card == index::ISLAND_SANCTUARY
    )
}

/// Plays on, answering each skip question from `answers` in order and
/// passing everything else, until `stop` holds. Returns the questions.
#[track_caller]
fn play_until(
    engine: &mut Engine<RegistryLookup>,
    answers: &mut std::collections::VecDeque<bool>,
    stop: impl Fn(&Engine<RegistryLookup>) -> bool,
) -> Vec<Asked> {
    let mut asked = Vec::new();
    for _ in 0..200 {
        if stop(engine) {
            return asked;
        }
        if skip_offered(engine) {
            asked.push(Asked {
                turn: engine.state().turn.number,
                step: engine.state().turn.step,
                resolving: !stack_is_empty(engine),
            });
            let answer = answers.pop_front().expect("a question nobody scripted");
            engine.apply(P0, PlayerAction::YesNo(answer)).unwrap();
            continue;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => engine
                .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                .unwrap(),
            Pending::ChooseBlockers { player, .. } => engine
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .unwrap(),
            other => panic!("unexpected question {other:?}"),
        }
    }
    panic!("never reached the stop");
}

fn p0_main_of(turn: u32) -> impl Fn(&Engine<RegistryLookup>) -> bool {
    move |e| {
        e.state().turn.number == turn
            && e.state().turn.active == P0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    }
}

fn p0_draw_step_priority(e: &Engine<RegistryLookup>) -> bool {
    e.state().turn.active == P0
        && e.state().turn.step == Step::Draw
        && stack_is_empty(e)
        && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
}

/// Howling Mine's additional card is a second draw in p0's draw step and is
/// asked about as its trigger resolves, after the turn-based draw's own
/// question. The first is declined (drawn), the second taken (skipped, the
/// restriction made). On p1's turn the Mine's card is p1's, and p0 is asked
/// nothing: "your draw step".
#[test]
fn a_howling_mine_draw_in_the_draw_step_is_offered_the_skip_too() {
    let mut engine = Duel::new(4101, index::PLAINS)
        .battlefield(0, &[index::ISLAND_SANCTUARY, index::HOWLING_MINE])
        .start();
    keep_mulligans(&mut engine);
    let before = hand(&engine);
    let mut answers = [false, true].into();
    let asked = play_until(&mut engine, &mut answers, p0_main_of(3));
    assert_eq!(
        asked,
        vec![
            Asked {
                turn: 3,
                step: Step::Draw,
                resolving: false
            },
            Asked {
                turn: 3,
                step: Step::Draw,
                resolving: true
            },
        ],
        "the turn-based draw, then the Mine's card as its trigger resolves"
    );
    assert_eq!(hand(&engine), before + 1, "one drawn, one skipped");
    assert!(restricted(&engine), "the skip made the restriction");
}

/// Two Sanctuaries: each gets one opportunity at each draw (CR 614.5). Both
/// decline the turn-based draw, which is then drawn; at the Mine's card the
/// first declines and the second skips it.
#[test]
fn each_sanctuary_has_one_opportunity_at_each_draw() {
    let mut engine = Duel::new(4102, index::PLAINS)
        .battlefield(
            0,
            &[
                index::ISLAND_SANCTUARY,
                index::ISLAND_SANCTUARY,
                index::HOWLING_MINE,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let before = hand(&engine);
    let mut answers = [false, false, false, true].into();
    let asked = play_until(&mut engine, &mut answers, p0_main_of(3));
    assert_eq!(asked.len(), 4, "two Sanctuaries, two draws: {asked:?}");
    assert!(answers.is_empty());
    assert_eq!(hand(&engine), before + 1);
    assert!(restricted(&engine));
}

/// Ancestral Recall cast in p0's draw step, at p0: three draws, three
/// questions as it resolves (CR 121.2), answered no, yes, no — two cards
/// come, one is skipped.
#[test]
fn each_card_of_a_multiple_draw_is_asked_on_its_own() {
    let mut engine = Duel::new(4103, index::PLAINS)
        .battlefield(0, &[index::ISLAND_SANCTUARY, index::ISLAND])
        .hand(0, &[index::ANCESTRAL_RECALL])
        .start();
    keep_mulligans(&mut engine);
    // The turn-based draw is skipped first.
    let mut answers = [true].into();
    play_until(&mut engine, &mut answers, p0_draw_step_priority);
    assert_eq!(engine.state().turn.number, 3);
    assert!(restricted(&engine));
    cast_from_hand(&mut engine, P0, index::ANCESTRAL_RECALL);
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    let before = hand(&engine);
    let mut answers = [false, true, false].into();
    let asked = play_until(&mut engine, &mut answers, |e| {
        stack_is_empty(e) && !skip_offered(e)
    });
    assert_eq!(asked.len(), 3, "one question per card: {asked:?}");
    assert!(asked.iter().all(|a| a.resolving && a.step == Step::Draw));
    assert_eq!(hand(&engine), before + 2, "no, yes, no");
}

/// The same Recall in p0's main phase: "during your draw step" does not
/// hold, so the three cards come without a question.
#[test]
fn a_draw_outside_the_draw_step_is_not_offered() {
    let mut engine = Duel::new(4104, index::PLAINS)
        .battlefield(0, &[index::ISLAND_SANCTUARY, index::ISLAND])
        .hand(0, &[index::ANCESTRAL_RECALL])
        .start();
    keep_mulligans(&mut engine);
    let mut answers = [false].into();
    play_until(&mut engine, &mut answers, p0_main_of(3));
    cast_from_hand(&mut engine, P0, index::ANCESTRAL_RECALL);
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    let before = hand(&engine);
    let mut none = std::collections::VecDeque::new();
    let asked = play_until(&mut engine, &mut none, stack_is_empty);
    assert!(asked.is_empty(), "{asked:?}");
    assert_eq!(hand(&engine), before + 3);
}

/// A draw waiting to be asked about is part of the position: the same
/// state without it hashes differently, so a checkpoint, a replay and loop
/// detection all see it.
#[test]
fn a_waiting_draw_is_in_the_snapshot() {
    let mut engine = Duel::new(4105, index::PLAINS)
        .battlefield(0, &[index::ISLAND_SANCTUARY, index::HOWLING_MINE])
        .start();
    keep_mulligans(&mut engine);
    let mut answers = [false].into();
    play_until(&mut engine, &mut answers, |e| {
        skip_offered(e) && !stack_is_empty(e)
    });
    assert_eq!(engine.state().draws_to_offer.len(), 1);
    let mut without = engine.state().clone();
    without.draws_to_offer.clear();
    assert_ne!(without.snapshot_hash(), engine.state().snapshot_hash());
}
