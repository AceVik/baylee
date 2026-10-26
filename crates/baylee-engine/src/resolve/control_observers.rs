//! One-shot delayed observers keep their watched and affected objects distinct.

use super::{Resolution, subjects, zones};
use crate::state::{DelayedAction, DelayedTrigger, DelayedWhen, GameState};
use baylee_cards_dsl::{DelayedObservation, Effect, TargetSpec};
use baylee_core::ids::DamageSourceRef;

fn reference(state: &GameState, res: &Resolution, spec: TargetSpec) -> Option<DamageSourceRef> {
    match spec {
        TargetSpec::ThisObject => subjects::source(state, res),
        TargetSpec::EventObject => subjects::event(state, res),
        _ => subjects::target(state, res, zones::spec_object(state, res, spec)?),
    }
    .filter(|reference| subjects::on_battlefield(state, *reference))
}

pub(super) fn register(
    state: &mut GameState,
    res: &Resolution,
    watch: TargetSpec,
    when: DelayedObservation,
    about: TargetSpec,
    effects: &'static [Effect],
) {
    let watched = if watch == TargetSpec::ThisObject {
        super::source_version(state, res)
            .map(|version| DamageSourceRef {
                object: res.source,
                version,
            })
            .or_else(|| state.source_identity(res.source))
            .filter(|reference| subjects::on_battlefield(state, *reference))
    } else {
        reference(state, res, watch)
    };
    let Some(watch) = watched else {
        return;
    };
    let Some(about) = reference(state, res, about) else {
        return;
    };
    let source_version = super::source_version(state, res)
        .or_else(|| {
            state
                .source_identity(res.source)
                .map(|source| source.version)
        })
        .unwrap_or(0);
    state.delayed.push(DelayedTrigger {
        controller: res.controller,
        when: DelayedWhen::Observed {
            watch,
            when,
            after: state.journal.last_seq(),
        },
        action: DelayedAction::TriggerAbout {
            source: res.source,
            source_version,
            effects,
            text: res.text,
            object: about.object,
            version: about.version,
            subject: None,
        },
    });
}

#[cfg(test)]
mod tests;
