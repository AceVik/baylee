//! Resolution-time attachment, including untargeted event subjects.
use super::{Resolution, TargetSpec, subjects, zones};
use crate::{object::Status, state::GameState, zone::Zone};
use baylee_cards_dsl::{AbilityDef, Effect};
use baylee_core::ids::TargetRef;
use baylee_core::{
    generated::subtypes,
    ids::{DamageSourceRef, ObjectId},
    types::TypeSet,
};

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
        attach_to(state, source, host);
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

pub(super) fn attach_to(state: &mut GameState, source: DamageSourceRef, host: ObjectId) {
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
    if can_attach_object(state, source.object, host) {
        state.attach(source.object, host);
    }
}

/// CR 301.5c permits a creature Equipment with reconfigure to attach through
/// any legal attach effect, including other cards' spells and abilities.
pub(super) fn can_attach_object(state: &GameState, attachment: ObjectId, host: ObjectId) -> bool {
    let (Some(object), Some(recipient)) = (state.object(attachment), state.object(host)) else {
        return false;
    };
    let chars = object.characteristics();
    if attachment == host
        || object.zone != Zone::Battlefield
        || object.status.contains(Status::PHASED_OUT)
        || chars.types.contains(TypeSet::BATTLE)
    {
        return false;
    }
    if chars.subtypes.contains(subtypes::enchantment::AURA) {
        return crate::aura_bindings::can_attach(state, attachment, host);
    }
    let equipment = chars.subtypes.contains(subtypes::artifact::EQUIPMENT);
    let required = if equipment {
        TypeSet::CREATURE
    } else if chars.subtypes.contains(subtypes::artifact::FORTIFICATION) {
        TypeSet::LAND
    } else {
        return false;
    };
    if chars.types.contains(TypeSet::CREATURE)
        && !(equipment && object.abilities(state.bases.as_ref()).iter().any(|ability| {
            matches!(ability, AbilityDef::Activated { effects, .. }
                 | AbilityDef::ActivatedConditional { effects, .. } if effects.contains(&Effect::ReconfigureAttach))
        }))
    {
        return false;
    }
    recipient.zone == Zone::Battlefield
        && !recipient.status.contains(Status::PHASED_OUT)
        && recipient.characteristics().types.contains(required)
        && !crate::eval::protected_from(state, host, attachment)
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

fn recipient(
    state: &GameState,
    res: &Resolution,
    slot: baylee_cards_dsl::TargetSlot,
) -> Option<TargetRef> {
    use baylee_cards_dsl::TargetSlot;
    if let Some(object) = slot_references(state, res, slot)
        .into_iter()
        .find(|&r| subjects::on_battlefield(state, r))
    {
        return Some(TargetRef::Object(object));
    }
    let players = match slot {
        TargetSlot::This => return None,
        TargetSlot::First => res
            .subject
            .target_restore_frames
            .first()
            .map_or(res.target_players, |frame| frame.saved_target_players),
        TargetSlot::Second => res.subject.second_players(),
        TargetSlot::Third => res.subject.third_players(),
    };
    players
        .iter()
        .find(|&p| !state.has_left(p))
        .map(TargetRef::Player)
}

fn can_attach_to_recipient(state: &GameState, source: DamageSourceRef, host: TargetRef) -> bool {
    subjects::on_battlefield(state, source)
        && match host {
            TargetRef::Object(target) => {
                subjects::on_battlefield(state, target)
                    && can_attach_object(state, source.object, target.object)
            }
            TargetRef::Player(player) => {
                state.object(source.object).is_some_and(|object| {
                    object
                        .characteristics()
                        .subtypes
                        .contains(subtypes::enchantment::AURA)
                }) && crate::aura_bindings::can_attach_player(state, source.object, player)
            }
        }
}

pub(super) fn attach_selected(state: &mut GameState, objects: &[DamageSourceRef], host: TargetRef) {
    for &source in objects {
        if can_attach_to_recipient(state, source, host) {
            match host {
                TargetRef::Object(target) => state.attach(source.object, target.object),
                TargetRef::Player(player) => state.attach_to_player(source.object, player),
            }
        }
    }
}

pub(super) fn attach_slot(
    state: &mut GameState,
    res: &mut Resolution,
    object: baylee_cards_dsl::TargetSlot,
    to: baylee_cards_dsl::TargetSlot,
    optional: bool,
) -> Option<super::Pending> {
    let host = recipient(state, res, to)?;
    let objects: Vec<DamageSourceRef> = slot_references(state, res, object)
        .into_iter()
        .filter(|&att| can_attach_to_recipient(state, att, host))
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
    attach_selected(state, &objects, host);
    None
}
