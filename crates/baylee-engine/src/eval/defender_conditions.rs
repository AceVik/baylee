//! Existential defending-player restrictions, with the attacker's exact defender first.
use crate::{state::GameState, text_changes::RuleContext};
use baylee_cards_dsl::Filter;
use baylee_core::ids::PlayerId;

pub(super) fn some_defender_controls_none(
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    filter: &Filter,
) -> bool {
    let none = |player| {
        !state.has_left(player)
            && !state
                .battlefield_seen()
                .filter_map(|id| state.object(id))
                .any(|object| {
                    object.controller == player
                        && super::matches_with_context(filter, state, object, you, context)
                })
    };
    if let Some(player) = crate::combat::historical_defending_player(state, context.source) {
        return none(player);
    }
    crate::combat::defender_options(state, state.turn.active)
        .into_iter()
        .filter_map(|defender| crate::combat::defending_player(state, defender))
        .any(none)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        engine::synthetic::{SyntheticLookup, preset},
        object::ObjectKind,
        zone::ZoneLocation,
    };
    use baylee_core::{ids::Defender, types::TypeSet};

    #[test]
    fn one_eligible_defender_suffices_until_the_source_has_a_specific_defender() {
        let mut preset = preset(58, &[]);
        preset.seats.push(preset.seats[1].clone());
        let mut state = GameState::from_preset(&preset, &SyntheticLookup::new(vec![])).unwrap();
        let you = PlayerId::new(0);
        let first = PlayerId::new(1);
        let second = PlayerId::new(2);
        state.turn.active = you;
        let name = state.names.intern("Defender condition fixture");
        let source = state.create_bare(you, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        let land = state.create_bare(
            first,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        state.object_mut(land).unwrap().base_mut().types = TypeSet::LAND;
        let context = crate::eval::live_context(&state, source);
        assert!(some_defender_controls_none(
            &state,
            you,
            context,
            &Filter::LAND
        ));
        let other = state.create_bare(
            second,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        state.object_mut(other).unwrap().base_mut().types = TypeSet::LAND;
        assert!(!some_defender_controls_none(
            &state,
            you,
            context,
            &Filter::LAND
        ));
        state.object_mut(other).unwrap().base_mut().types = TypeSet::ARTIFACT;
        state
            .combat
            .declare_attackers([crate::combat::AttackerInfo {
                creature: source,
                defending: Defender::Player(first),
                blocked: false,
                band: None,
            }]);
        assert!(!some_defender_controls_none(
            &state,
            you,
            context,
            &Filter::LAND
        ));
    }
}
