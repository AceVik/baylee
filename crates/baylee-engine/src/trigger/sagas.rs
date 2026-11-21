//! Every positive lore placement can cross chapter thresholds (CR 715.2b,
//! CR 715.2c, CR 702.155a).
use super::PendingTrigger;
use crate::event::{GameEvent, JournalEntry};
use crate::object::Status;
use crate::state::{CardLookup, GameState};
use crate::zone::Zone;
use baylee_cards_dsl::{AbilityDef, CounterKind, KeywordSet, TriggerEventKind};
use baylee_core::ids::DamageSourceRef;

pub(super) fn collect(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    let turn_start = state
        .players
        .get(usize::from(state.turn.active.get()))
        .map_or(0, |p| p.turn_start_timestamp);
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
        let entered_this_turn = state.per_turn.entered_battlefield.contains(&object)
            && (turn_start == 0 || source.timestamp > turn_start);
        let read_ahead_entry_turn = source
            .characteristics()
            .keywords
            .contains(KeywordSet::READ_AHEAD)
            && entered_this_turn;
        let abilities = source.ability_list(lookup);
        let text = super::object_text(state, source);
        let times = super::times_triggered_with_source(state, TriggerEventKind::Any, source);
        for (index, ability) in abilities.abilities.iter().enumerate() {
            let chapters = ability.saga_chapters();
            if chapters.is_empty() {
                continue;
            }
            let condition = match ability {
                AbilityDef::Triggered { condition, .. }
                | AbilityDef::ModalTriggered { condition, .. } => *condition,
                _ => None,
            };
            let once_per_turn = match ability {
                AbilityDef::Triggered { once_per_turn, .. }
                | AbilityDef::ModalTriggered { once_per_turn, .. } => *once_per_turn,
                _ => false,
            };
            let context = super::ability_context(state, source, &abilities, index);
            if !super::condition_matches(
                state,
                condition,
                source.controller,
                context,
                Some(version),
                Some(entry),
            ) {
                continue;
            }
            for &chapter in chapters {
                let ch = u16::from(chapter);
                if old >= ch || ch > new {
                    continue;
                }
                // CR 702.155a: on the turn a Saga with read ahead entered the
                // battlefield, its chapter abilities cannot trigger unless the
                // Saga has the exact number of lore counters named by that
                // chapter symbol.
                if read_ahead_entry_turn && ch != new {
                    continue;
                }
                for _ in 0..times {
                    triggers.push(saga_trigger(
                        source,
                        version,
                        index as u32,
                        &abilities,
                        text,
                        once_per_turn,
                    ));
                }
            }
        }
    }
}

fn saga_trigger(
    source: &crate::object::GameObject,
    version: u32,
    ability_index: u32,
    abilities: &crate::object::AbilityList,
    text: crate::text_changes::TextChangeMap,
    once_per_turn: bool,
) -> PendingTrigger {
    PendingTrigger {
        source: source.id,
        counter_source_version: Some(version),
        related_objects: None,
        grantor: None,
        event_subject: None,
        event_ability: None,
        event_object_identity: None,
        source_version: Some(version),
        ability_index,
        abilities: Some(abilities.clone()),
        text,
        controller: source.controller,
        timestamp: source.timestamp,
        event_object: None,
        event_mana_value: None,
        event_tap_snapshot: None,
        event_departure: None,
        event_mana: None,
        event_amount: None,
        event_player: None,
        implicit_target: None,
        synthetic_effects: None,
        synthetic_target: None,
        synthetic_condition: None,
        once_per_turn,
        chosen_mode: None,
        in_chosen_order: false,
    }
}
