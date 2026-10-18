//! Entry orientation replacements, evaluated before the permanent arrives.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::eval;
use baylee_cards_dsl::PlayerRel;

pub(crate) fn handles(modifier: &EnterModifier) -> bool {
    matches!(
        modifier,
        EnterModifier::Tapped
            | EnterModifier::TappedUnless(_)
            | EnterModifier::TappedUnlessCondition(_)
            | EnterModifier::TappedUnlessCount { .. }
            | EnterModifier::TappedUnlessAtMost { .. }
            | EnterModifier::TappedUnlessOpponents { .. }
            | EnterModifier::TappedUnlessSomeoneAtOrBelow { .. }
            | EnterModifier::TappedOrPayLife(_)
            | EnterModifier::TappedUnlessReveal(_)
            | EnterModifier::TappedUnlessRevealOrControl(_)
            | EnterModifier::TappedForPlayerCounters { .. }
    )
}

fn count(state: &GameState, entry: &Attempt, filter: &Filter) -> usize {
    state
        .battlefield_seen()
        .filter(|id| {
            *id != entry.object
                && !unavailable(state).contains(id)
                && state.object(*id).is_some_and(|object| {
                    eval::matches(filter, state, object, entry.controller, entry.object)
                })
        })
        .count()
}

pub(super) fn apply(
    state: &GameState,
    entry: &mut Attempt,
    modifier: EnterModifier,
) -> Option<(Question, Pending)> {
    let player = entry.controller;
    let source = state
        .object(entry.object)
        .and_then(|o| o.card)
        .map(|c| baylee_core::ids::AbilityRef::new(c.index, baylee_core::ids::AbilityRef::ENTERS));
    let tap = match modifier {
        EnterModifier::Tapped => true,
        EnterModifier::TappedUnless(filter) => count(state, entry, filter) == 0,
        EnterModifier::TappedUnlessCount { filter, at_least } => {
            count(state, entry, filter) < usize::from(at_least)
        }
        EnterModifier::TappedUnlessAtMost { filter, at_most } => {
            count(state, entry, filter) > usize::from(at_most)
        }
        EnterModifier::TappedUnlessCondition(condition) => {
            !eval::condition_holds(state, player, entry.object, condition)
        }
        EnterModifier::TappedUnlessOpponents { at_least } => {
            eval::players(PlayerRel::Opponent, state, player).map_or(0, |players| players.len())
                < usize::from(at_least)
        }
        EnterModifier::TappedUnlessSomeoneAtOrBelow { life } => !state
            .players
            .iter()
            .any(|p| !p.has_lost() && p.life <= life),
        EnterModifier::TappedOrPayLife(amount) => {
            if state.can_pay_life(player, i32::from(amount)) {
                return Some((
                    Question::TapLife(amount),
                    Pending::YesNo {
                        player,
                        source,
                        prompt: YesNoPrompt::PayLifeOrEnterTapped { amount },
                    },
                ));
            }
            true
        }
        EnterModifier::TappedForPlayerCounters { kind, amount } => {
            return Some((
                Question::TapCounters { kind, amount },
                Pending::YesNo {
                    player,
                    source,
                    prompt: YesNoPrompt::EnterTappedForCounters { kind, amount },
                },
            ));
        }
        EnterModifier::TappedUnlessReveal(filter)
        | EnterModifier::TappedUnlessRevealOrControl(filter) => {
            let satisfied = matches!(modifier, EnterModifier::TappedUnlessRevealOrControl(_))
                && count(state, entry, filter) != 0;
            // Revealing does not move a card. Unlike an entry sacrifice/discard,
            // it may name a card that is still in hand during this entry event.
            let options: Vec<_> = state
                .zones
                .list(ZoneLocation::Hand(player))
                .iter()
                .copied()
                .filter(|id| {
                    state.object(*id).is_some_and(|object| {
                        eval::matches(filter, state, object, player, entry.object)
                    })
                })
                .collect();
            if !options.is_empty() {
                return Some((
                    Question::TapReveal { satisfied },
                    Pending::ChooseCards {
                        player,
                        options,
                        min: 0,
                        max: 1,
                        prompt: ChoicePrompt::RevealOrEnterTapped,
                        total: None,
                    },
                ));
            }
            !satisfied
        }
        _ => false,
    };
    entry.tapped |= tap;
    None
}

pub(super) fn answer(state: &mut GameState, question: &Question, action: &PlayerAction) -> bool {
    if !matches!(
        question,
        Question::TapLife(_) | Question::TapCounters { .. } | Question::TapReveal { .. }
    ) {
        return false;
    }
    let mut entry = state
        .entries
        .pending
        .pop_front()
        .expect("entry orientation question");
    match (question, action) {
        (Question::TapLife(amount), PlayerAction::YesNo(pay)) => {
            if *pay && state.can_pay_life(entry.controller, i32::from(*amount)) {
                state.change_life(entry.controller, -i32::from(*amount), Cause::Cost);
            } else {
                entry.tapped = true;
            }
        }
        (Question::TapCounters { kind, amount }, PlayerAction::YesNo(accept)) => {
            if *accept {
                entry.tapped = true;
                state.put_player_counters_by(
                    entry.controller,
                    entry.controller,
                    *kind,
                    u32::from(*amount),
                );
            }
        }
        (Question::TapReveal { satisfied }, PlayerAction::ChooseObjects { objects }) => {
            if objects.is_empty() {
                entry.tapped |= !satisfied;
            } else {
                state.journal.record(GameEvent::Revealed {
                    player: entry.controller,
                    cards: objects.clone(),
                });
            }
        }
        _ => unreachable!("validated entry orientation answer"),
    }
    state.entries.pending.push_front(entry);
    true
}
