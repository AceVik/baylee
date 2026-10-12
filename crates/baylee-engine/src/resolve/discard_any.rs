//! Gather an arbitrarily large hand selection before one discard event.
use super::{AwaitingOp, Flow, Pending, Resolution};
use crate::{choice::ChoicePrompt, state::GameState, zone::ZoneLocation};
use baylee_core::ids::ObjectId;

/// A paged selection; no card moves while a page remains unanswered.
#[derive(Clone, Debug, Hash)]
pub struct Selection {
    remaining: Vec<ObjectId>,
    selected: Vec<ObjectId>,
}

pub(super) fn offer(state: &GameState, res: &mut Resolution) -> Option<Pending> {
    let remaining = state.zones.list(ZoneLocation::Hand(res.controller)).clone();
    if remaining.is_empty() {
        return None;
    }
    Some(page(
        res,
        Selection {
            remaining,
            selected: Vec::new(),
        },
    ))
}

fn page(res: &mut Resolution, mut selection: Selection) -> Pending {
    let count = selection.remaining.len().min(usize::from(u8::MAX));
    let options = selection
        .remaining
        .split_off(selection.remaining.len() - count);
    res.awaiting = Some(AwaitingOp::DiscardAny(Box::new(selection)));
    Pending::ChooseCards {
        player: res.controller,
        options,
        min: 0,
        max: u8::try_from(count).expect("bounded page"),
        prompt: ChoicePrompt::Discard,
        total: None,
    }
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
    chosen: &[ObjectId],
) -> Flow {
    selection.selected.extend_from_slice(chosen);
    if !selection.remaining.is_empty() {
        return Flow::Wait(page(res, selection));
    }
    res.awaiting = Some(AwaitingOp::DiscardThenDraw);
    super::resume::resume_inner(state, res, &selection.selected)
}
