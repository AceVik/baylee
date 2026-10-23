//! Matching a name chosen by the rules source.

use crate::state::GameState;
use baylee_core::ids::{NameRef, ObjectId};

pub(super) fn named_by_source(state: &GameState, this: ObjectId, candidate: NameRef) -> bool {
    state.object(this).is_some_and(|source| {
        let current = source.rules_acquisition();
        source.riders.iter().any(|rider| {
            matches!(rider, crate::object::Rider::ChosenName { acquisition, name, .. } if *acquisition == current && *name == candidate)
        })
    })
}
