//! Soulbond's two synthetic ETB triggered abilities (CR 702.95a–b).

use super::events::ObjectMove;
use super::{PendingTrigger, source_at_event_with_moves, was_phased_out_at};
use crate::{
    event::{GameEvent, JournalEntry},
    object::{GameObject, Status},
    state::GameState,
    text_changes::TextChangeMap,
    zone::Zone,
};
use baylee_cards_dsl::{Condition, Effect, KeywordSet, TriggerEventKind};
use baylee_core::ids::AbilityRef;

static SOULBOND_SELF_CONDITION: Condition = Condition::SoulbondSelfCanPair;
static SOULBOND_EVENT_CONDITION: Condition = Condition::SoulbondEventCanPair;

static SOULBOND_SELF_ETB: &[Effect] = &[Effect::SoulbondPair {
    with_event_object: false,
}];

static SOULBOND_OTHER_ETB: &[Effect] = &[Effect::SoulbondPair {
    with_event_object: true,
}];

fn push_soulbond_triggers(
    state: &GameState,
    source: &GameObject,
    entry: &JournalEntry,
    event_object_identity: Option<(u32, i16)>,
    effects: &'static [Effect],
    condition: &'static Condition,
    triggers: &mut Vec<PendingTrigger>,
) {
    let GameEvent::ZoneChanged {
        object: entering, ..
    } = entry.event
    else {
        return;
    };
    let count = crate::layers::keyword_instances(state, source, KeywordSet::SOULBOND)
        .saturating_mul(super::matching::times_triggered_for_entry(
            state,
            TriggerEventKind::EntersBattlefield,
            source,
            Some(entry),
        ));
    for _ in 0..count {
        triggers.push(PendingTrigger {
            text: TextChangeMap::IDENTITY,
            source_version: Some(source.version),
            related_objects: None,
            found_objects: None,
            grantor: None,
            event_subject: None,
            event_ability: None,
            event_object_identity,
            counter_source_version: None,
            event_mana: None,
            event_mana_value: None,
            event_tap_snapshot: None,
            event_departure: None,
            event_amount: None,
            event_player: None,
            source: source.id,
            ability_index: AbilityRef::SYNTHETIC,
            controller: source.controller,
            timestamp: source.timestamp,
            event_object: Some(entering),
            implicit_target: None,
            abilities: None,
            synthetic_effects: Some(effects),
            once_per_turn: false,
            synthetic_target: None,
            chosen_mode: None,
            synthetic_condition: Some(condition),
            in_chosen_order: false,
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn collect(
    state: &GameState,
    obj: &GameObject,
    events: &[JournalEntry],
    moves: &[ObjectMove],
    batch_has_zone_changes: bool,
    batch_has_phasing: bool,
    currently_phased_out: bool,
    triggers: &mut Vec<PendingTrigger>,
) {
    if !batch_has_zone_changes {
        return;
    }
    let permanent = obj.id;
    for entry in events {
        let GameEvent::ZoneChanged {
            object: entering,
            to: Zone::Battlefield,
            ..
        } = entry.event
        else {
            continue;
        };
        if batch_has_phasing
            && was_phased_out_at(events, permanent, currently_phased_out, entry.seq, false)
        {
            continue;
        }
        let source = source_at_event_with_moves(state, obj, moves, entry.seq, false);
        if source.zone != Zone::Battlefield
            || source.status.contains(Status::PHASED_OUT)
            || !source
                .characteristics()
                .keywords
                .contains(KeywordSet::SOULBOND)
        {
            continue;
        }
        let you = source.controller;
        if entering == permanent {
            // CR 702.95a (first ability): "When this creature enters, if you
            // control both this creature and another creature and both are
            // unpaired, you may pair this creature with another unpaired
            // creature you control..."
            if !state.is_unpaired_creature_controlled_by(permanent, Some(source.version), you)
                || !state.battlefield_seen().any(|id| {
                    id != permanent && state.is_unpaired_creature_controlled_by(id, None, you)
                })
            {
                continue;
            }
            push_soulbond_triggers(
                state,
                source,
                entry,
                None,
                SOULBOND_SELF_ETB,
                &SOULBOND_SELF_CONDITION,
                triggers,
            );
        } else {
            // CR 702.95a (second ability): "Whenever another creature you
            // control enters, if you control both that creature and this one
            // and both are unpaired, you may pair that creature with this
            // creature..."
            let Some((entering_version, entering_power)) =
                state.event_object_identity(entering, entry.seq)
            else {
                continue;
            };
            if !state.can_soulbond_pair(
                you,
                permanent,
                Some(source.version),
                entering,
                Some(entering_version),
            ) {
                continue;
            }
            push_soulbond_triggers(
                state,
                source,
                entry,
                Some((entering_version, entering_power)),
                SOULBOND_OTHER_ETB,
                &SOULBOND_EVENT_CONDITION,
                triggers,
            );
        }
    }
}
