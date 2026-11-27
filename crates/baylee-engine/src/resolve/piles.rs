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
    if let Some(AwaitingOp::TwoPilesChoosePile(split)) = res
        .awaiting
        .take_if(|op| matches!(op, AwaitingOp::TwoPilesChoosePile(_)))
    {
        let since = state.journal.last_seq();
        super::wave13a::two_piles_finish(state, res, *split, index);
        return finish_choice(state, res, since);
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

/// Resumes `Effect::ChoosePlayerFrom` with the player its controller chose:
/// the rest of the resolution reads that player as `PlayerRel::Chosen`.
///
/// # Panics
/// If no such choice is suspended.
#[allow(clippy::too_many_lines)]
pub fn resume_chosen_player(state: &mut GameState, res: &mut Resolution, chosen: PlayerId) -> Flow {
    match res.awaiting.take() {
        Some(AwaitingOp::ChoosePlayerFrom) => {
            super::subjects::begin_resume(state, res);
            res.chosen_player = Some(chosen);
            res.pc += 1;
            let flow = super::run(state, res);
            super::subjects::flush(state, res);
            flow
        }
        Some(AwaitingOp::ClashOpponent { on_win, on_loss }) => {
            super::subjects::begin_resume(state, res);
            let flow = super::clash::resume_opponent(state, res, chosen, on_win, on_loss);
            super::subjects::flush(state, res);
            flow
        }
        Some(AwaitingOp::TwoPilesPickSeparator(spec)) => {
            let since = state.journal.last_seq();
            super::subjects::begin_resume(state, res);
            let flow = if let Some(pending) =
                super::wave13a::ask_two_piles_separator(state, res, chosen, *spec)
            {
                super::next_choice(state, res, since, pending)
            } else {
                super::finish_choice(state, res, since)
            };
            super::subjects::flush(state, res);
            flow
        }
        Some(AwaitingOp::TwoPilesPickChooser(split)) => {
            let since = state.journal.last_seq();
            super::subjects::begin_resume(state, res);
            let pending = super::wave13a::ask_two_piles_chooser(state, res, chosen, *split);
            let flow = super::next_choice(state, res, since, pending);
            super::subjects::flush(state, res);
            flow
        }
        Some(AwaitingOp::PutAttachedHostPlayer(spec)) => {
            let since = state.journal.last_seq();
            super::subjects::begin_resume(state, res);
            let flow = if let Some(pending) =
                super::wave15b::resume_put_attached_player(state, res, *spec, chosen)
            {
                super::next_choice(state, res, since, pending)
            } else if matches!(res.awaiting, Some(AwaitingOp::Entry { .. })) {
                Flow::Enter
            } else {
                super::finish_choice(state, res, since)
            };
            super::subjects::flush(state, res);
            flow
        }
        Some(AwaitingOp::CreateTokenAttachedPlayer {
            token,
            count,
            is_aura,
        }) => {
            let since = state.journal.last_seq();
            super::subjects::begin_resume(state, res);
            super::wave15b::resume_create_token_attached_player(
                state, res, token, count, is_aura, chosen,
            );
            let flow = super::finish_choice(state, res, since);
            super::subjects::flush(state, res);
            flow
        }
        Some(AwaitingOp::DemonstrateOpponent) => {
            super::subjects::begin_resume(state, res);
            let flow = super::immediate::resume_demonstrate_opponent(state, res, chosen);
            super::subjects::flush(state, res);
            flow
        }
        Some(AwaitingOp::ChoosePlayerEx { remember, then }) => {
            super::subjects::begin_resume(state, res);
            let flow = super::player_number_choices::resume_choose_player_ex(
                state, res, chosen, remember, then,
            );
            super::subjects::flush(state, res);
            flow
        }
        Some(AwaitingOp::SecretGuessPickGuesser {
            min,
            max,
            track_used_on_source,
            on_correct,
            on_wrong,
        }) => {
            super::subjects::begin_resume(state, res);
            let flow = super::player_number_choices::resume_secret_guess_guesser(
                state,
                res,
                chosen,
                min,
                max,
                track_used_on_source,
                on_correct,
                on_wrong,
            );
            super::subjects::flush(state, res);
            flow
        }
        _ => panic!("no player choice suspended"),
    }
}

/// Resumes `Effect::CreateTokenAttacking` once its controller named the
/// player a token attacks (CR 508.4).
///
/// # Panics
/// If no such question is suspended.
#[must_use]
pub fn resume_attacking_player(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: PlayerId,
) -> Flow {
    let Some(AwaitingOp::AttackingTokens(tokens)) = res.awaiting.take() else {
        panic!("no attacking-token question suspended");
    };
    let since = state.journal.last_seq();
    super::subjects::begin_resume(state, res);
    let flow = if let Some(pending) =
        super::attacking_tokens::resume_player(state, res, *tokens, chosen)
    {
        super::next_choice(state, res, since, pending)
    } else {
        res.pc += 1;
        super::run(state, res)
    };
    super::subjects::flush(state, res);
    flow
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
