//! Counter events are replaced against the prospective permanent before entry.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::object::Rider;

pub(super) fn needs_entry(state: &GameState, id: ObjectId) -> bool {
    // External instructions attach counters after requesting the move. Keep that
    // move open whenever replacement ordering could need a decision. Intrinsic
    // counters alone keep their synchronous path when arithmetic is unambiguous.
    super::modifiers(state, id)
        .iter()
        .any(|m| matches!(m, EnterModifier::ChooseCounterPerEntryCount(_)))
        || state.effects.iter().any(|effect| {
            matches!(
                effect.modifier,
                baylee_cards_dsl::Modifier::AdditionalCounterPlacement { n: 1.. }
                    | baylee_cards_dsl::Modifier::AdditionalCounterOnEntryByCount { .. }
                    | baylee_cards_dsl::Modifier::DoubleCountersPlacedByYou
            )
        })
}

pub(super) fn question(state: &mut GameState, entry: &mut Attempt) -> Option<Pending> {
    if entry.refusal.is_some() {
        return None;
    }
    if !entry.counters_prepared {
        prepare(state, entry);
        entry.counters_prepared = true;
    }
    while let Some(plan) = entry.counter_plans.get_mut(entry.next_counter) {
        if let Some(additional) = plan.question() {
            return Some(Pending::YesNo {
                player: entry.controller,
                prompt: YesNoPrompt::ApplyAdditionalCountersNext {
                    permanent: entry.object,
                    additional,
                },
                source: None,
            });
        }
        entry.next_counter += 1;
    }
    None
}

fn prepare(state: &mut GameState, entry: &mut Attempt) {
    let Some(mut prospective) = state.object(entry.object).cloned() else {
        return;
    };
    let from_stack = prospective.zone == Zone::Stack;
    let x = if from_stack || entry.copy == CopyStatus::Handled {
        prospective.x_value
    } else {
        0
    };
    for modifier in entry.modifiers {
        if let EnterModifier::WithCounters { kind, amount } = modifier {
            let n = crate::eval::amount(amount, state, entry.controller, entry.object, Some(x));
            if n > 0 {
                entry
                    .counters
                    .push((None, *kind, u16::try_from(n).unwrap_or(u16::MAX)));
            }
        }
    }
    if prospective
        .abilities(state.bases.as_ref())
        .iter()
        .any(|ability| matches!(ability, AbilityDef::SagaChapter { .. }))
    {
        entry.counters.push((None, CounterKind::Lore, 1));
    }
    if let Some(values) = crate::layers::copiable_values(state, entry.object)
        && values
            .types
            .contains(baylee_core::types::TypeSet::PLANESWALKER)
        && let Some(loyalty) = values.loyalty
    {
        entry.counters.push((None, CounterKind::Loyalty, loyalty));
    }
    if from_stack {
        for rider in &prospective.riders {
            if let Rider::EntryP1P1(amount) = rider {
                let Ok(amount) = u16::try_from(*amount) else {
                    state.numeric_failure = Some("mana entry counter amount exceeds u16");
                    continue;
                };
                entry.counters.push((None, CounterKind::P1P1, amount));
            }
        }
    }
    prospective.zone = Zone::Battlefield;
    prospective.version = prospective.version.wrapping_add(1);
    let layers = crate::layers::LayerPlan::before_entries(&state.effects, unavailable(state));
    let projection = crate::layers::recompute_with(state, &prospective, &layers);
    prospective
        .cache
        .store(projection.characteristics, &prospective.base);
    prospective.controller = projection.controller;
    entry.controller = projection.controller;
    if let Some(defense) = crate::battle::intrinsic_defense(&prospective) {
        entry.counters.push((None, CounterKind::Defense, defense));
    }
    entry.counter_caps = prospective_caps(state, &prospective);
    let mut groups: Vec<(PlayerId, Vec<(CounterKind, u16)>)> = Vec::new();
    for (by, kind, count) in std::mem::take(&mut entry.counters) {
        // A prohibition removes the placement before replacement ordering.
        if entry.counter_caps.contains(&(kind, 0)) {
            continue;
        }
        let by = by.unwrap_or(entry.controller);
        if let Some((_, counters)) = groups.iter_mut().find(|(player, _)| *player == by) {
            if let Some((_, total)) = counters.iter_mut().find(|(existing, _)| *existing == kind) {
                *total = total.saturating_add(count);
            } else {
                counters.push((kind, count));
            }
        } else {
            groups.push((by, vec![(kind, count)]));
        }
    }
    // An external entry replacement can introduce the event's first counters.
    // Give the entering permanent's controller an empty group to build that
    // plan even when neither the card nor another instruction placed any.
    if !groups.iter().any(|(by, _)| *by == entry.controller) {
        groups.push((entry.controller, Vec::new()));
    }
    for (by, counters) in groups {
        if let Some(plan) = crate::replacement::counter_order::build_plan(
            state,
            by,
            &prospective,
            counters,
            true,
            None,
            true,
        ) {
            entry.counter_plans.push(plan);
        }
    }
}

fn prospective_caps(
    state: &GameState,
    object: &crate::object::GameObject,
) -> Vec<(CounterKind, u16)> {
    let mut caps = own_caps(state, object);
    caps.extend(state.effects.iter().filter_map(|effect| {
        let baylee_cards_dsl::Modifier::CounterMaximum { kind, maximum } = effect.modifier else {
            return None;
        };
        crate::effects::applies_to(state, effect, object).then_some((kind, maximum))
    }));
    caps
}

// An entering permanent's own prohibitions apply to its prospective body;
// they must constrain the placement before its static effects are registered.
fn own_caps(state: &GameState, object: &crate::object::GameObject) -> Vec<(CounterKind, u16)> {
    object
        .abilities(state.bases.as_ref())
        .iter()
        .filter_map(|ability| {
            let AbilityDef::Static(ability) = ability else {
                return None;
            };
            let baylee_cards_dsl::Modifier::CounterMaximum { kind, maximum } = ability.modifier
            else {
                return None;
            };
            (ability.source_zone == baylee_cards_dsl::ZoneRef::Battlefield
                && matches!(ability.filter, Filter::This)
                && ability.condition.is_none_or(|condition| {
                    crate::eval::condition_holds(state, object.controller, object.id, condition)
                }))
            .then_some((kind, maximum))
        })
        .collect()
}

pub(super) fn commit(state: &mut GameState, entry: Attempt) {
    for mut plan in entry.counter_plans {
        for (kind, amount) in &mut plan.counters {
            if let Some(maximum) = entry
                .counter_caps
                .iter()
                .filter_map(|(capped, maximum)| (*capped == *kind).then_some(*maximum))
                .min()
            {
                let already = state
                    .object(entry.object)
                    .map_or(0, |object| object.counters.get(*kind));
                *amount = (*amount).min(maximum.saturating_sub(already));
            }
        }
        crate::replacement::counter_order::commit_plan(state, plan);
    }
}
