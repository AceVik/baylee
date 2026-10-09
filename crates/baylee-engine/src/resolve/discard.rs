//! Library of Leng's question, asked before an effect's discard is made.
//!
//! "If an effect causes you to discard a card, discard it, but you may put
//! it on top of your library instead of into your graveyard." The question
//! has to come before the cards move, because a card that reached the
//! graveyard first and was then put back would be a different game (a
//! graveyard count read in between, a trigger on the arrival). So, like CR
//! 903.9b's commander question (`ask_commander_replace`), each instruction
//! that discards names its cards here before it touches them. A player who
//! controls a Library of Leng and is about to discard is asked one
//! arrangement of their cards: a graveyard pile and a library pile in any
//! order (`ArrangePrompt::DiscardToLibrary`). The answers wait in
//! `GameState::discard_answers`, and the discard spends them
//! (`GameState::discard_card`).
//!
//! What runs once everyone is answered is the instruction itself, in one of
//! two shapes ([`DiscardThen`]): the cards it chose at random or took whole
//! (re-running it would draw the random ones again), or the choice whose
//! answer discards, answered again with the same cards.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// What a discard Library of Leng was asked about does once answered.
#[derive(Clone, Debug)]
pub enum DiscardThen {
    /// Discard exactly the asked cards, then go on to the next instruction.
    Cards,
    /// Answer `op` again with `chosen`: the choice whose answer discards
    /// (a "discard N, then draw that many", a chain of players each
    /// discarding, Balance's hands). Nothing has moved yet, and the second
    /// visit finds every card answered.
    Choice {
        /// The choice the discard belongs to.
        op: Box<AwaitingOp>,
        /// Its answer.
        chosen: Vec<ObjectId>,
    },
}

/// Puts Library of Leng's question before an effect's discard of
/// `discards` (card, discarding player), and suspends if anyone has to
/// answer it. `None` when nobody does: the caller discards as printed.
///
/// **Call this before the instruction mutates anything**, for the reason
/// `ask_commander_replace` gives.
pub(super) fn ask_discard_destination(
    state: &GameState,
    res: &mut Resolution,
    discards: &[(ObjectId, PlayerId)],
    then: DiscardThen,
) -> Option<Pending> {
    let (player, cards) = next_asked(state, discards)?;
    let n = u32::try_from(cards.len()).unwrap_or(u32::MAX);
    res.awaiting = Some(AwaitingOp::DiscardDestination {
        discards: discards.to_vec(),
        then,
    });
    Some(Pending::Arrange {
        player,
        cards,
        piles: vec![
            ArrangePile::up_to(ArrangePlace::Graveyard, n),
            ArrangePile::up_to(ArrangePlace::LibraryTop, n),
        ],
        prompt: ArrangePrompt::DiscardToLibrary,
    })
}

/// The first discarding player, in the order the discards come, who
/// controls a Library of Leng and has cards not yet asked about, with
/// those cards.
fn next_asked(
    state: &GameState,
    discards: &[(ObjectId, PlayerId)],
) -> Option<(PlayerId, Vec<ObjectId>)> {
    let player = discards
        .iter()
        .find(|(card, player)| {
            !state.discard_answered(*card) && state.discard_top_source(*player).is_some()
        })
        .map(|&(_, player)| player)?;
    let cards = discards
        .iter()
        .filter(|(card, p)| *p == player && !state.discard_answered(*card))
        .map(|&(card, _)| card)
        .collect();
    Some((player, cards))
}

/// Resumes with one player's arrangement: `piles` is the graveyard pile
/// and the library pile, top first (an empty answer, a departed player's,
/// sends everything to the graveyard). The next player is asked, or the
/// instruction goes on.
pub(super) fn resume_discard_destination(
    state: &mut GameState,
    res: &mut Resolution,
    discards: &[(ObjectId, PlayerId)],
    then: DiscardThen,
    piles: &[Vec<ObjectId>],
) -> Flow {
    let since = state.journal.last_seq();
    if let Some((_, asked)) = next_asked(state, discards) {
        let top = piles.get(1).cloned().unwrap_or_default();
        let graveyard: Vec<ObjectId> = asked
            .iter()
            .copied()
            .filter(|card| !top.contains(card))
            .collect();
        state.record_discard_answers(&graveyard, &top);
    }
    if let Some(pending) = ask_discard_destination(state, res, discards, then.clone()) {
        return Flow::Wait(pending);
    }
    match then {
        DiscardThen::Cards => {
            for &(card, player) in discards {
                if state.zones.list(ZoneLocation::Hand(player)).contains(&card) {
                    state.discard_card(card, player, Cause::Effect);
                }
            }
            state.forget_discard_answers();
            super::finish_choice(state, res, since)
        }
        DiscardThen::Choice { op, chosen } => {
            res.awaiting = Some(*op);
            let flow = super::resume::resume_inner(state, res, &chosen);
            state.forget_discard_answers();
            flow
        }
    }
}

/// Discards `discards` for an instruction that is not answering a choice:
/// asks Library of Leng first if anyone has to be asked, else discards as
/// printed. `Some` is the question.
pub(super) fn discard_or_ask(
    state: &mut GameState,
    res: &mut Resolution,
    discards: &[(ObjectId, PlayerId)],
) -> Option<Pending> {
    if let Some(pending) = ask_discard_destination(state, res, discards, DiscardThen::Cards) {
        return Some(pending);
    }
    for &(card, player) in discards {
        state.discard_card(card, player, Cause::Effect);
    }
    None
}
