//! Facts bounded by an actual upkeep, independent of trigger resolution.
use crate::state::GameState;
use crate::turn::Step;
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};

/// Echo's intervening `if` (CR 702.30a), asked as the ability would trigger
/// and again as it resolves (CR 603.4): the permanent is on the battlefield
/// under `you`, and came under `you` after your last upkeep began. A change
/// of control restarts [`crate::object::GameObject::controlled_since`] in
/// either direction, so a permanent handed back has come under its owner's
/// control again and owes its echo again.
///
/// A permanent taken from `you` in response still "came under your control",
/// and the answer is `false` all the same: that ability could only ask for
/// mana and then sacrifice nothing (CR 701.21a), and removing it from the
/// stack is the same game without the question. `version` is the incarnation
/// that triggered, once there is one: a permanent that left and came back is
/// a new object this ability is not about (CR 400.7), with a
/// `controlled_since` of its own that would otherwise pass.
pub(super) fn came_under_your_control(
    state: &GameState,
    you: PlayerId,
    source: ObjectId,
    version: Option<u32>,
) -> bool {
    state.object(source).is_some_and(|object| {
        object.zone == crate::zone::Zone::Battlefield
            && !object.status.contains(crate::object::Status::PHASED_OUT)
            && version.is_none_or(|version| object.version == version)
            && object.controller == you
            && object.controlled_since > state.players[usize::from(you.get())].last_upkeep_began
    })
}

pub(super) fn blocked_since_last_upkeep(
    state: &GameState,
    you: PlayerId,
    source: ObjectId,
) -> bool {
    let Some(object) = state.object(source) else {
        return false;
    };
    let reference = DamageSourceRef {
        object: source,
        version: object.version,
    };
    let player = &state.players[usize::from(you.get())];
    player.blocked_since_upkeep.contains(&reference)
        || (state.turn.active == you
            && state.turn.step == Step::Upkeep
            && player.blocked_before_upkeep.contains(&reference))
}
