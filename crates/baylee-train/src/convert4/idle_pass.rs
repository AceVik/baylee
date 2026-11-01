//! Shared idle-priority classification for external behavior-cloning labels.

use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;

pub(super) fn is_idle_pass(pending: &Pending, view: &PlayerView, action: &PlayerAction) -> bool {
    matches!(action, PlayerAction::PassPriority)
        && view.casting.is_none()
        && view.announced_cast.is_none()
        && view.owed.is_none()
        && matches!(pending, Pending::Priority { legal, .. }
            if legal.lands.is_empty()
                && legal.castable.is_empty()
                && legal.suspendable.is_empty()
                && legal.payable.is_empty())
}

pub(super) fn live_weight(pending: &Pending, view: &PlayerView, action: &PlayerAction) -> f32 {
    if !is_idle_pass(pending, view, action) {
        1.0
    } else if view.active != view.seat && view.stack.is_empty() {
        0.0
    } else {
        0.15
    }
}

pub(super) fn llm_weight(pending: &Pending, view: &PlayerView, action: &PlayerAction) -> f32 {
    if is_idle_pass(pending, view, action) {
        0.25
    } else {
        1.0
    }
}

/// A watching model's label (an LLM shadow teacher): an idle pass weighs as an LLM
/// seat's own does, and so does mana it would make with nothing being cast or
/// owed. Mana floated ahead of a spell the policy then never casts, or one land
/// for another at plain priority, says little; a mana ability that pays what a
/// cast owes is a real payment choice and keeps its weight.
pub(super) fn llm_shadow_weight(
    pending: &Pending,
    view: &PlayerView,
    teacher: &PlayerAction,
) -> f32 {
    let floated = matches!(teacher, PlayerAction::ActivateManaAbility { .. })
        && view.casting.is_none()
        && view.announced_cast.is_none()
        && view.owed.is_none();
    if floated {
        0.25
    } else {
        llm_weight(pending, view, teacher)
    }
}

#[cfg(test)]
mod tests;
