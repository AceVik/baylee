//! Synthetic battlefield-entry triggers for keyword abilities:
//! - Exploit (`KeywordSet::EXPLOIT`, CR 702.110a–b)
//! - Evolve (`KeywordSet::EVOLVE`, CR 702.100a–d, CR 603.4)

use super::events::{ObjectMove, object_moves};
use super::matching::times_triggered_with_source;
use super::{PendingTrigger, source_at_event_with_moves};
use crate::{
    event::{GameEvent, JournalEntry},
    object::GameObject,
    state::GameState,
    text_changes::TextChangeMap,
    zone::Zone,
};
use baylee_cards_dsl::{Amount, Condition, CounterKind, Effect, KeywordSet, TriggerEventKind};
use baylee_core::{
    ids::{AbilityRef, DamageSourceRef},
    types::TypeSet,
};

static EXPLOIT_EFFECTS: &[Effect] = &[Effect::Exploit];

static EVOLVE_EFFECTS: &[Effect] = &[Effect::AddCounterSource {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(1),
}];

static EVOLVE_CONDITION: Condition = Condition::EvolveFromEvent;

pub(super) fn collect(
    state: &GameState,
    obj: &GameObject,
    events: &[JournalEntry],
    moves: &[ObjectMove],
    batch_has_zone_changes: bool,
    triggers: &mut Vec<PendingTrigger>,
) {
    if !batch_has_zone_changes {
        return;
    }
    let permanent = obj.id;
    for entry in events {
        let GameEvent::ZoneChanged {
            object,
            to: Zone::Battlefield,
            ..
        } = &entry.event
        else {
            continue;
        };
        let source = source_at_event_with_moves(state, obj, moves, entry.seq, false);
        let kws = source.characteristics().keywords;
        if *object == permanent {
            if kws.contains(KeywordSet::EXPLOIT) {
                let mult =
                    times_triggered_with_source(state, TriggerEventKind::EntersBattlefield, source);
                let n = crate::layers::keyword_instances(state, source, KeywordSet::EXPLOIT)
                    .saturating_mul(mult);
                push_entry_trigger(
                    source,
                    *object,
                    state.event_object_identity(*object, entry.seq),
                    n,
                    EXPLOIT_EFFECTS,
                    None,
                    triggers,
                );
            }
            continue;
        }
        if !kws.contains(KeywordSet::EVOLVE)
            || !source.characteristics().types.contains(TypeSet::CREATURE)
        {
            continue;
        }
        let Some(ent_curr) = state.object_or_departed(*object) else {
            continue;
        };
        let ent_moves = object_moves(events, batch_has_zone_changes, *object);
        let ent = source_at_event_with_moves(state, ent_curr, &ent_moves, entry.seq, false);
        if ent.controller != source.controller {
            continue;
        }
        let ent_ref = DamageSourceRef {
            object: *object,
            version: ent.version,
        };
        if !crate::eval::evolve_holds(state, permanent, Some(source.version), Some(ent_ref)) {
            continue;
        }
        let mult = times_triggered_with_source(state, TriggerEventKind::EntersBattlefield, source);
        let n = crate::layers::keyword_instances(state, source, KeywordSet::EVOLVE)
            .saturating_mul(mult);
        let identity = Some((ent.version, ent.characteristics().power.unwrap_or(0)));
        push_entry_trigger(
            source,
            *object,
            identity,
            n,
            EVOLVE_EFFECTS,
            Some(&EVOLVE_CONDITION),
            triggers,
        );
    }
}

fn push_entry_trigger(
    source: &GameObject,
    event_object: baylee_core::ids::ObjectId,
    event_object_identity: Option<(u32, i16)>,
    count: u32,
    effects: &'static [Effect],
    condition: Option<&'static Condition>,
    triggers: &mut Vec<PendingTrigger>,
) {
    for _ in 0..count {
        triggers.push(PendingTrigger {
            text: TextChangeMap::IDENTITY,
            source_version: Some(source.version),
            related_objects: None,
            grantor: None,
            event_subject: None,
            event_ability: None,
            event_object_identity,
            counter_source_version: None,
            event_mana: None,
            event_mana_value: None,
            event_departure: None,
            event_amount: None,
            event_player: None,
            source: source.id,
            ability_index: AbilityRef::SYNTHETIC,
            controller: source.controller,
            timestamp: source.timestamp,
            event_object: Some(event_object),
            implicit_target: None,
            abilities: None,
            synthetic_effects: Some(effects),
            once_per_turn: false,
            synthetic_target: None,
            synthetic_condition: condition,
            chosen_mode: None,
            in_chosen_order: false,
        });
    }
}

