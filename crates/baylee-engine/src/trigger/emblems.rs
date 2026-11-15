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
            let list = obj.ability_list(lookup);
            for (index, ability) in list.abilities.iter().enumerate() {
                let Some(firing) = triggered_parts(ability) else {
                    continue;
                };
                if firing.zone != TriggerZone::Battlefield {
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
                        let times = trigger_count(state, trigger, emblem, obj.controller) * hit;
                        let event_object = event_object_for(trigger, &entry.event, emblem, state);
                        let event_damage = event_damage_of(trigger, &entry.event, events);
                        for _ in 0..times {
                            triggers.push(PendingTrigger {
                                text: TextChangeMap::IDENTITY,
                                source_version: Some(obj.version),
                                related_objects: None,
                                grantor: None,
                                event_subject: None,
                                event_ability: activated_stack_ability(&entry.event),
                                event_object_identity: event_object
                                    .and_then(|id| state.event_object_identity(id, entry.seq)),
                                counter_source_version: None,
                                event_mana: produced_mana(&entry.event, events),
                                event_mana_value: None,
                                event_departure: entry
                                    .departure
                                    .as_ref()
                                    .map(|d| (d.controller, d.toughness)),
                                event_amount: event_damage,
                                event_player: event_player_of(&entry.event),
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
                                in_chosen_order: false,
                            });
                        }
                        // Once per matching event, as below.
                    }
                }
            }
        }
    }
}
