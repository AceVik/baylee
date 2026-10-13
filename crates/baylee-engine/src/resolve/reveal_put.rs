//! "Reveal that many cards from the top of your library. You may put a
//! creature card and/or a land card from among them onto the battlefield.
//! Put the rest on the bottom in a random order." (Ojer Kaslem, Deepest
//! Growth; `Effect::RevealTopPutOntoBattlefield`.)
//!
//! One optional pick per filter, asked in the order the card names them,
//! each card for one filter only; the picks enter together once the last is
//! answered, and the rest go to the bottom in an order the game's seeded
//! generator chooses.

use baylee_cards_dsl::{Filter, SearchDest};
use baylee_core::ids::ObjectId;

use super::{AwaitingOp, Resolution};
use crate::choice::{ChoicePrompt, Pending};
use crate::eval;
use crate::event::{Cause, GameEvent};
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};

/// The revealed cards and the picks still to ask.
#[derive(Clone, Debug, Hash)]
pub struct Reveal {
    /// Revealed, top card first.
    cards: Vec<ObjectId>,
    /// One optional pick per filter.
    picks: &'static [&'static Filter],
    /// The next pick to ask.
    next: usize,
    /// What was picked so far.
    chosen: Vec<ObjectId>,
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    count: u32,
    picks: &'static [&'static Filter],
) -> Option<Pending> {
    let you = res.controller;
    let cards: Vec<ObjectId> = state
        .zones
        .list(ZoneLocation::Library(you))
        .iter()
        .rev()
        .take(count as usize)
        .copied()
        .collect();
    if cards.is_empty() {
        return None;
    }
    state.journal.record(GameEvent::Revealed {
        player: you,
        cards: cards.clone(),
    });
    ask(
        state,
        res,
        Reveal {
            cards,
            picks,
            next: 0,
            chosen: Vec::new(),
        },
    )
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut reveal: Reveal,
    chosen: &[ObjectId],
) -> Option<Pending> {
    if let Some(&card) = chosen.first()
        && reveal.cards.contains(&card)
        && !reveal.chosen.contains(&card)
    {
        reveal.chosen.push(card);
    }
    reveal.next += 1;
    ask(state, res, reveal)
}

fn ask(state: &mut GameState, res: &mut Resolution, mut reveal: Reveal) -> Option<Pending> {
    let you = res.controller;
    while let Some(filter) = reveal.picks.get(reveal.next) {
        let options: Vec<ObjectId> = reveal
            .cards
            .iter()
            .copied()
            .filter(|card| !reveal.chosen.contains(card))
            .filter(|card| {
                state.object(*card).is_some_and(|o| {
                    eval::matches_with_context(filter, state, o, you, res.rule_context())
                })
            })
            .collect();
        if options.is_empty() {
            reveal.next += 1;
            continue;
        }
        res.awaiting = Some(AwaitingOp::RevealPut(Box::new(reveal)));
        return Some(Pending::ChooseCards {
            player: you,
            options,
            min: 0,
            max: 1,
            prompt: ChoicePrompt::Generic,
            total: None,
        });
    }
    finish(state, res, &reveal);
    None
}

/// The picks enter together; the rest go to the bottom at random.
fn finish(state: &mut GameState, res: &Resolution, reveal: &Reveal) {
    let you = res.controller;
    let in_library = |state: &GameState, card: ObjectId| {
        state.zones.list(ZoneLocation::Library(you)).contains(&card)
    };
    for &card in &reveal.chosen {
        if in_library(state, card) {
            super::search::put_found(state, you, card, SearchDest::Battlefield);
        }
    }
    let mut rest: Vec<ObjectId> = reveal
        .cards
        .iter()
        .copied()
        .filter(|card| !reveal.chosen.contains(card) && in_library(state, *card))
        .collect();
    state.rng.shuffle(&mut rest);
    for card in rest {
        let _ = state.move_object(
            card,
            ZoneLocation::Library(you),
            ZonePosition::Bottom,
            Cause::Effect,
        );
    }
}
