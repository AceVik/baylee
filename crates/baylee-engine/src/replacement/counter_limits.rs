//! Counter caps: forbidden placement and excess-counter state-based actions.
use crate::{effects::EffectFilter, state::GameState};
use baylee_cards_dsl::{CounterKind, Filter, Modifier};
use baylee_core::ids::ObjectId;

pub(super) fn room(state: &GameState, id: ObjectId, kind: CounterKind) -> Option<u16> {
    if !state.effects.may_limit_counters(kind) {
        return None;
    }
    let object = state.object(id)?;
    state
        .effects
        .iter()
        .filter_map(|effect| {
            let Modifier::CounterMaximum {
                kind: capped,
                maximum,
            } = effect.modifier
            else {
                return None;
            };
            (kind == capped && crate::effects::applies_to(state, effect, object))
                .then_some(maximum.saturating_sub(object.counters.get(kind)))
        })
        .min()
}

pub(crate) fn excess(state: &GameState) -> Vec<(ObjectId, CounterKind, u16)> {
    let mut actions = Vec::new();
    if !state.effects.has_counter_limits() {
        return actions;
    }
    for effect in state.effects.iter() {
        let Modifier::CounterMaximum { kind, maximum } = effect.modifier else {
            continue;
        };
        let mut add = |id| {
            if let Some(object) = state.object(id)
                && object.zone == crate::zone::Zone::Battlefield
                && !object.status.contains(crate::object::Status::PHASED_OUT)
                && crate::effects::applies_to(state, effect, object)
            {
                let excess = object.counters.get(kind).saturating_sub(maximum);
                if excess > 0 {
                    actions.push((id, kind, excess));
                }
            }
        };
        match effect.filter {
            EffectFilter::ObjectIs(id, _) => add(id),
            EffectFilter::Dsl(Filter::This) => {
                if let Some(id) = effect.source {
                    add(id);
                }
            }
            EffectFilter::Dsl(_)
            | EffectFilter::DslAbout(..)
            | EffectFilter::DslControlledBy(..) => {
                for id in state.battlefield_seen() {
                    add(id);
                }
            }
        }
    }
    // Multiple caps remove only enough to meet the strictest cap, not the sum.
    actions.sort_unstable_by_key(|(id, kind, _)| (*id, *kind));
    actions.dedup_by(|a, b| {
        if (a.0, a.1) != (b.0, b.1) {
            return false;
        }
        b.2 = b.2.max(a.2);
        true
    });
    actions
}
