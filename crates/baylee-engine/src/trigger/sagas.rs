//! Every positive lore placement can cross chapter thresholds (CR 714.2b).
use super::{PendingTrigger, TextChangeMap};
use crate::event::{GameEvent, JournalEntry};
use crate::object::Status;
use crate::state::{CardLookup, GameState};
use crate::zone::Zone;
use baylee_cards_dsl::{AbilityDef, CounterKind, TriggerEventKind};
use baylee_core::ids::DamageSourceRef;

pub(super) fn collect(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for entry in events {
        let GameEvent::CounterChanged {
            object,
            version,
            kind: CounterKind::Lore,
            old,
            new,
            ..
        } = entry.event
        else {
            continue;
        };
        if old >= new {
            continue;
        }
        let Some(source) = state.source_object(DamageSourceRef { object, version }) else {
            continue;
        };
        if source.zone != Zone::Battlefield || source.status.contains(Status::PHASED_OUT) {
            continue;
        }
        let abilities = source.ability_list(lookup);
        let times = super::times_triggered_with_source(state, TriggerEventKind::Any, source);
        for (index, ability) in abilities.abilities.iter().enumerate() {
            let AbilityDef::SagaChapter { chapter, .. } = ability else {
                continue;
            };
            if old >= u16::from(*chapter) || u16::from(*chapter) > new {
                continue;
            }
            for _ in 0..times {
                triggers.push(PendingTrigger {
                    source: object,
                    counter_source_version: Some(version),
                    related_objects: None,
                    event_object_identity: None,
                    source_version: Some(version),
                    ability_index: index as u32,
                    abilities: Some(abilities.clone()),
                    text: TextChangeMap::IDENTITY,
                    controller: source.controller,
                    timestamp: source.timestamp,
                    event_object: None,
                    event_mana_value: None,
                    event_departure: None,
                    event_mana: None,
                    event_amount: None,
                    event_player: None,
                    implicit_target: None,
                    synthetic_effects: None,
                    synthetic_target: None,
                    once_per_turn: false,
                    chosen_mode: None,
                });
            }
        }
    }
}
