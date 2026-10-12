//! The source-free, event-player-bound radiation trigger (CR 728.1).
use super::{GameState, PendingTrigger};
use crate::{
    event::{GameEvent, JournalEntry},
    text_changes::TextChangeMap,
    turn::Phase,
};
use baylee_cards_dsl::Effect;
use baylee_core::ids::{AbilityRef, ObjectId};

static EFFECTS: &[Effect] = &[Effect::Radiation];

pub(super) fn collect(
    state: &GameState,
    events: &[JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    let player = state.turn.active;
    if state.has_left(player) || state.players[usize::from(player.get())].rad == 0 {
        return;
    }
    for entry in events {
        if matches!(
            entry.event,
            GameEvent::StepChanged {
                phase: Phase::FirstMain,
                ..
            }
        ) {
            triggers.push(PendingTrigger {
                text: TextChangeMap::IDENTITY,
                source_version: None,
                related_objects: None,
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
}
