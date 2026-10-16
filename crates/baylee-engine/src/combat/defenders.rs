//! Exact attacked-permanent identity and defending-player history.

use super::{CombatState, Defender, GameState, ObjectId, PlayerId, Status, TypeSet};
use crate::event::DamageTarget;
use crate::zone::Zone;
use baylee_core::ids::DamageSourceRef;

/// Combat-only snapshot; losing a defender does not remove its attackers.
#[derive(Clone, Copy, Debug, Hash)]
pub(crate) struct DefenderContext {
    pub attacker: DamageSourceRef,
    pub target: Option<DamageSourceRef>,
    pub defending_player: Option<PlayerId>,
    pub controller: Option<PlayerId>,
    /// Types at the most recent valid projection, for type-loss transitions.
    pub types: TypeSet,
    pub active: bool,
}

impl CombatState {
    pub(crate) fn defender_contexts(&self) -> impl ExactSizeIterator<Item = &DefenderContext> {
        self.defender_contexts.values()
    }

    /// Stop attacking this incarnation without removing any attacking creature.
    pub(crate) fn remove_defender_from_combat(&mut self, reference: DamageSourceRef) {
        for context in self.defender_contexts.values_mut() {
            if context.target == Some(reference) {
                context.active = false;
            }
        }
    }

    /// Historical declarations do not make a permanent currently attacked.
    pub(crate) fn is_being_attacked(&self, reference: DamageSourceRef) -> bool {
        self.defender_contexts.values().any(|context| {
            context.active
                && context.target == Some(reference)
                && self.is_attacking(context.attacker.object)
        })
    }

    fn defender_context(&self, attacker: ObjectId) -> Option<&DefenderContext> {
        self.defender_contexts.get(&attacker)
    }
}

/// Record a declaration or an effect putting this creature into combat.
pub(crate) fn remember_attack(state: &mut GameState, attacker: ObjectId, defender: Defender) {
    let Some(attacker_ref) = state.source_identity(attacker) else {
        return;
    };
    let Some(player) = super::defending_player(state, defender) else {
        return;
    };
    let target = match defender {
        Defender::Player(_) => None,
        Defender::Planeswalker(id) | Defender::Battle(id) => state.source_identity(id),
    };
    let object = target.and_then(|reference| state.object(reference.object));
    let context = DefenderContext {
        attacker: attacker_ref,
        target,
        defending_player: Some(player),
        controller: object.map(|object| object.controller),
        types: object.map_or(TypeSet::EMPTY, |object| object.characteristics().types),
        active: true,
    };
    state
        .combat
        .remember_defender(attacker, attacker_ref.version, defender);
    state.combat.defender_contexts.insert(attacker, context);
}

/// Who could block this attacker, including after its defender leaves combat.
pub(crate) fn historical_defending_player(
    state: &GameState,
    attacker: ObjectId,
) -> Option<PlayerId> {
    if let Some(context) = state.combat.defender_context(attacker) {
        if state.object(attacker).is_some_and(|object| {
            object.zone == Zone::Battlefield && object.version != context.attacker.version
        }) {
            return None;
        }
        if target_is_active(state, context)
            && let Some(reference) = context.target
            && state
                .object(reference.object)
                .is_some_and(|object| object.characteristics().types.contains(TypeSet::BATTLE))
        {
            // A planeswalker can become a battle while already attacked;
            // the missing-protector SBA waits until it is no longer attacked.
            return crate::battle::designated_protector(state, reference.object);
        }
        return context.defending_player;
    }
    // Legacy synthetic combat fixtures have only declaration data.
    let defender = state.combat.defender_of(attacker).or_else(|| {
        state
            .combat
            .remembered_defender(attacker)
            .map(|(_, defender)| defender)
    })?;
    super::blocking_player(state, defender)
}

fn target_is_active(state: &GameState, context: &DefenderContext) -> bool {
    if !context.active {
        return false;
    }
    let Some(reference) = context.target else {
        return context
            .defending_player
            .is_some_and(|player| !state.has_left(player));
    };
    let Some(object) = state.object(reference.object) else {
        return false;
    };
    if object.version != reference.version
        || object.zone != Zone::Battlefield
        || object.status.contains(Status::PHASED_OUT)
        || Some(object.controller) != context.controller
    {
        return false;
    }
    let types = object.characteristics().types;
    if context.types.contains(TypeSet::BATTLE) {
        if types.contains(TypeSet::BATTLE) {
            return true;
        }
        // The exception uses the types immediately before this transition,
        // including a second type gained after attackers were declared.
        return context.types.contains(TypeSet::PLANESWALKER)
            && types.contains(TypeSet::PLANESWALKER)
            && crate::battle::designated_protector(state, object.id) == Some(object.controller);
    }
    types.contains(TypeSet::PLANESWALKER)
}

/// A removed defender never redirects unblocked damage to its former protector.
pub(crate) fn active_damage_recipient(
    state: &GameState,
    attacker: ObjectId,
) -> Option<DamageTarget> {
    if let Some(context) = state.combat.defender_context(attacker) {
        if !target_is_active(state, context) {
            return None;
        }
        return match context.target {
            Some(reference) => Some(DamageTarget::Object(reference.object)),
            None => context.defending_player.map(DamageTarget::Player),
        };
    }
    match state.combat.defender_of(attacker)? {
        Defender::Player(player) => {
            (!state.has_left(player)).then_some(DamageTarget::Player(player))
        }
        defender @ (Defender::Planeswalker(id) | Defender::Battle(id)) => {
            super::defending_player(state, defender).map(|_| DamageTarget::Object(id))
        }
    }
}

/// Run after the layer projection has settled, never during its fixed-point walk.
pub(crate) fn recheck_defenders(state: &mut GameState) {
    if !state.combat.is_active() && state.combat.blockers.is_empty() {
        return;
    }
    let updates: Vec<_> =
        state
            .combat
            .defender_contexts
            .values()
            .filter_map(|context| {
                if !context.active {
                    return None;
                }
                let active = target_is_active(state, context);
                let types = context
                    .target
                    .and_then(|reference| state.object(reference.object))
                    .map_or(context.types, |object| object.characteristics().types);
                let player = if active && types.contains(TypeSet::BATTLE) {
                    context.target.and_then(|reference| {
                        crate::battle::designated_protector(state, reference.object)
                    })
                } else {
                    context.defending_player
                };
                (!active || types != context.types || player != context.defending_player)
                    .then_some((context.attacker.object, active, types, player))
            })
            .collect();
    for (attacker, active, types, player) in updates {
        if let Some(context) = state.combat.defender_contexts.get_mut(&attacker) {
            context.active = active;
            if active {
                context.types = types;
                context.defending_player = player;
            }
        }
    }
    let removed: Vec<_> = state
        .combat
        .attackers()
        .iter()
        .map(|a| a.creature)
        .chain(state.combat.blockers.iter().map(|b| b.blocker))
        .filter(|&id| {
            state.object(id).is_none_or(|object| {
                let types = object.characteristics().types;
                !types.contains(TypeSet::CREATURE) || types.contains(TypeSet::BATTLE)
            })
        })
        .collect();
    for id in removed {
        state.combat.remove_from_combat(id);
    }
}
