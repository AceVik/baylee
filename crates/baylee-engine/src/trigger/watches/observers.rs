//! Put an observed event's independent delayed ability onto the normal stack.

use crate::state::{DelayedAction, DelayedTrigger, GameState};
use crate::trigger::PendingTrigger;

pub(super) fn trigger(state: &GameState, watch: &DelayedTrigger) -> Option<PendingTrigger> {
    match &watch.action {
        DelayedAction::TriggerAbout {
            source,
            source_version,
            effects,
            text,
            object,
            version,
            subject,
        } => Some(PendingTrigger {
            source: *source,
            source_version: Some(*source_version),
            controller: watch.controller,
            timestamp: state.object(*source).map_or(0, |source| source.timestamp),
            text: *text,
            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
            abilities: None,
            synthetic_effects: Some(*effects),
            synthetic_target: None,
            event_object: Some(*object),
            event_object_identity: Some((
                *version,
                state
                    .source_object(baylee_core::ids::DamageSourceRef {
                        object: *object,
                        version: *version,
                    })
                    .and_then(|object| object.characteristics().power)
                    .unwrap_or(0),
            )),
            event_subject: *subject,
            event_ability: None,
            related_objects: None,
            grantor: None,
            counter_source_version: None,
            event_amount: None,
            event_player: None,
            event_mana: None,
            event_mana_value: None,
            event_tap_snapshot: None,
            event_departure: None,
            implicit_target: None,
            once_per_turn: false,
            chosen_mode: None,
            synthetic_condition: None,
            in_chosen_order: false,
        }),
        DelayedAction::GeneralTrigger(spec) => general_trigger(state, watch.controller, spec),
        _ => None,
    }
}

fn general_trigger(
    state: &GameState,
    controller: baylee_core::ids::PlayerId,
    spec: &crate::state::DelayedTriggerSpec,
) -> Option<PendingTrigger> {
    let rem = spec.remembered.unwrap_or_default();
    let event_ref = rem
        .object
        .zip(rem.identity.map(|(v, _)| v))
        .map(|(object, version)| baylee_core::ids::DamageSourceRef { object, version });
    if !crate::eval::intervening_if_for_incarnation(
        state,
        spec.condition.copied(),
        controller,
        spec.source,
        Some(spec.source_version),
        None,
        event_ref,
    ) {
        return None;
    }
    let identity = rem.identity.map(|(ver, old_power)| {
        let live_power = rem.object.and_then(|obj| {
            state
                .source_object(baylee_core::ids::DamageSourceRef {
                    object: obj,
                    version: ver,
                })
                .and_then(|o| o.characteristics().power)
        });
        (ver, live_power.unwrap_or(old_power))
    });
    Some(PendingTrigger {
        source: spec.source,
        source_version: Some(spec.source_version),
        controller,
        timestamp: state.object(spec.source).map_or(0, |s| s.timestamp),
        text: spec.text,
        ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
        abilities: None,
        synthetic_effects: Some(spec.effects),
        synthetic_target: spec.targets,
        synthetic_condition: spec.condition,
        event_object: rem.object,
        event_object_identity: identity,
        event_subject: rem.subject,
        event_ability: None,
        related_objects: None,
        grantor: None,
        counter_source_version: None,
        event_amount: rem.amount,
        event_player: rem.player,
        event_mana: None,
        event_mana_value: rem.mana_value,
        event_tap_snapshot: None,
        event_departure: rem.departure,
        implicit_target: None,
        once_per_turn: false,
        chosen_mode: None,
        in_chosen_order: false,
    })
}