static CHAMPION_LTB_EFFECTS: &[Effect] = &[Effect::ReturnChampioned];

/// Champion (`AbilityDef::Champion`, `CR 702.72a–b`):
/// - ETB (`all_kinds == true`): triggers `AbilityDef::Champion { filter }` when
///   this permanent enters the battlefield.
/// - LTB (`all_kinds == false`): triggers `Effect::ReturnChampioned` when this
///   permanent leaves the battlefield (`CR 603.10a`).
#[allow(clippy::too_many_arguments)]
pub(super) fn collect_champion(
    state: &GameState,
    obj: &GameObject,
    departed_obj: Option<&GameObject>,
    list: &crate::object::AbilityList,
    events: &[JournalEntry],
    moves: &[ObjectMove],
    all_kinds: bool,
    phased_out_only: bool,
    triggers: &mut Vec<PendingTrigger>,
) {
    if phased_out_only
        || !list
            .abilities
            .iter()
            .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::Champion { .. }))
    {
        return;
    }
    let permanent = obj.id;
    for (index, ability) in list.abilities.iter().enumerate() {
        if !matches!(ability, baylee_cards_dsl::AbilityDef::Champion { .. }) {
            continue;
        }
        for entry in events {
            if all_kinds {
                let GameEvent::ZoneChanged {
                    object,
                    to: Zone::Battlefield,
                    ..
                } = &entry.event
                else {
                    continue;
                };
                if *object != permanent {
                    continue;
                }
                let source = source_at_event_with_moves(state, obj, moves, entry.seq, false);
                if source.characteristics().rules_text_lost
                    || source.characteristics().abilities_lost.is_some()
                {
                    continue;
                }
                let mult =
                    times_triggered_with_source(state, TriggerEventKind::EntersBattlefield, source);
                push_champion_etb(state, source, index, list, mult, triggers);
            } else {
                let GameEvent::ZoneChanged {
                    object,
                    from: Zone::Battlefield,
                    ..
                } = &entry.event
                else {
                    continue;
                };
                if *object != permanent {
                    continue;
                }
                let source = departed_obj.unwrap_or_else(|| {
                    source_at_event_with_moves(state, obj, moves, entry.seq, true)
                });
                if source.characteristics().rules_text_lost
                    || source.characteristics().abilities_lost.is_some()
                {
                    continue;
                }
                let mult = times_triggered_with_source(state, TriggerEventKind::Any, source);
                push_entry_trigger(
                    source,
                    permanent,
                    None,
                    mult,
                    CHAMPION_LTB_EFFECTS,
                    None,
                    triggers,
                );
            }
        }
    }
}

fn push_champion_etb(
    state: &GameState,
    source: &GameObject,
    index: usize,
    list: &crate::object::AbilityList,
    count: u32,
    triggers: &mut Vec<PendingTrigger>,
) {
    for _ in 0..count {
        triggers.push(PendingTrigger {
            text: super::object_text(state, source),
            source_version: Some(source.version),
            related_objects: None,
            grantor: None,
            event_subject: None,
            event_ability: None,
            event_object_identity: None,
            counter_source_version: None,
            event_mana: None,
            event_mana_value: None,
            event_departure: None,
            event_amount: None,
            event_player: None,
            source: source.id,
            ability_index: u32::try_from(index).expect("ability index fits u32"),
            controller: source.controller,
            timestamp: source.timestamp,
            event_object: Some(source.id),
            implicit_target: None,
            abilities: Some(list.clone()),
            synthetic_effects: None,
            once_per_turn: false,
            synthetic_target: None,
            synthetic_condition: None,
            chosen_mode: None,
            in_chosen_order: false,
        });
    }
}
