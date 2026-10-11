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
    let (player, cards) = next_asked(state, discards, res.controller)?;
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
    controller: PlayerId,
) -> Option<(PlayerId, Vec<ObjectId>)> {
    let player = discards
        .iter()
        .find(|(card, player)| {
            !state.discard_answered(*card)
                && (state.discard_top_source(*player).is_some()
                    || state.discard_reveal_source(*player, controller).is_some())
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
    if let Some((player, asked)) = next_asked(state, discards, res.controller) {
        let top = piles.get(1).cloned().unwrap_or_default();
        let top_set: std::collections::BTreeSet<_> = top.iter().copied().collect();
        let graveyard: Vec<_> = asked
            .into_iter()
            .filter(|card| !top_set.contains(card))
            .collect();
        let academy = state
            .discard_reveal_source(player, res.controller)
            .is_some();
        if academy && !top.is_empty() && state.discard_top_source(player).is_some() {
            return offer_reveal(
                state,
                res,
                DiscardReveal {
                    player,
                    discards: discards.to_vec(),
                    then,
                    graveyard,
                    top,
                    revealed: std::collections::BTreeSet::new(),
                    answered: 0,
                },
            );
        }
        let revealed = if academy {
            top_set
        } else {
            std::collections::BTreeSet::new()
        };
        state.record_discard_answers(&graveyard, &top, &revealed);
    }
    after_destinations(state, res, discards, then)
}

fn after_destinations(
    state: &mut GameState,
    res: &mut Resolution,
    discards: &[(ObjectId, PlayerId)],
    then: DiscardThen,
) -> Flow {
    let since = state.journal.last_seq();
    if let Some(pending) = ask_discard_destination(state, res, discards, then.clone()) {
        return Flow::Wait(pending);
    }
    match then {
        DiscardThen::Cards => {
            for &(card, player) in discards {
                if state.zones.list(ZoneLocation::Hand(player)).contains(&card) {
                    state.discard_card(card, player, Cause::Effect, Some(res.controller));
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
        state.discard_card(card, player, Cause::Effect, Some(res.controller));
    }
    None
}

/// Freeze participants and inability before choices, in APNAP order.
pub(super) fn start_chain(
    state: &mut GameState,
    res: &mut Resolution,
    who: PlayerRel,
    count: u8,
    reward_unable: bool,
) -> Option<Pending> {
    if count == 0 {
        return None;
    }
    let mut remaining = players_of(who, state, res.controller, res);
    remaining.retain(|player| !state.has_left(*player));
    let seats = state.players.len();
    remaining.sort_by_key(|player| {
        (usize::from(player.get()) + seats - usize::from(state.turn.active.get())) % seats
    });
    let draws = if reward_unable {
        remaining
            .iter()
            .filter(|p| state.zones.list(ZoneLocation::Hand(**p)).is_empty())
            .count() as u32
    } else {
        0
    };
    remaining.retain(|player| !state.zones.list(ZoneLocation::Hand(*player)).is_empty());
    remaining.reverse();
    next_in_chain(state, res, count, remaining, Vec::new(), draws)
}

#[allow(clippy::too_many_arguments)] // frozen continuation operands
pub(super) fn resume_chain(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerId,
    count: u8,
    remaining: Vec<PlayerId>,
    mut selected: Vec<(ObjectId, PlayerId)>,
    draws: u32,
    chosen: &[ObjectId],
    since: u64,
) -> Flow {
    selected.extend(chosen.iter().map(|card| (*card, player)));
    if let Some(pending) = next_in_chain(state, res, count, remaining, selected, draws) {
        next_choice(state, res, since, pending)
    } else {
        finish_choice(state, res, since)
    }
}

fn next_in_chain(
    state: &mut GameState,
    res: &mut Resolution,
    count: u8,
    mut remaining: Vec<PlayerId>,
    selected: Vec<(ObjectId, PlayerId)>,
    draws: u32,
) -> Option<Pending> {
    while let Some(player) = remaining.pop() {
        let options = state.zones.list(ZoneLocation::Hand(player)).clone();
        if options.is_empty() || state.has_left(player) {
            continue;
        }
        let n = usize::from(count).min(options.len()) as u8;
        res.awaiting = Some(AwaitingOp::DiscardChain {
            player,
            count,
            remaining,
            selected,
            draws,
        });
        return Some(Pending::ChooseCards {
            player,
            options,
            min: n,
            max: n,
            prompt: ChoicePrompt::Generic,
            total: None,
        });
    }
    if let Some(question) = ask_discard_destination(
        state,
        res,
        &selected,
        DiscardThen::Choice {
            op: Box::new(AwaitingOp::DiscardChain {
                player: res.controller,
                count,
                remaining,
                selected: selected.clone(),
                draws,
            }),
            chosen: Vec::new(),
        },
    ) {
        return Some(question);
    }
    for (card, player) in selected {
        if state.zones.list(ZoneLocation::Hand(player)).contains(&card) {
            state.discard_card(card, player, Cause::Effect, Some(res.controller));
        }
    }
    state.forget_discard_answers();
    if draws > 0 {
        state.draw_cards(res.controller, draws as usize);
    }
    None
}

/// Optional public disclosure when both library replacement effects apply.
#[derive(Clone, Debug)]
pub struct DiscardReveal {
    player: PlayerId,
    discards: Vec<(ObjectId, PlayerId)>,
    then: DiscardThen,
    graveyard: Vec<ObjectId>,
    top: Vec<ObjectId>,
    revealed: std::collections::BTreeSet<ObjectId>,
    answered: usize,
}

pub(super) fn resume_reveal(
    state: &mut GameState,
    res: &mut Resolution,
    mut choice: DiscardReveal,
    chosen: &[ObjectId],
) -> Flow {
    choice.revealed.extend(chosen.iter().copied());
    choice.answered = choice
        .answered
        .saturating_add(usize::from(u8::MAX))
        .min(choice.top.len());
    offer_reveal(state, res, choice)
}

fn offer_reveal(state: &mut GameState, res: &mut Resolution, choice: DiscardReveal) -> Flow {
    if choice.answered < choice.top.len() && !state.has_left(choice.player) {
        let end = (choice.answered + usize::from(u8::MAX)).min(choice.top.len());
        let options = choice.top[choice.answered..end].to_vec();
        let max = options.len() as u8;
        let player = choice.player;
        res.awaiting = Some(AwaitingOp::RevealDiscard(Box::new(choice)));
        return Flow::Wait(Pending::ChooseCards {
            player,
            options,
            min: 0,
            max,
            prompt: ChoicePrompt::RevealDiscard,
            total: None,
        });
    }
    state.record_discard_answers(&choice.graveyard, &choice.top, &choice.revealed);
    after_destinations(state, res, &choice.discards, choice.then)
}
