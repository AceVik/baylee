//! One-shot cast watches; consumed by the cast, even if their trigger is countered.
use super::{GameEvent, GameState, PendingTrigger, RuleContext};
use crate::event::JournalEntry;
use crate::state::{DelayedAction, DelayedTrigger, DelayedWhen};

fn matching_event<'a>(
    state: &GameState,
    watch: &DelayedTrigger,
    events: &'a [JournalEntry],
) -> Option<&'a JournalEntry> {
    let DelayedWhen::NextSpellCastThisTurn { filter, after } = watch.when else {
        return None;
    };
    let DelayedAction::Trigger { source, text, .. } = watch.action else {
        return None;
    };
    events.iter().find(|entry| {
        if entry.seq <= after {
            return false;
        }
        let GameEvent::SpellCast { object, player, .. } = entry.event else {
            return false;
        };
        player == watch.controller
            && state.object(object).is_some_and(|object| {
                crate::eval::matches_with_context(
                    filter,
                    state,
                    object,
                    player,
                    RuleContext {
                        source,
                        text,
                        resolved_name: None,
                        reference: None,
                    },
                )
            })
    })
}

pub(super) fn collect(
    state: &GameState,
    events: &[JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for watch in &state.delayed {
        let Some(entry) = matching_event(state, watch, events) else {
            continue;
        };
        let GameEvent::SpellCast { object, .. } = entry.event else {
            unreachable!()
        };
        let DelayedAction::Trigger {
            source,
            source_version,
            effects,
            text,
        } = watch.action
        else {
            unreachable!()
        };
        triggers.push(PendingTrigger {
            text,
            source_version: Some(source_version),
            event_ability: None,
            event_object_identity: state.event_object_identity(object, entry.seq),
            related_objects: None,
            event_subject: None,
            counter_source_version: None,
            event_amount: None,
            event_player: Some(watch.controller),
            event_mana: None,
            event_mana_value: None,
            event_departure: None,
            source,
            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
            abilities: None,
            controller: watch.controller,
            timestamp: state.object(source).map_or(0, |object| object.timestamp),
            event_object: Some(object),
            implicit_target: None,
            synthetic_effects: Some(effects),
            once_per_turn: false,
            synthetic_target: None,
            chosen_mode: None,
        });
    }
}

pub(crate) fn consume(state: &mut GameState, from_seq: u64) {
    if state.delayed.is_empty() {
        return;
    }
    let all_events = state.journal.entries();
    let events = &all_events[all_events.partition_point(|entry| entry.seq <= from_seq)..];
    if events.is_empty() {
        return;
    }
    let mut watches = std::mem::take(&mut state.delayed);
    watches.retain(|watch| matching_event(state, watch, events).is_none());
    state.delayed = watches;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::object::ObjectKind;
    use crate::text_changes::TextChangeMap;
    use crate::zone::ZoneLocation;
    use baylee_cards_dsl::{Effect, Filter};
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_core::types::TypeSet;

    fn spell(state: &mut GameState, player: PlayerId, types: TypeSet) -> ObjectId {
        let name = state.names.intern("cast watch test");
        let card = state.create_bare(player, ObjectKind::Spell, name, ZoneLocation::Stack);
        std::sync::Arc::make_mut(&mut state.object_mut(card).unwrap().base).types = types;
        state.invalidate_projections();
        state.journal.record(GameEvent::SpellCast {
            object: card,
            player,
            number: 1,
            instant_number: 0,
        });
        card
    }

    fn watch(state: &mut GameState) {
        state.delayed.push(DelayedTrigger {
            controller: PlayerId::new(0),
            when: DelayedWhen::NextSpellCastThisTurn {
                filter: &Filter::CREATURE,
                after: state.journal.last_seq(),
            },
            action: DelayedAction::Trigger {
                source: ObjectId::NO_SOURCE,
                source_version: 0,
                effects: &[Effect::EventSpellEntersWithP1P1 { n: 1 }],
                text: TextChangeMap::IDENTITY,
            },
        });
    }

    #[test]
    fn next_cast_watch_filters_controller_type_and_registration_time() {
        let mut state =
            GameState::from_preset(&preset(552, &[]), &SyntheticLookup::new(vec![])).unwrap();
        spell(&mut state, PlayerId::new(0), TypeSet::CREATURE);
        watch(&mut state);
        spell(&mut state, PlayerId::new(1), TypeSet::CREATURE);
        spell(&mut state, PlayerId::new(0), TypeSet::SORCERY);
        let mut triggers = vec![];
        collect(&state, state.journal.entries(), &mut triggers);
        assert!(triggers.is_empty());
        let first = spell(&mut state, PlayerId::new(0), TypeSet::CREATURE);
        spell(&mut state, PlayerId::new(0), TypeSet::CREATURE);
        collect(&state, state.journal.entries(), &mut triggers);
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].event_object, Some(first));
        assert_eq!(
            triggers[0]
                .event_object_identity
                .map(|(version, _)| version),
            Some(state.object(first).unwrap().version)
        );
        consume(&mut state, 0);
        assert!(
            state.delayed.is_empty(),
            "spent on cast, not successful trigger resolution"
        );
        triggers.clear();
        collect(&state, state.journal.entries(), &mut triggers);
        assert!(
            triggers.is_empty(),
            "countering the queued trigger cannot preserve the watch"
        );
    }

    #[test]
    fn multiple_watches_all_fire_for_same_next_spell_and_expire_at_turn_boundary() {
        let mut state =
            GameState::from_preset(&preset(553, &[]), &SyntheticLookup::new(vec![])).unwrap();
        watch(&mut state);
        watch(&mut state);
        assert!(
            state
                .delayed
                .iter()
                .all(|watch| watch.when.lapses_with_the_turn())
        );
        let first = spell(&mut state, PlayerId::new(0), TypeSet::CREATURE);
        let mut triggers = vec![];
        collect(&state, state.journal.entries(), &mut triggers);
        assert_eq!(triggers.len(), 2);
        assert!(
            triggers
                .iter()
                .all(|trigger| trigger.event_object == Some(first))
        );
        consume(&mut state, 0);
        assert!(state.delayed.is_empty());
    }
}
