//! Independent target groups retain their own recipients and incarnations.

use super::{Resolution, subjects};
use crate::state::GameState;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{Filter, TargetSlot};
use baylee_core::ids::PlayerId;

pub(super) fn second_player(state: &GameState, res: &Resolution) -> Option<PlayerId> {
    res.subject
        .second_players()
        .iter()
        .next()
        .or_else(|| {
            let id = *res.second_targets.first()?;
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
            TargetSlot::First | TargetSlot::Second => {
                let targets = if *slot == TargetSlot::First {
                    res.targets.as_slice()
                } else {
                    res.second_targets.as_slice()
                };
                references.extend(
                    targets
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
