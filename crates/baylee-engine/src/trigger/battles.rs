//! Intrinsic Siege defeat captures the occurrence before later instructions change its source.
use super::{PendingTrigger, TextChangeMap};
use crate::event::{GameEvent, JournalEntry};
use crate::object::Status;
use crate::state::{CardLookup, GameState};
use crate::zone::Zone;
use baylee_cards_dsl::{CounterKind, Effect, TriggerEventKind};
use baylee_core::ids::{AbilityRef, DamageSourceRef, ObjectId};

/// Event-time facts for a removal, before counter-dependent projections change.
pub(crate) fn removal_event(
    state: &GameState,
    id: ObjectId,
    kind: CounterKind,
    amount: u16,
) -> Option<GameEvent> {
    if kind != CounterKind::Defense {
        return None;
    }
    let source = state.object(id)?;
    let old = source.counters.get(kind);
    let characteristics = source.characteristics();
    if old == 0
        || amount < old
        || source.zone != Zone::Battlefield
        || source.status.contains(Status::PHASED_OUT)
        || !characteristics
            .types
            .contains(baylee_core::types::TypeSet::BATTLE)
        || !characteristics
            .subtypes
            .contains(baylee_core::generated::subtypes::battle::SIEGE)
        || characteristics.abilities_lost.is_some()
    {
        return None;
    }
    Some(GameEvent::BattleDefeated {
        source: DamageSourceRef {
            object: id,
            version: source.version,
        },
        controller: source.controller,
        timestamp: source.timestamp,
        times: super::times_triggered_with_source(state, TriggerEventKind::Any, source),
    })
}

pub(super) fn collect(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    static CAST: &[Effect] = &[Effect::ExileSourceThenCastTransformed { cast: true }];
    static EXILE: &[Effect] = &[Effect::ExileSourceThenCastTransformed { cast: false }];
    for entry in events {
        let GameEvent::BattleDefeated {
            source,
            controller,
            timestamp,
            times,
        } = entry.event
        else {
            continue;
        };
        // Copying changes the Siege's characteristics, never its physical
        // card. A single-faced card copying a Siege is exiled but cannot be
        // cast transformed; neither can a token. This fact is immutable for
        // the exact source incarnation, so collection can read its LKI.
        let cast = state
            .source_object(source)
            .filter(|o| o.is_card())
            .and_then(|o| o.card)
            .and_then(|card| lookup.card(card.index))
            .is_some_and(|def| {
                def.faces.len() == 2
                    && !def.faces[1].castable_from_hand
                    && !def.faces[1]
                        .types
                        .contains(baylee_core::types::TypeSet::LAND)
            });
        for _ in 0..times {
            triggers.push(PendingTrigger {
                source: source.object,
                source_version: Some(source.version),
                counter_source_version: None,
                related_objects: None,
                event_subject: None,
                event_ability: None,
                event_object_identity: None,
                ability_index: AbilityRef::SYNTHETIC,
                abilities: None,
                text: TextChangeMap::IDENTITY,
                controller,
                timestamp,
                event_object: None,
                event_mana_value: None,
                event_departure: None,
                event_mana: None,
                event_amount: None,
                event_player: None,
                implicit_target: None,
                synthetic_effects: Some(if cast { CAST } else { EXILE }),
                synthetic_target: None,
                once_per_turn: false,
                chosen_mode: None,
            });
        }
    }
}
