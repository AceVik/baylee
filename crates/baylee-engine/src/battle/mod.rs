//! Battle protector designations and their lifecycle rules.

#[cfg(test)]
mod entry_tests;
pub(crate) mod sba;
#[cfg(test)]
mod startup_tests;

use crate::object::{GameObject, Rider, Status};
use crate::state::GameState;
use crate::zone::Zone;
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};
use baylee_core::types::TypeSet;

/// The designation survives copying and loss of the Battle type (CR 310.9g).
#[must_use]
pub fn designated_protector(state: &GameState, id: ObjectId) -> Option<PlayerId> {
    state.object(id)?.riders.iter().find_map(|rider| {
        if let Rider::BattleProtector(player) = rider {
            Some(*player)
        } else {
            None
        }
    })
}

/// Protector of a phased-in Battle on the battlefield, before any SBA repair.
#[must_use]
pub fn protector(state: &GameState, id: ObjectId) -> Option<PlayerId> {
    let object = state.object(id)?;
    (object.zone == Zone::Battlefield
        && !object.status.contains(Status::PHASED_OUT)
        && object.characteristics().types.contains(TypeSet::BATTLE))
    .then(|| designated_protector(state, id))
    .flatten()
}

/// Eligible designations for the current or prospective Battle (CR 310.9a).
pub(crate) fn eligible_protectors(state: &GameState, object: &GameObject) -> Vec<PlayerId> {
    let siege = is_siege(object);
    state
        .players
        .iter()
        .map(|player| player.id)
        .filter(|player| {
            !state.has_left(*player)
                && if siege {
                    state.is_opponent(*player, object.controller)
                } else {
                    *player == object.controller
                }
        })
        .collect()
}

pub(crate) fn is_siege(object: &GameObject) -> bool {
    object
        .characteristics()
        .subtypes
        .contains(baylee_core::generated::subtypes::battle::SIEGE)
}

/// Assign only this incarnation; an actual change ends its being attacked.
pub(crate) fn set_protector(state: &mut GameState, reference: DamageSourceRef, player: PlayerId) {
    if !state.object(reference.object).is_some_and(|object| {
        object.version == reference.version && object.zone == Zone::Battlefield
    }) || designated_protector(state, reference.object) == Some(player)
    {
        return;
    }
    state.combat.remove_defender_from_combat(reference);
    let object = state
        .object_mut(reference.object)
        .expect("checked incarnation");
    object
        .riders
        .retain(|rider| !matches!(rider, Rider::BattleProtector(_)));
    object.riders.push(Rider::BattleProtector(player));
}

/// Defense is copy-layer metadata on the prospective body. Basic-land typing
/// removes printed text, but not this Battle-type intrinsic ability (CR 305.7).
pub(crate) fn intrinsic_defense(object: &GameObject) -> Option<u16> {
    let characteristics = object.characteristics();
    (characteristics.types.contains(TypeSet::BATTLE) && characteristics.abilities_lost.is_none())
        .then_some(characteristics.defense)
        .flatten()
}
