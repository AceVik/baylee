//! Capture creatures whose next controller turn prohibits their attacks.
use super::{Resolution, source_version, subjects, zones};
use crate::state::{DelayedAction, DelayedTrigger, DelayedWhen, GameState};
use baylee_cards_dsl::TargetSpec;
use baylee_core::ids::DamageSourceRef;

pub(super) fn schedule(state: &mut GameState, res: &Resolution, target: TargetSpec) {
    let Some(version) = source_version(state, res) else {
        return;
    };
    let source = DamageSourceRef {
        object: res.source,
        version,
    };
    for id in zones::spec_objects(state, res, target) {
        let reference = if target == TargetSpec::EventObject {
            subjects::event(state, res)
        } else {
            subjects::target(state, res, id)
        };
        let Some(affected) = reference.filter(|r| subjects::on_battlefield(state, *r)) else {
            continue;
        };
        state.delayed.push(DelayedTrigger {
            controller: res.controller,
            when: DelayedWhen::NextAffectedTurn,
            action: DelayedAction::RestrictAttacksForTurn { source, affected },
        });
    }
}
