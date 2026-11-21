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
            event_object.and_then(|id| events::event_identity_for(&trigger, entry, id, state));
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
                event_amount: events::event_amount_for(
                    &trigger,
                    &entry.event,
                    events,
                    state,
                    watch.controller,
                    context,
                ),
                event_player: events::event_player_for(&trigger, entry, state),
                event_ability: crate::trigger::activated_stack_ability(&entry.event),
                event_mana: super::produced_mana(&entry.event, events),
                event_mana_value: crate::trigger::cast_mana_value(state, entry),
                event_tap_snapshot: None,
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

/// Collect triggers from [`DelayedWhen::OnEvent`] (`Effect::RegisterDelayedTrigger`).
#[allow(clippy::too_many_lines)]
pub(crate) fn collect_general(
    state: &GameState,
    events: &[JournalEntry],
    delayed: &DelayedTrigger,
    out: &mut Vec<PendingTrigger>,
) {
    let DelayedWhen::OnEvent {
        trigger,
        watch,
        watch_battlefield,
        duration,
        context,
        after,
    } = delayed.when
    else {
        return;
    };
    let crate::state::DelayedAction::GeneralTrigger(ref spec) = delayed.action else {
        return;
    };
    let events = &events[events.partition_point(|entry| entry.seq <= after)..];
    if events.is_empty() {
        return;
    }
    if watch_battlefield
        && let Some(w) = watch
        && state
            .object(w.object)
            .is_none_or(|o| o.zone != crate::zone::Zone::Battlefield || o.version != w.version)
        && !events.iter().any(|e| {
            matches!(
                e.event,
                crate::event::GameEvent::ZoneChanged {
                    object,
                    from: crate::zone::Zone::Battlefield,
                    ..
                } if object == w.object
            ) && e.departure.as_ref().is_some_and(|d| d.version == w.version)
        })
    {
        return;
    }

    let one_shot = matches!(
        duration,
        baylee_cards_dsl::DelayedDuration::OneShot
            | baylee_cards_dsl::DelayedDuration::OneShotThisTurn
    );
    let match_source = watch.map_or(spec.source, |w| w.object);
    let timestamp = state.object(spec.source).map_or(0, |o| o.timestamp);

    for entry in events {
        let departed_this_entry = watch_battlefield
            && watch.is_some_and(|w| {
                matches!(
                    entry.event,
                    crate::event::GameEvent::ZoneChanged {
                        object,
                        from: crate::zone::Zone::Battlefield,
                        ..
                    } if object == w.object
                ) && entry
                    .departure
                    .as_ref()
                    .is_some_and(|d| d.version == w.version)
            });

        let mut count =
            matching::departure_hit(trigger, entry, match_source).unwrap_or_else(|| {
                matching::hits_with_context(
                    trigger,
                    &entry.event,
                    events,
                    state,
                    context,
                    delayed.controller,
                )
            });

        if count == 0 {
            if departed_this_entry {
                break;
            }
            continue;
        }
        if one_shot {
            count = 1;
        }

        let (
            event_object,
            event_object_identity,
            event_subject,
            event_amount,
            event_player,
            event_ability,
            event_mana,
            event_mana_value,
            event_departure,
        ) = if let Some(rem) = spec.remembered {
            let identity = rem.identity.map(|(ver, old_power)| {
                let live_power = rem.object.and_then(|obj| {
                    state
                        .source_object(baylee_core::ids::DamageSourceRef {
                            object: obj,
                            version: ver,
                        })
                        .and_then(|o| o.characteristics().power)
                });
                (ver, live_power.unwrap_or(old_power))
            });
            (
                rem.object,
                identity,
                rem.subject,
                rem.amount,
                rem.player,
                None,
                None,
                rem.mana_value,
                rem.departure,
            )
        } else {
            let ev_obj = events::event_object_for(trigger, &entry.event, match_source, state)
                .or_else(|| watch.map(|w| w.object));
            let ev_id = ev_obj
                .and_then(|id| {
                    entry.departure.as_deref().and_then(|dep| {
                        let ver = state
                            .object(id)
                            .map_or_else(|| dep.version.saturating_add(1), |o| o.version);
                        (matches!(entry.event, crate::event::GameEvent::ZoneChanged { object, .. } if object == id))
                            .then_some((ver, dep.power))
                    })
                })
                .or_else(|| {
                    ev_obj.and_then(|id| {
                        if entry.tap_observation.is_some() {
                            events::event_identity_for(trigger, entry, id, state)
                        } else {
                            state.event_object_identity(id, entry.seq)
                        }
                    })
                });
            (
                ev_obj,
                ev_id,
                events::event_subject_for(trigger, &entry.event, match_source, state),
                events::event_amount_for(
                    trigger,
                    &entry.event,
                    events,
                    state,
                    delayed.controller,
                    context,
                ),
                events::event_player_for(trigger, entry, state),
                crate::trigger::activated_stack_ability(&entry.event),
                super::produced_mana(&entry.event, events),
                crate::trigger::cast_mana_value(state, entry),
                entry
                    .departure
                    .as_ref()
                    .map(|d| (d.controller, d.toughness)),
            )
        };

        let event_ref = event_object
            .zip(event_object_identity.map(|(v, _)| v))
            .map(|(object, version)| baylee_core::ids::DamageSourceRef { object, version });
        let event_tap_snapshot = if spec.remembered.is_none() {
            events::tap_event_snapshot(state, events, entry)
        } else {
            None
        };
        if crate::eval::intervening_if_for_incarnation(
            state,
            spec.condition.copied(),
            delayed.controller,
            spec.source,
            Some(spec.source_version),
            None,
            event_ref,
        ) {
            for _ in 0..count {
                out.push(PendingTrigger {
                    text: spec.text,
                    source_version: Some(spec.source_version),
                    related_objects: None,
                    grantor: None,
                    event_subject,
                    event_ability,
                    event_object_identity,
                    counter_source_version: None,
                    event_amount,
                    event_player,
                    event_mana,
                    event_mana_value,
                    event_tap_snapshot,
                    event_departure,
                    source: spec.source,
                    ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                    abilities: None,
                    controller: delayed.controller,
                    timestamp,
                    event_object,
                    implicit_target: None,
                    synthetic_effects: Some(spec.effects),
                    synthetic_target: spec.targets,
                    synthetic_condition: spec.condition,
                    once_per_turn: false,
                    chosen_mode: None,
                    in_chosen_order: false,
                });
            }
        }

        if one_shot || departed_this_entry {
            break;
        }
    }
}

