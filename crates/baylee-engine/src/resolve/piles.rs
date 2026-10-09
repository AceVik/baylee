//! Separating cards into piles and choosing one (Fact or Fiction and kin).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// The half of `SearchOpponentSplits` after the search: the found cards are
/// revealed, and an opponent is asked which `count` of them go to the
/// graveyard — or nobody is, when there is nothing to choose between.
pub(super) fn begin_split(
    state: &mut GameState,
    res: &mut Resolution,
    found: &[ObjectId],
    count: u8,
    library: PlayerId,
    receiver: PlayerId,
) -> Option<Pending> {
    if found.is_empty() {
        state.shuffle_library(library);
        return None;
    }
    state.journal.record(GameEvent::Revealed {
        player: receiver,
        cards: found.to_vec(),
    });
    // "An opponent chooses two of those cards": with two or fewer found,
    // every one of them is chosen; with no opponent left nobody chooses, and
    // none is.
    if found.len() <= usize::from(count) {
        finish_split(state, found, found, library, receiver);
        return None;
    }
    let opponents = eval::players(PlayerRel::Opponent, state, receiver).unwrap_or_default();
    match opponents.as_slice() {
        [] => {
            finish_split(state, found, &[], library, receiver);
            None
        }
        [only] => Some(ask_splitter(
            res,
            *only,
            found.to_vec(),
            count,
            library,
            receiver,
        )),
        _ => {
            res.awaiting = Some(AwaitingOp::PickSplitter {
                found: found.to_vec(),
                count,
                library,
                receiver,
            });
            Some(Pending::ChoosePlayer {
                player: res.controller,
                options: opponents,
            })
        }
    }
}

/// Asks `opponent` to separate the revealed cards: the ones named are the
/// first pile, the rest the second.
pub(super) fn ask_separator(
    res: &mut Resolution,
    opponent: PlayerId,
    cards: Vec<ObjectId>,
) -> Pending {
    let n = u8::try_from(cards.len()).unwrap_or(u8::MAX);
    res.awaiting = Some(AwaitingOp::FirstPile {
        cards: cards.clone(),
    });
    Pending::ChooseCards {
        player: opponent,
        options: cards,
        min: 0,
        max: n,
        prompt: ChoicePrompt::FirstPile,
        total: None,
    }
}

/// Resumes a pile choice: the pile at `index` goes into its cards' owners'
/// hands (the controller's, whose library they were revealed from), every
/// other pile into the graveyard. Only cards still in the library move.
///
/// # Panics
/// If no pile choice is suspended.
#[must_use]
pub fn resume_pile(state: &mut GameState, res: &mut Resolution, index: usize) -> Flow {
    subjects::begin_resume(state, res);
    let flow = resume_pile_inner(state, res, index);
    subjects::flush(state, res);
    flow
}

pub(super) fn resume_pile_inner(state: &mut GameState, res: &mut Resolution, index: usize) -> Flow {
    if let Some(AwaitingOp::LabelAttacker {
        attacker,
        rest,
        left,
        right,
    }) = res
        .awaiting
        .take_if(|op| matches!(op, AwaitingOp::LabelAttacker { .. }))
    {
        return super::river::resume_label(state, res, (attacker, rest, left, right), index);
    }
    let since = state.journal.last_seq();
    let Some(AwaitingOp::TakePile { piles }) = res.awaiting.take() else {
        panic!("pile choice not suspended");
    };
    for (i, pile) in piles.iter().enumerate() {
        for &card in pile {
            let Some(owner) = state
                .object(card)
                .filter(|o| o.zone == crate::zone::Zone::Library)
                .map(|o| o.owner)
            else {
                continue;
            };
            let to = if i == index {
                ZoneLocation::Hand(owner)
            } else {
                ZoneLocation::Graveyard(owner)
            };
            let _ = state.move_object(card, to, ZonePosition::Top, Cause::Effect);
        }
    }
    finish_choice(state, res, since)
}

/// Asks `opponent` which `count` of the found cards go to the graveyard.
pub(super) fn ask_splitter(
    res: &mut Resolution,
    opponent: PlayerId,
    found: Vec<ObjectId>,
    count: u8,
    library: PlayerId,
    receiver: PlayerId,
) -> Pending {
    res.awaiting = Some(AwaitingOp::SplitToGraveyard {
        found: found.clone(),
        library,
        receiver,
    });
    Pending::ChooseCards {
        player: opponent,
        options: found,
        min: count,
        max: count,
        prompt: ChoicePrompt::PutIntoGraveyard,
        total: None,
    }
}

/// "Put the chosen cards into your graveyard and the rest into your hand.
/// Then shuffle." Only cards still in the library move: the answer arrives
/// after the question, and nothing in between may have left them there.
pub(super) fn finish_split(
    state: &mut GameState,
    found: &[ObjectId],
    chosen: &[ObjectId],
    library: PlayerId,
    receiver: PlayerId,
) {
    for &card in found {
        let Some(owner) = state
            .object(card)
            .filter(|o| o.zone == crate::zone::Zone::Library)
            .map(|o| o.owner)
        else {
            continue;
        };
        let to = if chosen.contains(&card) {
            ZoneLocation::Graveyard(owner)
        } else {
            ZoneLocation::Hand(receiver)
        };
        let _ = state.move_object(card, to, ZonePosition::Top, Cause::Effect);
    }
    state.shuffle_library(library);
}

/// Resumes a split search once the controller named the opponent who
/// chooses.
///
/// # Panics
/// If no splitter question is suspended.
#[must_use]
pub fn resume_pick_splitter(res: &mut Resolution, opponent: PlayerId) -> Flow {
    match res.awaiting.take() {
        Some(AwaitingOp::SacrificeOpponent { player, options }) => {
            Flow::Wait(chosen::ask_sacrifice(res, player, opponent, options))
        }
        Some(AwaitingOp::PickSplitter {
            found,
            count,
            library,
            receiver,
        }) => Flow::Wait(ask_splitter(res, opponent, found, count, library, receiver)),
        // Fact or Fiction's separator, named the same way.
        Some(AwaitingOp::PickSeparator { cards }) => {
            Flow::Wait(ask_separator(res, opponent, cards))
        }
        _ => panic!("splitter not suspended"),
    }
}
