//! Exact activation-event countering and shared historical combat removal.
use super::Resolution;
use crate::{object::Rider, state::GameState};
use baylee_core::ids::ObjectId;

pub(super) fn counter(state: &mut GameState, res: &mut Resolution) {
    let reference = state.object(res.on_stack).and_then(|object| {
        object.riders.iter().find_map(|rider| match rider {
            Rider::EventAbility(reference) => Some(*reference),
            _ => None,
        })
    });
    if let Some(reference) = reference
        && state
            .object(reference.object)
            .is_some_and(|object| object.version == reference.version)
    {
        super::zones::counter_ability(state, res, reference.object);
    }
}

pub(super) fn remove_from_combat(state: &mut GameState, target: ObjectId, unblock: bool) {
    let was_blocking = state.combat.blocked_by(target);
    if let Some(reference) = state.source_identity(target) {
        state.combat.remove_defender_from_combat(reference);
    }
    state.combat.remove_from_combat(target);
    if unblock {
        for attacker in was_blocking {
            if state.blocked_only_by(attacker, target) {
                state.combat.unblock(attacker);
            }
        }
    }
}