/// Whether a [`DelayedWhen::OnEvent`] watch has been consumed by `events` or by
/// its watched battlefield incarnation having departed (CR 603.7b, CR 400.7).
pub(crate) fn on_event_watch_consumed(
    state: &GameState,
    events: &[JournalEntry],
    delayed: &DelayedTrigger,
) -> bool {
    let DelayedWhen::OnEvent {
        trigger,
        watch,
        watch_battlefield,
        duration,
        context,
        after,
    } = delayed.when
    else {
        return false;
    };
    if watch_battlefield
        && let Some(w) = watch
        && state
            .object(w.object)
            .is_none_or(|o| o.zone != crate::zone::Zone::Battlefield || o.version != w.version)
    {
        return true;
    }
    if !matches!(
        duration,
        baylee_cards_dsl::DelayedDuration::OneShot
            | baylee_cards_dsl::DelayedDuration::OneShotThisTurn
    ) {
        return false;
    }
    let crate::state::DelayedAction::GeneralTrigger(ref spec) = delayed.action else {
        return false;
    };
    let match_source = watch.map_or(spec.source, |w| w.object);
    let events = &events[events.partition_point(|entry| entry.seq <= after)..];
    events.iter().any(|entry| {
        matching::departure_hit(trigger, entry, match_source).unwrap_or_else(|| {
            matching::hits_with_context(
                trigger,
                &entry.event,
                events,
                state,
                context,
                delayed.controller,
            )
        }) > 0
    })
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
