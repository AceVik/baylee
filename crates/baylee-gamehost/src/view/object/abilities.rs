use baylee_engine::copiable_abilities::AbilityDefs;
use baylee_engine::object::{AbilityOrigin, GameObject, ObjectKind, Status};
use baylee_engine::state::GameState;
use baylee_view::{AbilitySource, TokenAbility};

/// Copy exceptions and merged lists carry their public printed provenance.
/// Never infer it from the physical card, a pointer or a hidden donor.
pub(super) fn sources(state: &GameState, obj: &GameObject, known: bool) -> Vec<AbilitySource> {
    if !known || obj.status.contains(Status::FACE_DOWN) {
        return Vec::new();
    }
    let Some(list) = state.printed_ability_list(obj.id) else {
        return Vec::new();
    };
    if !matches!(list.abilities, AbilityDefs::Shared(_)) {
        return Vec::new();
    }
    (0..list.abilities.len())
        .filter(|&index| {
            obj.kind != ObjectKind::AbilityOnStack
                || obj.ability.is_some_and(|a| a.index as usize == index)
        })
        .filter_map(|index| {
            let origin = list.origin(index);
            let rules = origin
                .origin
                .and_then(AbilityOrigin::printed)
                .map(super::rules_face);
            let token = origin
                .origin
                .and_then(AbilityOrigin::token)
                .map(|id| TokenAbility {
                    token: id.get() - 1,
                    index: origin.index,
                });
            (rules.is_some() || token.is_some()).then(|| AbilitySource {
                index: u32::try_from(index).expect("engine ability index"),
                rules,
                ability: origin.index,
                token,
                copy_modifier: origin.copy_modifier,
            })
        })
        .collect()
}
