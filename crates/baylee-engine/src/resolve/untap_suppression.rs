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
    for id in zones::spec_objects(state, res, target) {
        let reference = if target == TargetSpec::EventObject {
            subjects::event(state, res)
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
