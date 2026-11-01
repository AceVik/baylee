//! A resolving sweep's self references name an incarnation, not an arena slot.
use crate::object::GameObject;
use crate::resolve::{Resolution, subjects};
use crate::state::GameState;
use baylee_cards_dsl::Filter;

pub(super) fn matches(
    filter: &Filter,
    state: &GameState,
    object: &GameObject,
    res: &Resolution,
) -> bool {
    if object.is_merged_component() {
        return false;
    }
    let same_source = || {
        subjects::source(state, res)
            .is_some_and(|source| source.object == object.id && source.version == object.version)
    };
    match filter {
        Filter::This => same_source(),
        Filter::Another => !same_source(),
        Filter::AttachedToBySource => subjects::event_subject(state, res).map_or_else(
            || {
                crate::eval::matches_with_context(
                    filter,
                    state,
                    object,
                    res.controller,
                    res.rule_context(),
                )
            },
            |subject| subject.object == object.id && subject.version == object.version,
        ),
        Filter::Reference => subjects::event(state, res).map_or_else(
            || {
                crate::eval::matches_with_context(
                    filter,
                    state,
                    object,
                    res.controller,
                    res.rule_context(),
                )
            },
            |event| event.object == object.id && event.version == object.version,
        ),
        Filter::And(parts) => parts.iter().all(|part| matches(part, state, object, res)),
        Filter::Or(parts) => parts.iter().any(|part| matches(part, state, object, res)),
        Filter::Not(part) => !matches(part, state, object, res),
        _ => crate::eval::matches_with_context(
            filter,
            state,
            object,
            res.controller,
            res.rule_context(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::event::Cause;
    use crate::object::ObjectKind;
    use crate::resolve::SubjectContext;
    use crate::zone::{Zone, ZoneLocation, ZonePosition};
    use baylee_cards_dsl::Effect;
    use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
    use baylee_core::types::TypeSet;
    use smallvec::SmallVec;

    fn fixture() -> (GameState, Resolution, ObjectId) {
        let you = PlayerId::new(0);
        let mut state =
            GameState::from_preset(&preset(711, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let name = state.names.intern("Sweep source fixture");
        let source = state.create_bare(you, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        let other = state.create_bare(you, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        for id in [source, other] {
            state.object_mut(id).unwrap().base_mut().types = TypeSet::CREATURE;
            state.object_mut(id).unwrap().regeneration_shields = 1;
        }
        let version = state.object(source).unwrap().version;
        let res = Resolution {
            source,
            on_stack: source,
            controller: you,
            effects: vec![],
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
            target_lki: None,
            subject: SubjectContext::default().with_source_version(Some(version)),
            text: crate::text_changes::TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
            loot: None,
            draw_watch: None,
            this_way_from: None,
            coin_stake: None,
        };
        (state, res, other)
    }

    fn blink_source(state: &mut GameState, res: &Resolution) {
        state
            .move_object(
                res.source,
                ZoneLocation::Exile(res.controller),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                res.source,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }

    #[test]
    fn another_creature_sweep_exempts_only_the_original_source_incarnation() {
        for blink in [false, true] {
            let (mut state, mut res, other) = fixture();
            if blink {
                blink_source(&mut state, &res);
            }
            assert!(
                super::super::exec(
                    &mut state,
                    &mut res,
                    Effect::DestroyAll {
                        filter: &Filter::ANOTHER_CREATURE,
                        no_regen: true,
                    }
                )
                .is_none()
            );
            assert_eq!(state.object(other).unwrap().zone, Zone::Graveyard);
            assert_eq!(
                state.object(res.source).unwrap().zone,
                if blink {
                    Zone::Graveyard
                } else {
                    Zone::Battlefield
                }
            );
        }
    }

    #[test]
    fn nested_self_predicates_keep_the_captured_incarnation() {
        let (mut state, res, other) = fixture();
        let filter = Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::Or(&[Filter::This]))]);
        assert!(!matches(
            &filter,
            &state,
            state.object(res.source).unwrap(),
            &res
        ));
        assert!(matches(&filter, &state, state.object(other).unwrap(), &res));
        blink_source(&mut state, &res);
        assert!(matches(
            &filter,
            &state,
            state.object(res.source).unwrap(),
            &res
        ));
    }
}
