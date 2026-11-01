//! Decisions tested across the real engine/view boundary.

use baylee_ai::{AIProfile, HeuristicAgent, pending_player};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::{DeckEntry, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_gamehost::{PlayerView, RegistryLookup, SeatContext, SeatKind, Session, player_view};

/// The view a seat gets at a decision point, as every fixture in this file
/// wants it: the seat being asked is the seat the table is waiting for, and
/// none of these tables ever opens a payment window.
fn asked_view(
    state: &baylee_engine::state::GameState,
    seat: PlayerId,
    seq: u64,
    pending: &Pending,
) -> PlayerView {
    player_view(
        state,
        seat,
        seq,
        Some(pending),
        &SeatContext {
            awaiting: Some(seat),
            ..Default::default()
        },
        &[],
    )
}

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).expect("registered fixture"),
        print: baylee_core::ids::PrintRef::new(0),
    }
}

fn position(hand: &[&str], board: &[&str]) -> GamePreset {
    let mut preset = baylee_cards::decks::probe_preset(41, entry("Forest").card).unwrap();
    preset.house_rules.starting_player = Some(0);
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().map(|name| entry(name)).collect());
    preset.seats[0].starting_battlefield = board.iter().map(|name| entry(name)).collect();
    preset
}

/// Answers everything a hand-driven fixture does not ask about and stops at
/// seat 0's own main phase with the stack empty. A target question is
/// answered with whichever of `aim` is among its options; a scry keeps its
/// cards on top.
fn settle(engine: &mut Engine<RegistryLookup>, aim: &[ObjectId]) {
    let me = PlayerId::new(0);
    for _ in 0..100 {
        let pending = engine.pending().clone();
        let Some(seat) = pending_player(&pending) else {
            panic!("the game ended");
        };
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat != me => PlayerAction::PassPriority,
            Pending::Priority { .. } => {
                let view = asked_view(engine.state(), me, 0, engine.pending());
                if view.stack.is_empty()
                    && view.active == me
                    && view.phase == baylee_view::Phase::FirstMain
                {
                    return;
                }
                PlayerAction::PassPriority
            }
            Pending::ChooseTargets { options, .. } => PlayerAction::ChooseObjects {
                objects: options.into_iter().filter(|o| aim.contains(o)).collect(),
            },
            Pending::YesNo { .. } => PlayerAction::YesNo(true),
            Pending::Arrange { cards, .. } => PlayerAction::Arrange {
                piles: vec![cards, vec![]],
            },
            other => panic!("unexpected question: {other:?}"),
        };
        engine.apply(seat, action).expect("a legal answer");
    }
    panic!("the table never came back to seat 0's main phase");
}

/// Seat 0's untapped permanents whose card is `name`.
fn untapped(view: &PlayerView, name: &str) -> Vec<ObjectId> {
    view.battlefield_of(PlayerId::new(0))
        .filter(|o| o.card.is_some_and(|c| c.index == entry(name).card))
        .filter(|o| !o.status.contains(baylee_view::ObjectStatus::TAPPED))
        .map(|o| o.id)
        .collect()
}

/// Taps each of seat 0's `sources` for mana, which then floats.
fn float(engine: &mut Engine<RegistryLookup>, sources: &[ObjectId]) {
    for &source in sources {
        engine
            .apply(
                PlayerId::new(0),
                PlayerAction::ActivateManaAbility { source },
            )
            .expect("a basic land taps for mana");
    }
}

#[path = "ai_decisions/card_choices.rs"]
mod card_choices;
#[path = "ai_decisions/casting.rs"]
mod casting;
#[path = "ai_decisions/combat.rs"]
mod combat;
#[path = "ai_decisions/copies.rs"]
mod copies;
#[path = "ai_decisions/counterspells.rs"]
mod counterspells;
#[path = "ai_decisions/graveyard.rs"]
mod graveyard;
#[path = "ai_decisions/mana.rs"]
mod mana;
#[path = "ai_decisions/randomness.rs"]
mod randomness;
#[path = "ai_decisions/stack.rs"]
mod stack;
#[path = "ai_decisions/time_counters.rs"]
mod time_counters;
