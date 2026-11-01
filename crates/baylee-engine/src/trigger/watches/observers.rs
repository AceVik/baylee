//! Put an observed event's independent delayed ability onto the normal stack.

use crate::state::{DelayedAction, DelayedTrigger, GameState};
use crate::trigger::PendingTrigger;

pub(super) fn trigger(state: &GameState, watch: &DelayedTrigger) -> Option<PendingTrigger> {
    let DelayedAction::TriggerAbout {
        source,
        source_version,
        effects,
        text,
        object,
        version,
        subject,
    } = watch.action
    else {
        return None;
    };
    Some(PendingTrigger {
        source,
        source_version: Some(source_version),
        controller: watch.controller,
        timestamp: state.object(source).map_or(0, |source| source.timestamp),
        text,
        ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
        abilities: None,
        synthetic_effects: Some(effects),
        synthetic_target: None,
        event_object: Some(object),
        event_object_identity: Some((
            version,
            state
                .source_object(baylee_core::ids::DamageSourceRef { object, version })
                .and_then(|object| object.characteristics().power)
                .unwrap_or(0),
        )),
        event_subject: subject,
        event_ability: None,
        related_objects: None,
        counter_source_version: None,
        event_amount: None,
        event_player: None,
        event_mana: None,
        event_mana_value: None,
        event_departure: None,
        implicit_target: None,
        once_per_turn: false,
        chosen_mode: None,
    })
}
