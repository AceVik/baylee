//! As-though permissions preserve abilities while ignoring all landwalk forms.
use super::*;
use baylee_cards_dsl::{Filter, Modifier};

#[test]
fn all_landwalk_permission_ignores_basic_and_filter_walks_but_not_flying() {
    for matching in [false, true] {
        let mut state = empty_state();
        let walker = creature(&mut state, P0, 2, 2, KeywordSet::FORESTWALK);
        let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
        land_of_type(&mut state, P1, land::FOREST);
        if matching {
            permit(
                &mut state,
                walker,
                Modifier::LandwalkMatching(&Filter::LAND),
            );
        }
        assert!(!can_block(&state, P1, blocker, walker));
        permit(&mut state, walker, Modifier::BlockableWithoutAnyLandwalk);
        assert!(can_block(&state, P1, blocker, walker));
        assert!(
            state
                .object(walker)
                .unwrap()
                .characteristics()
                .keywords
                .contains(KeywordSet::FORESTWALK)
        );
        state.object_mut(walker).unwrap().base_mut().keywords =
            KeywordSet::FORESTWALK.union(KeywordSet::FLYING);
        assert!(!can_block(&state, P1, blocker, walker));
    }
}

#[test]
fn all_landwalk_permission_applies_only_to_its_affected_attacker() {
    let mut state = empty_state();
    let walker = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let other = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    land_of_type(&mut state, P1, land::DESERT);
    for id in [walker, other] {
        permit(&mut state, id, Modifier::LandwalkMatching(&Filter::LAND));
    }
    permit(&mut state, walker, Modifier::BlockableWithoutAnyLandwalk);
    assert!(can_block(&state, P1, blocker, walker));
    assert!(!can_block(&state, P1, blocker, other));
}
