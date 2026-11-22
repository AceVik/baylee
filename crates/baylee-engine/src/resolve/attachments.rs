//! Resolution-time attachment, including untargeted event subjects.
use super::{Resolution, TargetSpec, subjects, zones};
use crate::{object::Status, state::GameState, zone::Zone};
use baylee_core::{generated::subtypes, ids::DamageSourceRef, types::TypeSet};

pub(super) fn attach(
    state: &mut GameState,
    res: &Resolution,
    target: TargetSpec,
    reconfigure: bool,
) {
    let Some(source) =
        subjects::source(state, res).filter(|identity| subjects::is_current(state, *identity))
    else {
        return;
    };
    let host = if matches!(target, TargetSpec::EventObject) {
        subjects::event(state, res)
            .filter(|identity| subjects::is_current(state, *identity))
            .map(|identity| identity.object)
    } else {
        zones::spec_object(state, res, target)
    };
    if let Some(host) = host {
        attach_to(state, source, host, reconfigure);
        return;
    }
    let player = match target {
        TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::PlayerWhoAttackedThisTurn
        | TargetSpec::OpponentWithMoreThanActivePlayer(_)
        | TargetSpec::OpponentWithMoreThanPlayer(..)
        | TargetSpec::AnyTarget
        | TargetSpec::PlayerOrObject(_)
        | TargetSpec::OpponentOrObject(_) => res.target_players.iter().next().or(res.chosen_player),
        TargetSpec::Player(rel) => {
            if res.targeted {
                res.target_players.iter().next().or(res.chosen_player)
            } else {
                super::players_of(rel, state, res.controller, res)
                    .into_iter()
                    .next()
            }
        }
        _ => None,
    };
    if let Some(player) = player {
        attach_to_player(state, source, player, reconfigure);
    }
}

pub(super) fn attach_to_player(
    state: &mut GameState,
    source: DamageSourceRef,
    player: baylee_core::ids::PlayerId,
    reconfigure: bool,
) {
    if !subjects::is_current(state, source) {
        return;
    }
    let Some(object) = state.object(source.object) else {
        return;
    };
    // An Aura spell records its intended player host before its normal entry procedure (CR 303.4b).
    if object.zone == Zone::Stack {
        state.attach_to_player(source.object, player);
        return;
    }
    if object.zone != Zone::Battlefield
        || reconfigure
        || object
            .characteristics()
            .types
            .intersects(TypeSet::CREATURE.union(TypeSet::BATTLE))
        || object.status.contains(Status::PHASED_OUT)
        || !object
            .characteristics()
            .subtypes
            .contains(subtypes::enchantment::AURA)
    {
        return;
    }
    if crate::aura_bindings::can_attach_player(state, source.object, player) {
        state.attach_to_player(source.object, player);
    }
}

pub(super) fn attach_to(
    state: &mut GameState,
    source: DamageSourceRef,
    host: baylee_core::ids::ObjectId,
    reconfigure: bool,
) {
    if !subjects::is_current(state, source) {
        return;
    }
    let Some(object) = state.object(source.object) else {
        return;
    };
    // An Aura spell records its intended host before its normal entry procedure.
    if object.zone == Zone::Stack {
        state.attach(source.object, host);
        return;
    }
    if object.zone != Zone::Battlefield
        || (!reconfigure
            && object
                .characteristics()
                .types
                .intersects(TypeSet::CREATURE.union(TypeSet::BATTLE)))
        || source.object == host
        || object.status.contains(Status::PHASED_OUT)
    {
        return;
    }
    let legal = if object
        .characteristics()
        .subtypes
        .contains(subtypes::enchantment::AURA)
    {
        crate::aura_bindings::can_attach(state, source.object, host)
    } else {
        let required = if object
            .characteristics()
            .subtypes
            .contains(subtypes::artifact::EQUIPMENT)
        {
            TypeSet::CREATURE
        } else if object
            .characteristics()
            .subtypes
            .contains(subtypes::artifact::FORTIFICATION)
        {
            TypeSet::LAND
        } else {
            return;
        };
        state.object(host).is_some_and(|host| {
            host.zone == Zone::Battlefield
                && host.characteristics().types.contains(required)
                && !host.status.contains(Status::PHASED_OUT)
        }) && !crate::eval::protected_from(state, host, source.object)
    };
    if legal {
        state.attach(source.object, host);
    }
}

pub(super) fn can_attach_object_to_host(
    state: &GameState,
    source: DamageSourceRef,
    host: DamageSourceRef,
) -> bool {
    if !subjects::on_battlefield(state, source)
        || !subjects::on_battlefield(state, host)
        || source.object == host.object
    {
        return false;
    }
    let Some(object) = state.object(source.object) else {
        return false;
    };
    if object
        .characteristics()
        .types
        .intersects(TypeSet::CREATURE.union(TypeSet::BATTLE))
    {
        return false;
    }
    if object
        .characteristics()
        .subtypes
        .contains(subtypes::enchantment::AURA)
    {
        crate::aura_bindings::can_attach(state, source.object, host.object)
    } else {
        let required = if object
            .characteristics()
            .subtypes
            .contains(subtypes::artifact::EQUIPMENT)
        {
            TypeSet::CREATURE
        } else if object
            .characteristics()
            .subtypes
            .contains(subtypes::artifact::FORTIFICATION)
        {
            TypeSet::LAND
        } else {
            return false;
        };
        state.object(host.object).is_some_and(|h| {
            h.zone == Zone::Battlefield
                && h.characteristics().types.contains(required)
                && !h.status.contains(Status::PHASED_OUT)
        }) && !crate::eval::protected_from(state, host.object, source.object)
    }
}

fn slot_references(
    state: &GameState,
    res: &Resolution,
    slot: baylee_cards_dsl::TargetSlot,
) -> Vec<DamageSourceRef> {
    match slot {
        baylee_cards_dsl::TargetSlot::This => subjects::source(state, res).into_iter().collect(),
        other => res
            .slot_targets(other)
            .iter()
            .filter_map(|&id| subjects::target(state, res, id))
            .collect(),
    }
}

pub(super) fn attach_slot(
    state: &mut GameState,
    res: &mut Resolution,
    object: baylee_cards_dsl::TargetSlot,
    to: baylee_cards_dsl::TargetSlot,
    optional: bool,
) -> Option<super::Pending> {
    let host = slot_references(state, res, to)
        .into_iter()
        .find(|&r| subjects::on_battlefield(state, r))?;
    let objects: Vec<DamageSourceRef> = slot_references(state, res, object)
        .into_iter()
        .filter(|&att| can_attach_object_to_host(state, att, host))
        .collect();
    if objects.is_empty() {
        return None;
    }
    if optional {
        if state.has_left(res.controller) {
            return None;
        }
        let source = super::resolving_ability(state, res);
        res.awaiting = Some(super::AwaitingOp::AttachSlot { objects, host });
        return Some(super::Pending::YesNo {
            player: res.controller,
            prompt: crate::choice::YesNoPrompt::MayDo,
            source,
        });
    }
    for att in objects {
        attach_to(state, att, host.object, false);
    }
    None
}
