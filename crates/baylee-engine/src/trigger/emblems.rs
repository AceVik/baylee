//! Trigger collection for command-zone emblems.
#[allow(clippy::wildcard_imports)] // The parent's trigger vocabulary.
use super::*;

pub(super) fn collect_emblems(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    // Mulligan redraws remain in the journal for replay and presentation.
    // They happened before the first turn, outside the emblem's gameplay.
    let first_turn = events
        .iter()
        .position(|entry| matches!(entry.event, GameEvent::TurnStarted { number: 1, .. }));
    // Emblems (command zone, CR 114.2): their triggered abilities fire
    // from the command zone.
    for seat in 0..state.players.len() {
        let p = PlayerId::new(seat as u8);
        for &emblem in state.zones.list(ZoneLocation::Command(p)) {
            let Some(obj) = state
                .object(emblem)
                .filter(|o| o.kind == crate::object::ObjectKind::Emblem)
            else {
                continue;
            };
            collect_emblem_triggers(state, lookup, events, first_turn, emblem, obj, triggers);
        }
    }
}

fn collect_emblem_triggers(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[crate::event::JournalEntry],
    first_turn: Option<usize>,
    emblem: ObjectId,
    obj: &crate::object::GameObject,
    triggers: &mut Vec<PendingTrigger>,
) {
    let list = obj.ability_list(lookup);
    for (index, ability) in list.abilities.iter().enumerate() {
        let Some(firing) = triggered_parts(ability) else {
            continue;
        };
        if !firing.zone.functions_on_battlefield() && !firing.zone.functions_in_command() {
            continue;
        }
        let trigger = firing.trigger;
        let context = ability_context(state, obj, &list, index);
        // CR 603.4's first check, as in the battlefield loop below.
        for (at, entry) in events.iter().enumerate() {
            if first_turn.is_some_and(|start| at < start)
                && matches!(entry.event, GameEvent::CardsDrawn { .. })
            {
                continue;
            }
            if !condition_matches(
                state,
                firing.condition,
                obj.controller,
                context,
                Some(obj.version),
                Some(entry),
            ) {
                continue;
            }
            let hit = hits_with_context(
                trigger,
                &entry.event,
                events,
                state,
                context,
                obj.controller,
            );
            if hit > 0 {
                let times = super::matching::trigger_count_for_entry(
                    state,
                    trigger,
                    emblem,
                    obj.controller,
                    Some(entry),
                ) * hit;
                let event_object = event_object_for(trigger, &entry.event, emblem, state);
                let event_damage = event_amount_for(
                    trigger,
                    &entry.event,
                    events,
                    state,
                    obj.controller,
                    context,
                );
                let found_objects = super::event_found_for(
                    trigger,
                    &entry.event,
                    events,
                    state,
                    obj.controller,
                    context,
                );
                for _ in 0..times {
                    triggers.push(PendingTrigger {
                        text: TextChangeMap::IDENTITY,
                        source_version: Some(obj.version),
                        related_objects: None,
                        found_objects: found_objects.clone(),
                        grantor: None,
                        event_subject: None,
                        event_ability: activated_stack_ability(&entry.event),
                        event_object_identity: event_object
                            .and_then(|id| event_identity_for(trigger, entry, id, state)),
                        counter_source_version: None,
                        event_mana: produced_mana(&entry.event, events),
                        event_mana_value: None,
                        event_tap_snapshot: super::events::tap_event_snapshot(state, events, entry),
                        event_departure: entry
                            .departure
                            .as_ref()
                            .map(|d| (d.controller, d.toughness)),
                        event_amount: event_damage,
                        event_player: event_player_for(trigger, entry, state),
                        source: emblem,
                        ability_index: index as u32,
                        abilities: Some(list.clone()),
                        controller: obj.controller,
                        timestamp: obj.timestamp,
                        event_object,
                        implicit_target: None,
                        synthetic_effects: None,
                        once_per_turn: firing.once_per_turn,
                        synthetic_target: None,
                        chosen_mode: None,
                        synthetic_condition: None,
                        in_chosen_order: false,
                    });
                }
                // Once per matching event, as below.
            }
        }
    }
}
