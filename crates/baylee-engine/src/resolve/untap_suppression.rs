//! Finite untap suppression bound to the affected permanent's incarnation.

use super::{Resolution, subjects, zones};
use crate::{
    effects::{ContinuousEffect, EffectFilter, EffectOrigin},
    state::GameState,
};
use baylee_cards_dsl::{Duration, Modifier, TargetSpec};
use baylee_core::ids::EffectId;

pub(super) fn apply(state: &mut GameState, res: &Resolution, target: TargetSpec, steps: u8) {
    if steps == 0 {
        return;
    }
    let objects: smallvec::SmallVec<[baylee_core::ids::ObjectId; 4]> = if !res.targeted
        && res.targets.is_empty()
        && let TargetSpec::Object(filter) = target
    {
        let context = res.rule_context();
        state
            .battlefield_seen()
            .filter(|&id| {
                state.object(id).is_some_and(|o| {
                    crate::eval::matches_with_context(filter, state, o, res.controller, context)
                })
            })
            .collect()
    } else {
        zones::spec_objects(state, res, target)
            .into_iter()
            .collect()
    };
    for id in objects {
        let reference = if target == TargetSpec::EventObject {
            subjects::event(state, res)
        } else if target == TargetSpec::ThisObject {
            subjects::source(state, res)
        } else {
            subjects::target(state, res, id)
        };
        let Some(reference) = reference.filter(|r| subjects::on_battlefield(state, *r)) else {
            continue;
        };
        let timestamp = state.next_timestamp();
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(res.source),
            controller: res.controller,
            origin: EffectOrigin::Resolution,
            layer: Modifier::DoesNotUntap.layer(),
            timestamp,
            duration: Duration::UntilAffectedUntapSteps(steps),
            filter: EffectFilter::ObjectIs(reference.object, reference.version),
            modifier: Modifier::DoesNotUntap,
        });
    }
}
