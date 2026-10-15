//! Symmetric creature-type restrictions on a pair of announced targets.

use super::{GameObject, GameState, RuleContext, TargetSpec};
use crate::object::Characteristics;
use baylee_cards_dsl::Filter;
use baylee_core::ids::ObjectId;
use baylee_core::types::SubtypeSet;

pub(crate) fn shares(a: &Characteristics, b: &Characteristics) -> bool {
    a.subtypes
        .iter()
        .any(|kind| SubtypeSet::ALL_CREATURE.contains(kind) && b.subtypes.contains(kind))
}

pub(crate) fn reads_pair(filter: &Filter) -> bool {
    match filter {
        Filter::SharesCreatureTypeWithOtherTarget => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(reads_pair),
        Filter::Not(inner) => reads_pair(inner),
        _ => false,
    }
}

/// Do not offer a first choice which cannot be completed with a distinct,
/// legal second creature. Both slots use the same printed restriction.
pub(super) fn retain_partners(
    state: &GameState,
    spec: &TargetSpec,
    context: RuleContext,
    options: &mut Vec<ObjectId>,
) {
    let TargetSpec::Object(filter) = spec else {
        return;
    };
    if context.reference.is_some() || !reads_pair(filter) {
        return;
    }
    let candidates = options.clone();
    options.retain(|id| {
        state.object(*id).is_some_and(|object| {
            candidates.iter().any(|other| {
                id != other
                    && state.object(*other).is_some_and(|other| {
                        shares(object.characteristics(), other.characteristics())
                    })
            })
        })
    });
}

/// At resolution the counterpart is its announced incarnation. A card that
/// left and returned cannot substitute its new types for the departed target.
pub(super) fn stack_options(
    state: &GameState,
    stack: &GameObject,
    spec: &TargetSpec,
    context: RuleContext,
    reference: Option<baylee_core::ids::DamageSourceRef>,
    retargeting: bool,
) -> Option<Vec<ObjectId>> {
    let TargetSpec::Object(filter) = spec else {
        return None;
    };
    if !reads_pair(filter) {
        return None;
    }
    let other = if retargeting {
        None
    } else {
        let Some(identity) = reference else {
            return Some(Vec::new());
        };
        state.damage_source(identity.object, Some(identity.version))
    };
    let mut options = super::target_options_with_context(
        &TargetSpec::Object(&Filter::Any),
        state,
        stack.controller,
        context,
    );
    options.retain(|id| {
        state.object(*id).is_some_and(|object| {
            matches(
                filter,
                state,
                object,
                stack,
                context,
                retargeting
                    || (state.source_identity(object.id) != reference
                        && other.is_some_and(|other| {
                            shares(object.characteristics(), other.characteristics())
                        })),
            )
        })
    });
    Some(options)
}

fn matches(
    filter: &Filter,
    state: &GameState,
    object: &GameObject,
    stack: &GameObject,
    context: RuleContext,
    shared: bool,
) -> bool {
    match filter {
        Filter::SharesCreatureTypeWithOtherTarget => shared,
        Filter::And(parts) => parts
            .iter()
            .all(|part| matches(part, state, object, stack, context, shared)),
        Filter::Or(parts) => parts
            .iter()
            .any(|part| matches(part, state, object, stack, context, shared)),
        Filter::Not(inner) => !matches(inner, state, object, stack, context, shared),
        other => super::matches_for_x(
            other,
            state,
            object,
            stack.controller,
            context,
            Some(stack.x_value),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::object::ObjectKind;
    use crate::zone::{ZoneLocation, ZonePosition};
    use baylee_core::generated::subtypes::{artifact, creature, land};
    use baylee_core::ids::{PlayerId, SubtypeId};
    use baylee_core::types::TypeSet;

    const YOU: PlayerId = PlayerId::new(0);
    const PAIR: Filter = Filter::And(&[
        Filter::YOUR_CREATURE,
        Filter::SharesCreatureTypeWithOtherTarget,
    ]);

    fn creature(state: &mut GameState, kinds: &[SubtypeId]) -> ObjectId {
        let name = state.names.intern("pair candidate");
        let id = state.create_bare(YOU, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        let object = state.object_mut(id).unwrap();
        let mut base = (*object.base).clone();
        base.types = TypeSet::CREATURE;
        base.subtypes = SubtypeSet::from_slice(kinds);
        base.power = Some(2);
        base.toughness = Some(2);
        object.base = std::sync::Arc::new(base);
        state.invalidate_projections();
        state.refresh_characteristics();
        id
    }

    #[test]
    fn pair_options_exclude_dead_ends_and_noncreature_subtype_overlap() {
        let mut state =
            GameState::from_preset(&preset(581, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let wolf = creature(&mut state, &[creature::WOLF]);
        let other_wolf = creature(&mut state, &[creature::WOLF]);
        let bear = creature(&mut state, &[creature::BEAR, artifact::FOOD, land::CAVE]);
        let human = creature(&mut state, &[creature::HUMAN, artifact::FOOD, land::CAVE]);
        let spec = TargetSpec::Object(&PAIR);
        let mut context = super::super::live_context(&state, ObjectId::NO_SOURCE);
        let options = super::super::target_options_with_context(&spec, &state, YOU, context);
        assert_eq!(options, vec![wolf, other_wolf]);
        assert!(!options.contains(&bear));
        assert!(!options.contains(&human));
        context.reference = Some(wolf);
        assert_eq!(
            super::super::target_options_with_context(&spec, &state, YOU, context),
            vec![other_wolf]
        );
    }

    #[test]
    fn pair_resolution_uses_departed_incarnation_after_return_with_different_types() {
        let mut state =
            GameState::from_preset(&preset(582, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let first = creature(&mut state, &[creature::WOLF]);
        let second = creature(&mut state, &[creature::WOLF]);
        let name = state.names.intern("pair ability");
        let stack = state.create_bare(YOU, ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);
        state.object_mut(stack).unwrap().targets.push(first);
        state
            .object_mut(stack)
            .unwrap()
            .set_second(smallvec::smallvec![second], None);
        let refs = crate::sources::TargetReferences {
            first: state.capture_target_group(&[first]),
            second: state.capture_target_group(&[second]),
        };
        state.bind_target_references(stack, &refs);
        state
            .move_object(
                first,
                ZoneLocation::Graveyard(YOU),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                first,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        let object = state.object_mut(first).unwrap();
        let mut base = (*object.base).clone();
        base.subtypes = SubtypeSet::from_slice(&[creature::BEAR]);
        object.base = std::sync::Arc::new(base);
        state.invalidate_projections();
        state.refresh_characteristics();
        let stack = state.object(stack).unwrap();
        let options = super::super::stack_target_options_relative(
            &state,
            stack,
            &TargetSpec::Object(&PAIR),
            state.recorded_target_reference(stack.id, false, 0),
        )
        .0;
        assert!(
            options.contains(&second),
            "the surviving Wolf reads the original Wolf, not the returned Bear"
        );
    }
}
