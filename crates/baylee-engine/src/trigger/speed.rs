//! The source-free speed trigger, captured when an opponent actually loses life.

use super::PendingTrigger;
use crate::event::{GameEvent, JournalEntry};
use crate::text_changes::TextChangeMap;
use baylee_cards_dsl::Effect;
use baylee_core::ids::{AbilityRef, ObjectId};

static EFFECTS: &[Effect] = &[Effect::IncreaseSpeed];

pub(super) fn collect(events: &[JournalEntry], triggers: &mut Vec<PendingTrigger>) {
    for entry in events {
        let GameEvent::SpeedTriggered { player } = entry.event else {
            continue;
        };
        triggers.push(PendingTrigger {
            text: TextChangeMap::IDENTITY,
            source_version: None,
            related_objects: None,
            event_subject: None,
            event_ability: None,
            event_object_identity: None,
            counter_source_version: None,
            event_mana: None,
            event_mana_value: None,
            event_departure: None,
            event_amount: None,
            event_player: Some(player),
            source: ObjectId::NO_SOURCE,
            ability_index: AbilityRef::SYNTHETIC,
            abilities: None,
            controller: player,
            timestamp: 0,
            event_object: None,
            implicit_target: None,
            synthetic_effects: Some(EFFECTS),
            once_per_turn: false,
            synthetic_target: None,
            chosen_mode: None,
        });
    }
}
