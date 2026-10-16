//! Protector choice and defense are decided against the prospective Battle.

use super::{Attempt, GameState, ObjectId, Pending, Question, Zone};
use crate::object::GameObject;
use baylee_cards_dsl::Modifier;
use baylee_core::types::TypeSet;

pub(super) fn needs_entry(state: &GameState, id: ObjectId) -> bool {
    state
        .object(id)
        .is_some_and(|object| object.characteristics().types.contains(TypeSet::BATTLE))
        || state.effects.iter().any(|effect| match effect.modifier {
            Modifier::AddType(types)
            | Modifier::BecomeType { types, .. }
            | Modifier::AddTypeIfCountersAtLeast { types, .. } => types.contains(TypeSet::BATTLE),
            Modifier::BecomeCopyOf(_) => true,
            _ => false,
        })
}

pub(super) fn prospective(state: &GameState, id: ObjectId) -> Option<GameObject> {
    let mut object = state.object(id)?.clone();
    object.zone = Zone::Battlefield;
    object.version = object.version.wrapping_add(1);
    let layers =
        crate::layers::LayerPlan::before_entries(&state.effects, super::unavailable(state));
    let projection = crate::layers::recompute_with(state, &object, &layers);
    object.cache.store(projection.characteristics, &object.base);
    object.controller = projection.controller;
    Some(object)
}

pub(super) fn question(state: &GameState, entry: &mut Attempt) -> Option<(Question, Pending)> {
    if entry.refusal.is_some() {
        return None;
    }
    if !needs_entry(state, entry.object) {
        entry.battle_protector = None;
        return None;
    }
    let object = prospective(state, entry.object)?;
    entry.controller = object.controller;
    if !object.characteristics().types.contains(TypeSet::BATTLE) {
        entry.battle_protector = None;
        return None;
    }
    let options = crate::battle::eligible_protectors(state, &object);
    if entry
        .battle_protector
        .is_some_and(|player| options.contains(&player))
    {
        return None;
    }
    entry.battle_protector = None;
    match options.as_slice() {
        [] => None, // It enters undesignated; the Battle SBA handles no eligible player.
        [only] => {
            entry.battle_protector = Some(*only);
            None
        }
        _ => Some((
            Question::BattleProtector,
            Pending::ChoosePlayer {
                player: entry.controller,
                options,
            },
        )),
    }
}
