//! Stack predicates retain the kind and source of the original ability.
use super::{Filter, GameObject, GameState, PlayerId, RuleContext};
use crate::object::{ObjectKind, Rider};
use baylee_cards_dsl::AbilityDef;

pub(super) fn activated_source_matches(
    filter: &Filter,
    state: &GameState,
    object: &GameObject,
    you: PlayerId,
    context: RuleContext,
) -> bool {
    if object.kind != ObjectKind::AbilityOnStack {
        return false;
    }
    let Some(ability) = object.ability else {
        return false;
    };
    let activated = object.riders.contains(&Rider::GrantedActivated)
        || object
            .own_abilities
            .as_ref()
            .and_then(|list| list.get(ability.index as usize))
            .is_some_and(|definition| {
                matches!(
                    definition,
                    AbilityDef::Activated { .. }
                        | AbilityDef::ModalActivated { .. }
                        | AbilityDef::ActivatedConditional { .. }
                        | AbilityDef::Loyalty { .. }
                )
            });
    if !activated {
        return false;
    }
    // CR 113.7a: live information while this incarnation remains, otherwise LKI.
    let identity = state.recorded_ability_source(object.id).or_else(|| {
        object.riders.iter().find_map(|rider| match rider {
            Rider::AbilitySourceVersion(version) => Some(baylee_core::ids::DamageSourceRef {
                object: ability.source,
                version: *version,
            }),
            _ => None,
        })
    });
    identity
        .and_then(|source| state.source_object(source))
        .is_some_and(|source| super::matches_with_context(filter, state, source, you, context))
}
