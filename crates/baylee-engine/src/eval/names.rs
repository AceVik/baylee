//! Matching a name chosen by the rules source.

use crate::state::GameState;
use crate::text_changes::RuleContext;
use baylee_core::ids::NameRef;

pub(super) fn named_by_source(state: &GameState, context: RuleContext, candidate: NameRef) -> bool {
    if let Some(name) = context.resolved_name {
        return name == candidate;
    }
    state.object(context.source).is_some_and(|source| {
        let current = source.rules_acquisition();
        source.riders.iter().any(|rider| {
            matches!(rider, crate::object::Rider::ChosenName { acquisition, name, .. } if *acquisition == current && *name == candidate)
        })
    })
}
