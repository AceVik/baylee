//! Minimum-power selection is a single scan and never targets a creature.
use super::{AwaitingOp, ChoicePrompt, Pending, Resolution, eval};
use crate::{sba, state::GameState};
use baylee_cards_dsl::Filter;
use baylee_core::ids::ObjectId;

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    no_regen: bool,
) -> Option<Pending> {
    let mut least = None;
    let mut options = Vec::new();
    for id in state.battlefield_seen() {
        let Some(object) = state.object(id) else {
            continue;
        };
        if !eval::matches_with_context(filter, state, object, res.controller, res.rule_context()) {
            continue;
        }
        // Keep signed power: negative powers still participate in the minimum.
        let power = object.characteristics().power.unwrap_or(0);
        if least.is_none_or(|minimum| power < minimum) {
            least = Some(power);
            options.clear();
        }
        if least == Some(power) {
            options.push(id);
        }
    }
    match options.as_slice() {
        [] => None,
        [card] => {
            destroy(state, *card, no_regen);
            None
        }
        _ => {
            res.awaiting = Some(AwaitingOp::DestroyWeakest { no_regen });
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

pub(super) fn destroy(state: &mut GameState, card: ObjectId, no_regen: bool) {
    if no_regen {
        sba::destroy_no_regen(state, card);
    } else {
        sba::destroy(state, card);
    }
}
