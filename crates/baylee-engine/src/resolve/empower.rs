//! Empower Jace creates only when necessary, then chooses one token (CR 701.71a).
use super::{AwaitingOp, ChoicePrompt, GameState, Pending, Resolution};
use baylee_cards_dsl::{CounterKind, TokenDef};
use baylee_core::{generated::subtypes, ids::ObjectId, types::TypeSet};

fn candidates(state: &GameState, res: &Resolution) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|&id| {
            state.object(id).is_some_and(|o| {
                o.controller == res.controller
                    && o.card.is_none()
                    && o.characteristics().types.contains(TypeSet::PLANESWALKER)
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::planeswalker::JACE)
            })
        })
        .collect()
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    token: &'static TokenDef,
    amount: u16,
) -> Option<Pending> {
    if candidates(state, res).is_empty() {
        let created =
            super::tokens::create_tokens_with_text(state, res.controller, token, None, 1, res.text);
        if created.iter().any(|id| state.entries.contains(*id)) || state.entries.has_after() {
            res.awaiting = Some(AwaitingOp::Entry {
                next: None,
                after: Some(super::after_entry::AfterEntry::EmpowerJace { amount }),
            });
            return None;
        }
    }
    after_entry(state, res, amount)
}

/// Entry replacements have finished; choose among the actual surviving tokens.
pub(super) fn after_entry(
    state: &mut GameState,
    res: &mut Resolution,
    amount: u16,
) -> Option<Pending> {
    state.refresh_characteristics();
    let options = candidates(state, res);
    match options.as_slice() {
        [] => None,
        [only] => {
            finish(state, res, *only, amount);
            None
        }
        _ => {
            res.awaiting = Some(AwaitingOp::EmpowerJace { amount });
            Some(Pending::ChooseCards {
                player: res.controller,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
    }
}

pub(super) fn finish(state: &mut GameState, res: &Resolution, jace: ObjectId, amount: u16) {
    crate::replacement::put_counters_by(state, res.controller, jace, CounterKind::Loyalty, amount);
}
