//! Suspended player decisions and exact entry payments.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn ask(
    state: &mut GameState,
    entry: Attempt,
    question: Question,
    pending: Pending,
) -> Pending {
    state.entries.pending.push_front(entry);
    state.entries.question = Some(question);
    pending
}

fn before_next_entry(state: &mut GameState) -> Option<Pending> {
    if let Some(pending) = crate::graveyard_order::pending(state) {
        state.entries.question = Some(Question::GraveyardOrder);
        return Some(pending);
    }
    equilibrium::entry_order(state)
}

fn accept_sacrifice(
    state: &mut GameState,
    entry: Attempt,
    options: Vec<ObjectId>,
    count: u8,
) -> Pending {
    let player = entry.controller;
    ask(
        state,
        entry,
        Question::AcceptSacrifice {
            options,
            count: u32::from(count),
        },
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::SacrificeToEnter { count },
            source: None,
        },
    )
}

/// Run entry work until all moves have completed or a player must decide.
pub(crate) fn advance(state: &mut GameState) -> Option<Pending> {
    'entries: loop {
        if state.entries.pending.is_empty() {
            state.entries.putter = None;
            state.entries.chosen_entry = None;
            return None;
        }
        state.refresh_characteristics();
        if let Some(pending) = before_next_entry(state) {
            return Some(pending);
        }
        let mut entry = state.entries.pending.pop_front()?;
        if state
            .object(entry.object)
            .is_none_or(|o| o.version != entry.version)
        {
            continue;
        }
        if let Some((question, pending)) = copy_question(state, &mut entry) {
            return Some(ask(state, entry, question, pending));
        }
        if !entry.denied {
            if let Some(pending) = equilibrium::replacement_order(state, &mut entry) {
                return Some(ask(state, entry, Question::Replacement, pending));
            }
            while let Some(&modifier) = entry.modifiers.get(entry.next) {
                entry.next += 1;
                let player = entry.controller;
                if let Some((question, pending)) = super::forms::question(&entry, modifier) {
                    return Some(ask(state, entry, question, pending));
                }
                match modifier {
                    EnterModifier::ChooseNumber { .. } | EnterModifier::ChooseShape(_) => {
                        unreachable!("handled entry form")
                    }
                    EnterModifier::SacrificeOrGraveyard { filter, count } => {
                        let options = options(state, &entry, filter, false);
                        if options.len() < usize::from(count) {
                            entry.denied = true;
                            break;
                        }
                        return Some(accept_sacrifice(state, entry, options, count));
                    }
                    EnterModifier::DiscardOrGraveyard(filter) => {
                        let options = options(state, &entry, filter, true);
                        if options.is_empty() {
                            entry.denied = true;
                            break;
                        }
                        return Some(ask(
                            state,
                            entry,
                            Question::Discard,
                            Pending::ChooseCards {
                                player,
                                options,
                                min: 0,
                                max: 1,
                                prompt: ChoicePrompt::CostDiscard,
                                total: None,
                            },
                        ));
                    }
                    EnterModifier::SacrificeForPowerToughness(filter) => {
                        let options = options(state, &entry, filter, false);
                        entry.sacrifices = Some(0);
                        if options.is_empty() {
                            continue;
                        }
                        let max = u32::try_from(options.len()).expect("arena fits u32");
                        return Some(ask(
                            state,
                            entry,
                            Question::SacrificeCount { options },
                            Pending::ChooseNumber {
                                player,
                                min: 0,
                                max,
                                reason: crate::choice::NumberPrompt::Sacrifices,
                            },
                        ));
                    }
                    EnterModifier::SacrificeOthers(filter) => {
                        let victims = options(state, &entry, filter, false);
                        sacrifice(state, &victims);
                        state.entries.pending.push_front(entry);
                        continue 'entries;
                    }
                    _ => {}
                }
            }
        }
        state.entries.chosen_entry = None;
        finish(state, entry);
    }
}

