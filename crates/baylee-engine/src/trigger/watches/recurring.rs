//! Global delayed triggers retain their creator's context and observe future events.
use crate::event::JournalEntry;
use crate::state::{DelayedTrigger, DelayedWhen, GameState};
use crate::trigger::{PendingTrigger, events, matching};

pub(super) fn collect(
    state: &GameState,
    events: &[JournalEntry],
    watch: &DelayedTrigger,
    base: &PendingTrigger,
    out: &mut Vec<PendingTrigger>,
) {
    let DelayedWhen::TriggeredThisTurn {
        event,
        filter,
        targets,
        context,
        after,
        ..
    } = watch.when
    else {
        return;
    };
    let trigger = match event {
        baylee_cards_dsl::CombatWatchEvent::Blocks => baylee_cards_dsl::Trigger::Blocks(filter),
        baylee_cards_dsl::CombatWatchEvent::AttacksAndIsntBlocked => {
            baylee_cards_dsl::Trigger::AttacksAndIsntBlocked(filter)
        }
    };
    let events = &events[events.partition_point(|entry| entry.seq <= after)..];
    for entry in events {
        let count = matching::departure_hit(&trigger, entry, base.source).unwrap_or_else(|| {
            matching::hits_with_context(
                &trigger,
                &entry.event,
                events,
                state,
                context,
                watch.controller,
            )
        });
        if count == 0 {
            continue;
        }
        let event_object = events::event_object_for(&trigger, &entry.event, base.source, state);
        let event_object_identity =
            event_object.and_then(|id| state.event_object_identity(id, entry.seq));
        for _ in 0..count {
            out.push(PendingTrigger {
                event_object,
                event_object_identity,
                event_subject: events::event_subject_for(
                    &trigger,
                    &entry.event,
                    base.source,
                    state,
                ),
                event_amount: events::event_damage_of(&trigger, &entry.event, events),
                event_player: events::event_player_of(&entry.event),
                event_ability: crate::trigger::activated_stack_ability(&entry.event),
                event_mana: super::produced_mana(&entry.event, events),
                event_mana_value: crate::trigger::cast_mana_value(state, &entry.event),
                event_departure: entry
                    .departure
                    .as_ref()
                    .map(|d| (d.controller, d.toughness)),
                synthetic_target: targets,
                ..base.clone()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::event::{Cause, GameEvent};
    use crate::object::ObjectKind;
    use crate::state::DelayedAction;
    use crate::zone::{ZoneLocation, ZonePosition};
    use baylee_cards_dsl::{Amount, CombatWatchEvent, Effect, Filter};
    use baylee_core::ids::PlayerId;
    use baylee_core::types::TypeSet;

    const YOU: PlayerId = PlayerId::new(0);
    const THEM: PlayerId = PlayerId::new(1);
    static EFFECTS: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }];

    #[test]
    fn global_watch_follows_future_creatures_after_its_source_leaves_and_repeats() {
        let lookup = SyntheticLookup::new(vec![]);
        let mut state = GameState::from_preset(&preset(912, &[]), &lookup).unwrap();
        let name = state.names.intern("Combat observer fixture");
        let source = state.create_bare(YOU, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        let source_ref = state.source_identity(source).unwrap();
        let context = crate::eval::live_context(&state, source);
        state.delayed.push(DelayedTrigger {
            controller: YOU,
            when: DelayedWhen::TriggeredThisTurn {
                event: CombatWatchEvent::Blocks,
                filter: &Filter::ControlledByYou,
                targets: None,
                context,
                after: state.journal.last_seq(),
                this_combat_only: false,
            },
            action: DelayedAction::Trigger {
                source,
                source_version: source_ref.version,
                effects: EFFECTS,
                text: context.text,
            },
        });
        state
            .move_object(
                source,
                ZoneLocation::Graveyard(YOU),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let blocker =
            state.create_bare(YOU, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        state.object_mut(blocker).unwrap().base_mut().types = TypeSet::CREATURE;
        let other = state.create_bare(THEM, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        state.object_mut(other).unwrap().base_mut().types = TypeSet::CREATURE;
        for _ in 0..2 {
            let since = state.journal.last_seq();
            state.journal.record(GameEvent::BecameBlocker {
                object: blocker,
                attacker: other,
            });
            state.journal.record(GameEvent::BecameBlocker {
                object: blocker,
                attacker: source,
            });
            state.journal.record(GameEvent::BecameBlocker {
                object: other,
                attacker: blocker,
            });
            let triggers = crate::trigger::collect(&state, &lookup, since);
            assert_eq!(
                triggers.len(),
                1,
                "one trigger per blocking creature, not per pair"
            );
            assert_eq!(triggers[0].event_object, Some(blocker));
            assert_eq!(triggers[0].controller, YOU);
            assert_eq!(state.delayed.len(), 1, "recurring watch is not consumed");
        }
        assert!(state.delayed[0].when.lapses_with_the_turn());
    }
}
