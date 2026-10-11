//! Hand-only discard triggers retain the discarded incarnation and causing player.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn collect(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for entry in events {
        let GameEvent::Discarded {
            object,
            version,
            player,
            by: Some(by),
            known: true,
        } = entry.event
        else {
            continue;
        };
        if !state.is_opponent(by, player) || state.has_left(player) {
            continue;
        }
        let Some(card) = state.source_object(baylee_core::ids::DamageSourceRef { object, version })
        else {
            continue;
        };
        let list = card.ability_list(lookup);
        for (index, ability) in list.abilities.iter().enumerate() {
            let Some(firing) = triggered_parts(ability) else {
                continue;
            };
            if *firing.trigger != Trigger::DiscardedThisByOpponent {
                continue;
            }
            let context = ability_context(state, card, &list, index);
            if !condition_matches(state, firing.condition, player, context) {
                continue;
            }
            triggers.push(PendingTrigger {
                text: TextChangeMap::IDENTITY,
                source: object,
                source_version: Some(version),
                ability_index: index as u32,
                abilities: Some(list.clone()),
                controller: player,
                timestamp: card.timestamp,
                event_object: Some(object),
                event_object_identity: Some((version, card.characteristics().power.unwrap_or(0))),
                counter_source_version: None,
                event_mana: None,
                event_mana_value: None,
                event_departure: None,
                event_amount: None,
                event_player: Some(by),
                implicit_target: None,
                synthetic_effects: None,
                once_per_turn: firing.once_per_turn,
                synthetic_target: None,
                chosen_mode: None,
            });
        }
    }
}