/// The engine validates the offered seat, cardinality and membership before entering here.
pub(crate) fn answer(state: &mut GameState, action: &PlayerAction) {
    let question = state.entries.question.take().expect("entry question");
    if equilibrium::answer_special(state, &question, action) {
        return;
    }
    if let Question::GraveyardOrder = question {
        let PlayerAction::Arrange { piles } = action else {
            unreachable!("graveyard arrangement")
        };
        crate::graveyard_order::answer(state, piles.first().map_or(&[], Vec::as_slice));
        return;
    }
    if super::forms::answer(state, &question, action) {
        return;
    }
    if let Question::Copy {
        index,
        mods,
        until_eot,
    } = question
    {
        answer_copy(state, action, index, mods, until_eot);
        return;
    }
    let mut entry = state
        .entries
        .pending
        .pop_front()
        .expect("entry continuation");
    match (question, action) {
        (Question::Number, PlayerAction::ChooseNumber(number)) => {
            entry.chosen_number = Some(u8::try_from(*number).expect("bounded choice"));
        }
        (Question::AcceptSacrifice { options, count }, PlayerAction::YesNo(accept)) => {
            if *accept {
                select(state, entry, options, count, Vec::new());
                return;
            }
            entry.denied = true;
        }
        (Question::SacrificeCount { options }, PlayerAction::ChooseNumber(count)) => {
            entry.sacrifices = Some(*count);
            if *count > 0 {
                select(state, entry, options, *count, Vec::new());
                return;
            }
        }
        (
            Question::Sacrifice {
                mut options,
                remaining,
                mut chosen,
            },
            PlayerAction::ChooseObjects { objects },
        ) => {
            chosen.extend_from_slice(objects);
            let remaining = remaining - u32::try_from(objects.len()).expect("page fits u32");
            if remaining > 0 {
                let mut used = objects.clone();
                used.sort_unstable();
                options.retain(|id| used.binary_search(id).is_err());
                select(state, entry, options, remaining, chosen);
                return;
            }
            sacrifice(state, &chosen);
        }
        (Question::Discard, PlayerAction::ChooseObjects { objects }) => {
            if let Some(&card) = objects.first() {
                if state.discard_top_source(entry.controller).is_some()
                    && !state.discard_answered(card)
                {
                    state.entries.question = Some(Question::DiscardDestination { card });
                    state.entries.pending.push_front(entry);
                    return;
                }
                let player = entry.controller;
                state.discard_card(card, player, Cause::Effect, Some(player));
            } else {
                entry.denied = true;
            }
        }
        (Question::DiscardDestination { card }, PlayerAction::Arrange { piles }) => {
            let top = piles.get(1).map_or(&[][..], Vec::as_slice);
            let graveyard = if top.contains(&card) {
                &[][..]
            } else {
                std::slice::from_ref(&card)
            };
            state.record_discard_answers(graveyard, top, &BTreeSet::new());
            state.discard_card(
                card,
                entry.controller,
                Cause::Effect,
                Some(entry.controller),
            );
        }
        _ => unreachable!("entry answer shape was validated"),
    }
    state.entries.pending.push_front(entry);
}

pub(super) fn select(
    state: &mut GameState,
    entry: Attempt,
    options: Vec<ObjectId>,
    remaining: u32,
    chosen: Vec<ObjectId>,
) {
    state.entries.question = Some(Question::Sacrifice {
        options,
        remaining,
        chosen,
    });
    state.entries.pending.push_front(entry);
}

/// Re-publish a second question of the same entry after its previous answer.
pub(crate) fn next_question(state: &GameState) -> Option<Pending> {
    let entry = state.entries.pending.front()?;
    let player = entry.controller;
    match state.entries.question.as_ref()? {
        Question::Sacrifice {
            options, remaining, ..
        } => {
            let count = u8::try_from((*remaining).min(255)).expect("bounded page");
            Some(Pending::ChooseCards {
                player,
                options: options.clone(),
                min: count,
                max: count,
                prompt: ChoicePrompt::CostSacrifice,
                total: None,
            })
        }
        Question::DiscardDestination { card } => Some(Pending::Arrange {
            player,
            cards: vec![*card],
            piles: vec![
                ArrangePile::up_to(ArrangePlace::Graveyard, 1),
                ArrangePile::up_to(ArrangePlace::LibraryTop, 1),
            ],
            prompt: ArrangePrompt::DiscardToLibrary,
        }),
        _ => None,
    }
}
