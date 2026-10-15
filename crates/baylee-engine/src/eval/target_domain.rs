//! Structural targeting permissions over every selected target instance.
use crate::{object::GameObject, state::GameState};
use baylee_cards_dsl::{AbilityDef, Filter, Modifier, TargetReq, TargetSpec};
use baylee_core::{generated::subtypes::creature::WALL, ids::ObjectId};

fn wall_filter(filter: &Filter) -> bool {
    match filter {
        Filter::HasSubtype(subtype) => *subtype == WALL,
        Filter::And(parts) => parts.iter().any(wall_filter),
        Filter::Or(parts) => !parts.is_empty() && parts.iter().all(wall_filter),
        _ => false,
    }
}

/// Only affirmative subtype constraints prove the domain, never board contents.
pub(crate) fn wall_spec(spec: TargetSpec) -> bool {
    match spec {
        TargetSpec::Object(filter)
        | TargetSpec::ObjectOfEachOpponent(filter)
        | TargetSpec::ObjectOfFirstTargetsPlayer(filter)
        | TargetSpec::ObjectOfEventPlayer(filter)
        | TargetSpec::ObjectControlledBy(filter, _) => wall_filter(filter),
        _ => false,
    }
}

/// Optional target instances participate even when the player chooses none.
pub(crate) fn walls(first: Option<TargetReq>, second: Option<TargetReq>) -> bool {
    wall_requirements([first, second].into_iter().flatten())
}

pub(crate) fn wall_requirements(requirements: impl IntoIterator<Item = TargetReq>) -> bool {
    let mut any = false;
    for req in requirements {
        if req.max == 0 && !req.count_is_x {
            continue;
        }
        any = true;
        if !wall_spec(req.spec) {
            return false;
        }
    }
    any
}

pub(crate) fn modes_wall_only(modes: &[baylee_cards_dsl::SpellMode], set: u8) -> bool {
    wall_requirements(
        modes
            .iter()
            .enumerate()
            .filter(|(index, _)| *index < 8 && set & (1 << index) != 0)
            .flat_map(|(_, mode)| [mode.targets, mode.second_targets].into_iter().flatten()),
    )
}

/// The actual selected definition, including grants from another printed card.
pub(crate) fn requirements(
    definition: &AbilityDef,
    mode: Option<u8>,
) -> (Option<TargetReq>, Option<TargetReq>) {
    match definition {
        AbilityDef::Activated {
            targets,
            second_targets,
            ..
        }
        | AbilityDef::ActivatedConditional {
            targets,
            second_targets,
            ..
        }
        | AbilityDef::Loyalty {
            targets,
            second_targets,
            ..
        }
        | AbilityDef::Triggered {
            targets,
            second_targets,
            ..
        }
        | AbilityDef::Spell {
            targets,
            second_targets,
            ..
        } => (*targets, *second_targets),
        AbilityDef::SagaChapter { targets, .. } => (*targets, None),
        AbilityDef::ModalTriggered { modes, .. }
        | AbilityDef::ModalActivated { modes, .. }
        | AbilityDef::ModalSpell { modes, .. } => mode
            .and_then(|index| modes.get(usize::from(index)))
            .map_or((None, None), |mode| (mode.targets, mode.second_targets)),
        _ => (None, None),
    }
}

pub(crate) fn ability_walls(definition: &AbilityDef, mode: Option<u8>) -> bool {
    let (first, second) = requirements(definition, mode);
    walls(first, second)
}

/// Frozen requirements win over a definition and do not require a live source.
pub(crate) fn stack_walls(object: &GameObject) -> bool {
    let inherited = object
        .ability
        .and_then(|location| {
            object.own_abilities.as_ref().and_then(|list| {
                crate::copiable_abilities::AbilityDefs::from(list)
                    .get(location.index as usize)
                    .map(|definition| requirements(definition, object.mode_index))
            })
        })
        .unwrap_or_default();
    walls(
        object.target_req.or(inherited.0),
        object.second_target_req().or(inherited.1),
    )
}

/// Apply this rule only to a targeting menu, never to a nontargeting choice.
pub(crate) fn retain(state: &GameState, options: &mut Vec<ObjectId>, wall_only: bool) {
    if !wall_only {
        return;
    }
    options.retain(|id| {
        !state.effects.iter().any(|effect| {
            matches!(effect.modifier, Modifier::CantBeTargetedByWallOnly)
                && state
                    .object(*id)
                    .is_some_and(|object| crate::effects::applies_to(state, effect, object))
        })
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    static WALL_FILTER: Filter = Filter::HasSubtype(WALL);
    static WALL_AND_CREATURE: Filter = Filter::And(&[WALL_FILTER, Filter::CREATURE]);
    static WALL_OR_CREATURE: Filter = Filter::Or(&[WALL_FILTER, Filter::CREATURE]);

    #[test]
    fn proves_constraints_without_inspecting_the_board() {
        assert!(wall_filter(&WALL_FILTER));
        assert!(wall_filter(&WALL_AND_CREATURE));
        assert!(!wall_filter(&WALL_OR_CREATURE));
        assert!(!wall_filter(&Filter::Not(&Filter::CREATURE)));
        assert!(!wall_filter(&Filter::Or(&[])));
        assert!(!wall_filter(&Filter::And(&[])));
        assert!(!wall_spec(TargetSpec::PlayerOrObject(&WALL_FILTER)));
        assert!(!wall_spec(TargetSpec::Spell(&WALL_FILTER)));
        assert!(!walls(None, None));
    }

    #[test]
    fn optional_broad_domain_still_broadens_the_whole_spell() {
        let wall = Some(TargetReq::one(TargetSpec::Object(&WALL_FILTER)));
        let broad = Some(TargetReq::up_to(TargetSpec::Object(&Filter::ARTIFACT), 1));
        assert!(walls(wall, wall));
        assert!(!walls(wall, broad));
        assert!(!walls(broad, wall));
        assert!(walls(
            wall,
            Some(TargetReq::up_to(TargetSpec::AnyPlayer, 0))
        ));
    }
}
