//! Combat facts bounded by an actual upkeep, independent of trigger resolution.
use crate::state::GameState;
use crate::turn::Step;
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};

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
