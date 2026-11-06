//! Hand-only discard triggers retain the discarded incarnation and causing player.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn collect(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    collect_madness(state, events, triggers);
    collect_opponent_discards(state, lookup, events, triggers);
}

fn collect_madness(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    static MADNESS: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::Madness];
    for entry in events {
        let GameEvent::MadnessExiled {
            object,
            version,
            owner,
        } = entry.event
        else {
            continue;
        };
        if state.has_left(owner) {
            continue;
        }
        // Triggering uses the discarded incarnation, even if a replacement
        // moves the exile card again before the trigger reaches the stack.
        let Some(card) = state.source_object(baylee_core::ids::DamageSourceRef { object, version })
        else {
            continue;
        };
        triggers.push(PendingTrigger {
            text: TextChangeMap::IDENTITY,
            source: object,
            source_version: Some(version),
            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
            abilities: None,
            controller: owner,
            timestamp: card.timestamp,
            event_object: Some(object),
            related_objects: None,
            event_subject: None,
            event_ability: None,
            event_object_identity: Some((version, card.characteristics().power.unwrap_or(0))),
            counter_source_version: None,
            event_mana: None,
            event_mana_value: None,
            event_departure: None,
            event_amount: None,
            event_player: Some(owner),
            implicit_target: None,
            synthetic_effects: Some(MADNESS),
            once_per_turn: false,
            synthetic_target: None,
            chosen_mode: None,
            in_chosen_order: false,
        });
    }
}

fn collect_opponent_discards(
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
            if !condition_matches(
                state,
                firing.condition,
                player,
                context,
                Some(version),
                Some(entry),
            ) {
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
                related_objects: None,
                event_subject: None,
                event_ability: None,
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
                in_chosen_order: false,
            });
        }
    }
}
