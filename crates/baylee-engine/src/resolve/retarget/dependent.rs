//! Dependent graveyard targets are checked against the final targeted player.
use super::{Aim, Retarget, TargetReq, TargetSpec, eval};
use crate::{object::GameObject, state::GameState};
use baylee_core::ids::PlayerId;

fn player(retarget: &Retarget, candidate: Option<Aim>) -> Option<PlayerId> {
    retarget
        .was
        .iter()
        .enumerate()
        .find_map(|(slot, (group, old))| {
            if *group != 0 {
                return None;
            }
            let aim = if slot == retarget.now.len() {
                candidate.unwrap_or(*old)
            } else {
                retarget.now.get(slot).copied().flatten().unwrap_or(*old)
            };
            match aim {
                Aim::Player(player) => Some(player),
                Aim::Object(_) => None,
            }
        })
}

pub(super) fn requirement(
    obj: &GameObject,
    retarget: &Retarget,
    group: u8,
    candidate: Option<Aim>,
) -> Option<TargetReq> {
    let mut req = super::requirement(obj, group)?;
    if group == 1
        && let TargetSpec::CardInPlayersGraveyard(filter, _) = req.spec
    {
        req.spec = TargetSpec::CardInPlayersGraveyard(filter, player(retarget, candidate)?);
    }
    Some(req)
}

fn legal(state: &GameState, obj: &GameObject, req: &TargetReq, aim: Aim) -> bool {
    let Aim::Object(reference) = aim else {
        return false;
    };
    state.source_identity(reference.object) == Some(reference)
        && eval::stack_target_options(state, obj, &req.spec)
            .0
            .contains(&reference.object)
}

/// CR 115.7d permits retaining an already illegal target, but a newly changed
/// player may not make a formerly legal, unchanged graveyard target illegal.
pub(super) fn may_keep(
    state: &GameState,
    obj: &GameObject,
    retarget: &Retarget,
    group: u8,
    original: Aim,
) -> bool {
    if group != 1 {
        return true;
    }
    let Some(old) = super::requirement(obj, group) else {
        return false;
    };
    let Some(new) = requirement(obj, retarget, group, None) else {
        return false;
    };
    old.spec == new.spec || !legal(state, obj, &old, original) || legal(state, obj, &new, original)
}

pub(super) fn can_finish(
    state: &GameState,
    obj: &GameObject,
    retarget: &Retarget,
    candidate: Aim,
) -> bool {
    let slot = retarget.now.len();
    let (group, old_aim) = retarget.was[slot];
    if group == 1 {
        return candidate != old_aim || may_keep(state, obj, retarget, group, old_aim);
    }
    let Some(old) = obj.second_target_req() else {
        return true;
    };
    if !matches!(old.spec, TargetSpec::CardInPlayersGraveyard(..)) {
        return true;
    }
    let Some(new) = requirement(obj, retarget, 1, Some(candidate)) else {
        return false;
    };
    if old.spec == new.spec {
        return true;
    }
    let targets: Vec<_> = retarget
        .was
        .iter()
        .filter(|(group, _)| *group == 1)
        .map(|(_, aim)| *aim)
        .collect();
    let mut available: Vec<_> = eval::stack_target_options(state, obj, &new.spec)
        .0
        .into_iter()
        .filter_map(|id| state.source_identity(id).map(Aim::Object))
        .collect();
    for target in &targets {
        if !legal(state, obj, &old, *target) && !available.contains(target) {
            available.push(*target);
        }
    }
    available.len() >= targets.len()
}
