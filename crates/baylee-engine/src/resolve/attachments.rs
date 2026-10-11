//! Resolution-time attachment, including untargeted event subjects.
use super::{Resolution, TargetSpec, event_object_identity, subjects, zones};
use crate::{object::Status, state::GameState, zone::Zone};
use baylee_core::{generated::subtypes, ids::DamageSourceRef, types::TypeSet};

pub(super) fn attach(
    state: &mut GameState,
    res: &Resolution,
    target: TargetSpec,
    reconfigure: bool,
) {
    let Some(source) =
        subjects::source(state, res).filter(|identity| subjects::is_current(state, *identity))
    else {
        return;
    };
    let host = if matches!(target, TargetSpec::EventObject) {
        res.event_object
            .zip(event_object_identity(state, res))
            .map(|(object, (version, _))| DamageSourceRef { object, version })
            .filter(|identity| subjects::is_current(state, *identity))
            .map(|identity| identity.object)
    } else {
        zones::spec_object(state, res, target)
    };
    let Some(host) = host else {
        return;
    };
    attach_to(state, source, host, reconfigure);
}

pub(super) fn attach_to(
    state: &mut GameState,
    source: DamageSourceRef,
    host: baylee_core::ids::ObjectId,
    reconfigure: bool,
) {
    if !subjects::is_current(state, source) {
        return;
    }
    let Some(object) = state.object(source.object) else {
        return;
    };
    // An Aura spell records its intended host before its normal entry procedure.
    if object.zone == Zone::Stack {
        state.attach(source.object, host);
        return;
    }
    if object.zone != Zone::Battlefield
        || (!reconfigure
            && object
                .characteristics()
                .types
                .intersects(TypeSet::CREATURE.union(TypeSet::BATTLE)))
        || source.object == host
        || object.status.contains(Status::PHASED_OUT)
    {
        return;
    }
    let legal = if object
        .characteristics()
        .subtypes
        .contains(subtypes::enchantment::AURA)
    {
        crate::aura_bindings::can_attach(state, source.object, host)
    } else {
        let required = if object
            .characteristics()
            .subtypes
            .contains(subtypes::artifact::EQUIPMENT)
        {
            TypeSet::CREATURE
        } else if object
            .characteristics()
            .subtypes
            .contains(subtypes::artifact::FORTIFICATION)
        {
            TypeSet::LAND
        } else {
            return;
        };
        state.object(host).is_some_and(|host| {
            host.zone == Zone::Battlefield
                && host.characteristics().types.contains(required)
                && !host.status.contains(Status::PHASED_OUT)
        }) && !crate::eval::protected_from(state, host, source.object)
    };
    if legal {
        state.attach(source.object, host);
    }
}
