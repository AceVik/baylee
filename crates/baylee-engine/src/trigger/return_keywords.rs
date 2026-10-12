//! Death-triggered return keywords, including their counter conditions.
use super::{PendingTrigger, source_at_event, times_triggered};
use crate::{event::GameEvent, object::GameObject, state::GameState, text_changes::TextChangeMap};

/// Undying (CR 702.93a) and persist (CR 702.79a) as the one sentence each
/// of them is: "return it to the battlefield under its owner's control with
/// a +1/+1 (or -1/-1) counter on it".
///
/// `TargetSpec::EventObject` and not a target: neither keyword prints the
/// word, so nothing may be chosen and nothing can be made illegal by a
/// hexproof granted in response — the ability returns the card that just
/// died or it returns nothing.
static UNDYING_RETURN: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::return_to_owner_with(
        baylee_cards_dsl::TargetSpec::EventObject,
        baylee_cards_dsl::CounterKind::P1P1,
        1,
    )];

static PERSIST_RETURN: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::return_to_owner_with(
        baylee_cards_dsl::TargetSpec::EventObject,
        baylee_cards_dsl::CounterKind::M1M1,
        1,
    )];

/// Count independent return abilities before departure, after copy and ability
/// layers. The common case without either keyword needs no effect scan.
pub(crate) fn return_keyword_counts(state: &GameState, object: &GameObject) -> [u32; 2] {
    use baylee_cards_dsl::{KeywordSet, Layer, Modifier};
    let kinds = [KeywordSet::UNDYING, KeywordSet::PERSIST];
    let active = object.characteristics().keywords;
    if !kinds.iter().any(|kind| active.contains(*kind)) {
        return [0; 2];
    }
    let base =
        crate::layers::copiable_values(state, object.id).unwrap_or_else(|| object.base.clone());
    let mut counts = kinds.map(|kind| u32::from(base.keywords.contains(kind)));
    let plan = crate::layers::LayerPlan::build(&state.effects);
    for effect in plan.effects_for(&state.effects, Layer::Ability, object.id) {
        if !crate::effects::applies_to(state, effect, object) {
            continue;
        }
        for (index, kind) in kinds.iter().enumerate() {
            match effect.modifier {
                Modifier::AddKeyword(keywords) if keywords.contains(*kind) => {
                    counts[index] = counts[index].saturating_add(1);
                }
                Modifier::AddKeywordIfCountersAtLeast {
                    kind: counter,
                    at_least,
                    keywords,
                } if keywords.contains(*kind)
                    && object.counters.get(counter) >= u16::from(at_least) =>
                {
                    counts[index] = counts[index].saturating_add(1);
                }
                Modifier::RemoveKeyword(keywords) if keywords.contains(*kind) => counts[index] = 0,
                Modifier::LoseAllAbilities | Modifier::LoseKeywords => counts[index] = 0,
                _ => {}
            }
        }
    }
    std::array::from_fn(|index| {
        if active.contains(kinds[index]) {
            counts[index].max(1)
        } else {
            0
        }
    })
}

pub(super) fn collect(
    state: &GameState,
    obj: &GameObject,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for entry in events {
        if !matches!(entry.event, GameEvent::ZoneChanged {
            object, from: crate::zone::Zone::Battlefield, to: crate::zone::Zone::Graveyard, ..
        } if object == obj.id)
        {
            continue;
        }
        let source = source_at_event(state, obj, events, entry, true);
        let counts = entry.departure.as_ref().map_or_else(
            || {
                [
                    baylee_cards_dsl::KeywordSet::UNDYING,
                    baylee_cards_dsl::KeywordSet::PERSIST,
                ]
                .map(|keyword| u32::from(source.characteristics().keywords.contains(keyword)))
            },
            |departure| departure.return_keywords,
        );
        for (index, (kind, effects)) in [
            (baylee_cards_dsl::CounterKind::P1P1, UNDYING_RETURN),
            (baylee_cards_dsl::CounterKind::M1M1, PERSIST_RETURN),
        ]
        .into_iter()
        .enumerate()
        {
            if counts[index] == 0 || source.counters.get(kind) > 0 {
                continue;
            }
            // CR 603.10a reads each death's exact pre-departure source, not
            // a later incarnation or whichever copy currently owns the handle.
            let times = counts[index].saturating_mul(times_triggered(
                state,
                baylee_cards_dsl::TriggerEventKind::Any,
                obj.id,
            ));
            for _ in 0..times {
                triggers.push(PendingTrigger {
                    text: TextChangeMap::IDENTITY,
                    source_version: Some(source.version),
                    related_objects: None,
                    event_object_identity: Some((
                        source.version.wrapping_add(1),
                        source.characteristics().power.unwrap_or(0),
                    )),
                    counter_source_version: None,
                    event_mana: None,
                    event_mana_value: None,
                    event_departure: None,
                    event_amount: None,
                    event_player: None,
                    source: obj.id,
                    ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                    controller: source.controller,
                    timestamp: source.timestamp,
                    event_object: Some(obj.id),
                    implicit_target: Some(obj.id),
                    abilities: None,
                    synthetic_effects: Some(effects),
                    once_per_turn: false,
                    synthetic_target: None,
                    chosen_mode: None,
                });
            }
        }
    }
}
