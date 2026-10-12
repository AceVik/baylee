//! Sparse restoration of temporary copies, including announced spell choices.
//!
//! Only an object actually becoming a temporary copy allocates a snapshot.
//! Keeping this on the state avoids another pointer on every stack ability.

use crate::object::{GameObject, ObjectKind, PaidRecord, Rider};
use crate::state::GameState;
use baylee_core::ids::{DamageSourceRef, ObjectId};

pub(super) fn prepare(
    state: &mut GameState,
    source: ObjectId,
    target: DamageSourceRef,
    until_eot: bool,
    entering: bool,
) {
    if until_eot {
        if let Some(object) = state.object(source) {
            let identity = DamageSourceRef {
                object: source,
                version: object.version.wrapping_add(u32::from(entering)),
            };
            if !state
                .temporary_copy_origins
                .iter()
                .any(|(id, _)| *id == identity)
            {
                let original = Box::new(object.clone());
                state.temporary_copy_origins.push((identity, original));
            }
        }
    } else {
        // A permanent copy supersedes every temporary copy of this recipient.
        state
            .temporary_copy_origins
            .retain(|(id, _)| id.object != source);
    }
    let spell = state
        .source_object(target)
        .filter(|object| {
            object.zone == crate::zone::Zone::Stack && object.kind != ObjectKind::AbilityOnStack
        })
        .cloned();
    if let Some(spell) = spell
        && let Some(object) = state.object_mut(source)
    {
        copy_decisions(object, &spell);
        // Copying choices does not copy actual mana expenditure or create a cast.
        object.paid = spell.paid.as_ref().map(|paid| {
            Box::new(PaidRecord {
                sacrificed_lki: paid.sacrificed_lki,
                sacrificed: paid.sacrificed,
                source_after_cost: paid.source_after_cost,
                tapped: paid.tapped,
                ..PaidRecord::default()
            })
        });
    }
}

fn copy_decisions(object: &mut GameObject, original: &GameObject) {
    object.x_value = original.x_value;
    object.kicked = original.kicked;
    object.replicated = original.replicated;
    object.alt_cast = original.alt_cast;
    object.mode_index = original.mode_index;
    object.modes = original.modes;
    object.chosen_player = original.chosen_player;
    object.targets.clone_from(&original.targets);
    object.target_req = original.target_req;
    object.target_players = original.target_players;
    object.second.clone_from(&original.second);
}

/// Restore only copied rules and decisions; counters, damage and control persist.
pub(crate) fn restore(state: &mut GameState) {
    for (identity, original) in std::mem::take(&mut state.temporary_copy_origins) {
        let Some(object) = state
            .object_mut(identity.object)
            .filter(|object| object.version == identity.version)
        else {
            continue;
        };
        object.own_abilities.clone_from(&original.own_abilities);
        object.own_origin = original.own_origin;
        object
            .riders
            .retain(|rider| !matches!(rider, Rider::RulesAcquisition(_)));
        let acquisition = original.rules_acquisition();
        if acquisition != 0 {
            object.riders.push(Rider::RulesAcquisition(acquisition));
        }
        object.refresh_chosen_name();
        copy_decisions(object, &original);
        object.paid = original.paid;
    }
}
