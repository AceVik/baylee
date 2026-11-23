//! Independent target groups retain their own recipients and incarnations.

use super::{Resolution, subjects};
use crate::state::GameState;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{Filter, TargetSlot};
use baylee_core::ids::PlayerId;

fn controller_from_lki(
    state: &GameState,
    res: &Resolution,
    id: baylee_core::ids::ObjectId,
) -> Option<PlayerId> {
    res.target_lki
        .as_ref()
        .and_then(|all| {
            all.iter().find(|lki| {
                lki.id == id
                    && state
                        .object(id)
                        .is_none_or(|object| object.version != lki.version)
            })
        })
        .map(|lki| lki.controller)
        .or_else(|| state.last_known_controller(id))
}

pub(super) fn first_player(state: &GameState, res: &Resolution) -> Option<PlayerId> {
    let (players, chosen_player, targets) =
        if let Some(frame) = res.subject.target_restore_frames.first() {
            (
                frame.saved_target_players,
                frame.saved_chosen_player,
                frame.saved_targets.as_slice(),
            )
        } else {
            (
                res.target_players,
                res.chosen_player,
                res.targets.as_slice(),
            )
        };
    let slot_had_object_targets = state
        .object(res.on_stack)
        .is_some_and(|o| !o.targets.is_empty());
    players
        .iter()
        .next()
        .or_else(|| {
            (!slot_had_object_targets && targets.is_empty())
                .then_some(chosen_player)
                .flatten()
        })
        .or_else(|| {
            let id = *targets.first()?;
            controller_from_lki(state, res, id)
        })
        .filter(|&player| !state.has_left(player))
}

pub(super) fn second_player(state: &GameState, res: &Resolution) -> Option<PlayerId> {
    res.subject
        .second_players()
        .iter()
        .next()
        .or_else(|| {
            let id = *res.second_targets.first()?;
            controller_from_lki(state, res, id)
        })
        .filter(|&player| !state.has_left(player))
}

pub(super) fn third_player(state: &GameState, res: &Resolution) -> Option<PlayerId> {
    res.subject
        .third_players()
        .iter()
        .next()
        .or_else(|| {
            let id = *res.subject.third_targets().first()?;
            controller_from_lki(state, res, id)
        })
        .filter(|&player| !state.has_left(player))
}

pub(super) fn destroy(
    state: &mut GameState,
    res: &Resolution,
    slots: &[TargetSlot],
    no_regen: bool,
) {
    let mut references = Vec::new();
    for slot in slots {
        match slot {
            TargetSlot::This => references.extend(subjects::source(state, res)),
            other => {
                references.extend(
                    res.slot_targets(*other)
                        .iter()
                        .filter_map(|&id| subjects::target(state, res, id)),
                );
            }
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    let targets: Vec<_> = references
        .into_iter()
        .filter(|&reference| {
            subjects::on_battlefield(state, reference) && seen.insert(reference.object)
        })
        .map(|reference| reference.object)
        .collect();
    crate::sba::destroy_all(state, &targets, no_regen);
}

pub(super) fn graveyard_count(
    state: &GameState,
    res: &Resolution,
    player: PlayerId,
    filter: &Filter,
) -> u32 {
    let count = state
        .zones
        .list(ZoneLocation::Graveyard(player))
        .iter()
        .filter(|&&id| {
            state.object(id).is_some_and(|object| {
                crate::eval::matches_with_context(
                    filter,
                    state,
                    object,
                    res.controller,
                    res.rule_context(),
                )
            })
        })
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}
