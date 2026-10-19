//! A new pair is checked jointly, never against its superseded counterpart.
use super::{Aim, Retarget, TargetSpec};
use crate::{eval, object::GameObject, state::GameState};

fn is_pair(obj: &GameObject) -> bool {
    [obj.target_req, obj.second_target_req()]
        .into_iter()
        .flatten()
        .any(|req| matches!(req.spec, TargetSpec::Object(filter) if eval::target_pair::reads_pair(filter)))
}

pub(super) fn can_finish(
    state: &GameState,
    obj: &GameObject,
    retarget: &Retarget,
    candidate: Aim,
) -> bool {
    if !is_pair(obj) {
        return true;
    }
    // This relation is explicitly a pair of one-object sequential slots.
    if retarget.was.len() != 2 {
        return false;
    }
    if retarget.now.len() == 1 {
        let first = retarget.now[0].unwrap_or(retarget.was[0].1);
        return valid(state, retarget, [first, candidate]);
    }
    let Some(req) = super::requirement(obj, retarget.was[1].0) else {
        return false;
    };
    // An unchanged original may remain illegal. Otherwise the new second
    // target must satisfy its ordinary target restrictions as well.
    valid(state, retarget, [candidate, retarget.was[1].1])
        || eval::stack_retarget_options(state, obj, &req.spec)
            .0
            .into_iter()
            .any(|id| {
                state.source_identity(id).is_some_and(|reference| {
                    valid(state, retarget, [candidate, Aim::Object(reference)])
                })
            })
}

fn valid(state: &GameState, retarget: &Retarget, pair: [Aim; 2]) -> bool {
    // Choosing no new targets is legal even if the original pair no longer is.
    if pair == [retarget.was[0].1, retarget.was[1].1] {
        return true;
    }
    let [Aim::Object(first), Aim::Object(second)] = pair else {
        return false;
    };
    if first == second {
        return false;
    }
    state
        .damage_source(first.object, Some(first.version))
        .zip(state.damage_source(second.object, Some(second.version)))
        .is_some_and(|(first, second)| {
            eval::target_pair::shares(first.characteristics(), second.characteristics())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::object::ObjectKind;
    use crate::zone::ZoneLocation;
    use baylee_cards_dsl::{Filter, TargetReq};
    use baylee_core::generated::subtypes::creature::{BEAR, WOLF};
    use baylee_core::ids::{PlayerId, SubtypeId};
    use baylee_core::types::{SubtypeSet, TypeSet};

    const PAIR: Filter = Filter::And(&[
        Filter::YOUR_CREATURE,
        Filter::SharesCreatureTypeWithOtherTarget,
    ]);

    #[test]
    fn new_pair_is_checked_together_and_can_change_both_creature_types() {
        let mut state =
            GameState::from_preset(&preset(593, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let you = PlayerId::new(0);
        let mut add = |kind: SubtypeId| {
            let name = state.names.intern("paired creature");
            let id = state.create_bare(you, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
            let object = state.object_mut(id).unwrap();
            let mut base = (*object.base).clone();
            base.types = TypeSet::CREATURE;
            base.subtypes = SubtypeSet::from_slice(&[kind]);
            object.base = std::sync::Arc::new(base);
            id
        };
        let [wolf, other_wolf, bear, other_bear] = [add(WOLF), add(WOLF), add(BEAR), add(BEAR)];
        state.invalidate_projections();
        state.refresh_characteristics();
        let name = state.names.intern("paired ability");
        let spell = state.create_bare(you, ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);
        let req = TargetReq::one(TargetSpec::Object(&PAIR));
        let object = state.object_mut(spell).unwrap();
        object.targets.push(wolf);
        object.target_req = Some(req);
        object.set_second(smallvec::smallvec![other_wolf], Some(req));
        let aim = |id| Aim::Object(state.source_identity(id).unwrap());
        let mut retarget = Retarget {
            spell,
            change_to: None,
            was: vec![(0, aim(wolf)), (1, aim(other_wolf))],
            now: Vec::new(),
            copy: false,
            players_only: false,
        };
        let object = state.object(spell).unwrap();
        assert!(
            eval::stack_retarget_options(&state, object, &req.spec)
                .0
                .contains(&bear)
        );
        assert!(can_finish(&state, object, &retarget, aim(bear)));
        retarget.now.push(Some(aim(bear)));
        // Neither branch has written its staged first choice to the ability.
        // Both still ask about the original second target, but the permitted
        // completion and eventual first target differ.
        let mut other_branch = retarget.clone();
        other_branch.now[0] = Some(aim(other_bear));
        let fingerprint = |choice: Retarget| {
            crate::resolve::AwaitingOp::NewTargets(Box::new(choice)).search_fingerprint()
        };
        assert_ne!(
            fingerprint(retarget.clone()),
            fingerprint(other_branch.clone())
        );
        assert!(can_finish(&state, object, &other_branch, aim(bear)));
        assert!(!can_finish(&state, object, &other_branch, aim(other_bear)));
        assert_eq!(object.targets.as_slice(), &[wolf]);
        assert_eq!(object.second_targets(), &[other_wolf]);
        assert!(!can_finish(&state, object, &retarget, aim(other_wolf)));
        assert!(!can_finish(&state, object, &retarget, aim(bear)));
        assert!(can_finish(&state, object, &retarget, aim(other_bear)));
        retarget.now[0] = None;
        assert!(can_finish(&state, object, &retarget, aim(other_wolf)));
        assert!(!can_finish(&state, object, &retarget, aim(other_bear)));
    }
}
