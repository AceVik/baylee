//! Exact-incarnation returns, with separate stack and turn-based timing.
use super::{Resolution, source_version, subjects};
use crate::{
    event::Cause,
    object::ObjectKind,
    state::{DelayedAction, DelayedTrigger, DelayedWhen, GameState},
    zone::{Zone, ZoneLocation, ZonePosition},
};

pub(super) fn schedule_untap(state: &mut GameState, res: &Resolution) {
    let Some(source) = subjects::source(state, res)
        .filter(|reference| subjects::on_battlefield(state, *reference))
    else {
        return;
    };
    state.delayed.push(DelayedTrigger {
        controller: res.controller,
        when: DelayedWhen::NextUntap,
        action: DelayedAction::ReturnToHandDuringUntap {
            card: source.object,
            version: source.version,
        },
    });
}

pub(super) fn exile_until_end(state: &mut GameState, res: &Resolution, tapped: bool) {
    let source_version = source_version(state, res)
        .or_else(|| state.object(res.source).map(|o| o.version))
        .unwrap_or(0);
    let mut objects = Vec::with_capacity(res.targets.len());
    for &target in &res.targets {
        let Some(object) = state.object(target).filter(|o| o.zone == Zone::Battlefield) else {
            continue;
        };
        let owner = object.owner;
        if state
            .move_object(
                target,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            )
            .is_err()
        {
            continue;
        }
        let Some(object) = state.object(target).filter(|o| o.zone == Zone::Exile) else {
            continue;
        };
        objects.push(baylee_core::ids::DamageSourceRef {
            object: target,
            version: object.version,
        });
    }
    if !objects.is_empty() {
        state.delayed.push(DelayedTrigger {
            controller: res.controller,
            when: DelayedWhen::NextEndStep,
            action: DelayedAction::ReturnExiledGroup {
                source: baylee_core::ids::DamageSourceRef {
                    object: res.source,
                    version: source_version,
                },
                objects,
                tapped,
            },
        });
    }
}

pub(super) fn return_exiled(state: &mut GameState, res: &Resolution, tapped: bool) {
    let objects: Vec<_> = state
        .object(res.on_stack)
        .into_iter()
        .flat_map(|object| object.riders.iter())
        .filter_map(|rider| match rider {
            crate::object::Rider::DelayedReturnObject(reference) => Some(*reference),
            _ => None,
        })
        .collect();
    for reference in objects {
        let Some(object) = state
            .object(reference.object)
            .filter(|o| o.zone == Zone::Exile && o.version == reference.version)
        else {
            continue;
        };
        let owner = object.owner;
        if state.has_left(owner) {
            continue;
        }
        if let Some(object) = state.object_mut(reference.object) {
            object.set_controller(owner);
            object.kind = ObjectKind::Permanent;
        }
        // Establish the entry instruction before replacement effects run.
        state.set_tapped(reference.object, tapped);
        let _ = state.move_object(
            reference.object,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        );
    }
}
