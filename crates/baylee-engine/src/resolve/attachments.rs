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

pub(super) fn can_attach_to_recipient(
    state: &GameState,
    source: DamageSourceRef,
    host: TargetRef,
) -> bool {
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

/// Resolve an arbitrary [`TargetSpec`] to its live, unphased battlefield permanent(s).
fn resolve_spec_attachments(
    state: &GameState,
    res: &Resolution,
    spec: TargetSpec,
) -> smallvec::SmallVec<[DamageSourceRef; 4]> {
    let mut out = smallvec::SmallVec::new();
    match spec {
        TargetSpec::ThisObject => {
            if let Some(r) =
                subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))
            {
                out.push(r);
            }
        }
        TargetSpec::EventObject => {
            if let Some(r) =
                subjects::event(state, res).filter(|r| subjects::on_battlefield(state, *r))
            {
                out.push(r);
            }
        }
        TargetSpec::Object(filter) => {
            let you = res.controller;
            if res.targeted {
                for &id in &res.targets {
                    if let Some(r) = subjects::target(state, res, id)
                        .filter(|r| subjects::on_battlefield(state, *r))
                        && state.object(id).is_some_and(|obj| {
                            super::matches_in_resolution(filter, state, obj, you, res, Some(you))
                        })
                    {
                        out.push(r);
                    }
                }
            } else {
                for id in state.battlefield_seen() {
                    if let Some(obj) = state.object(id)
                        && super::matches_in_resolution(filter, state, obj, you, res, Some(you))
                    {
                        out.push(DamageSourceRef {
                            object: id,
                            version: obj.version,
                        });
                    }
                }
            }
        }
        other => {
            if let Some(id) = zones::spec_object(state, res, other)
                && let Some(r) = subjects::target(state, res, id)
                    .or_else(|| state.source_identity(id))
                    .filter(|r| subjects::on_battlefield(state, *r))
            {
                out.push(r);
            }
        }
    }
    out
}

/// Execute [`Effect::Unattach`] (`CR 701.3d`): unattach each matching attachment
/// from whatever permanent or player it is currently attached to.
pub(super) fn unattach(state: &mut GameState, res: &Resolution, target: TargetSpec) {
    let objects = resolve_spec_attachments(state, res, target);
    let mut changed = false;
    for reference in objects {
        if let Some(obj) = state.object_mut(reference.object)
            && obj.version == reference.version
            && obj.zone == Zone::Battlefield
            && obj.is_attached()
        {
            obj.attached_to = None;
            obj.set_attached_player(None);
            changed = true;
        }
    }
    if changed {
        state.invalidate_projections();
    }
}

/// Execute [`Effect::AttachObject`]: attach `object` (source, event object,
/// target(s), or filter-matched permanents) to `to`, optionally asking first.
pub(super) fn attach_object(
    state: &mut GameState,
    res: &mut Resolution,
    object: TargetSpec,
    to: TargetSpec,
    optional: bool,
) -> Option<super::Pending> {
    let host = super::wave15b::resolve_spec_host(state, res, to)?;
    let objects: Vec<DamageSourceRef> = resolve_spec_attachments(state, res, object)
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

/// Start [`Effect::MoveAllAttachments`] (Glamer Spinners): choose a permanent
/// matching `recipient_filter` (with `reference` set to `from`) and move every
/// matching attachment that can legally attach to it, leaving illegal ones behind.
pub(super) fn move_all_attachments(
    state: &mut GameState,
    res: &mut Resolution,
    attachments: &'static baylee_cards_dsl::Filter,
    from: TargetSpec,
    recipient_filter: &'static baylee_cards_dsl::Filter,
    must_change_recipient: bool,
    optional: bool,
) -> Option<super::Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let from_ref = resolve_spec_attachments(state, res, from)
        .into_iter()
        .next()?;
    let has_matching_attachment = state.battlefield_seen().any(|id| {
        state.object(id).is_some_and(|o| {
            o.attached_to == Some(from_ref.object)
                && super::matches_in_resolution(attachments, state, o, you, res, Some(you))
        })
    });
    if !has_matching_attachment {
        return None;
    }
    let ctx = crate::text_changes::RuleContext {
        reference: Some(from_ref.object),
        ..res.rule_context()
    };
    let options: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            if must_change_recipient && id == from_ref.object {
                return false;
            }
            state.object(id).is_some_and(|host| {
                crate::eval::matches_with_context(recipient_filter, state, host, you, ctx)
            })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(super::AwaitingOp::MoveAllAttachments {
        from: from_ref,
        attachments,
    });
    Some(super::Pending::ChooseCards {
        player: you,
        options,
        min: u8::from(!optional),
        max: 1,
        prompt: crate::choice::ChoicePrompt::Generic,
        total: None,
    })
}

/// Resume [`Effect::MoveAllAttachments`]: move every matching attachment from
/// `from` that can legally attach to `chosen[0]`, leaving illegal ones behind.
pub(super) fn resume_move_all_attachments(
    state: &mut GameState,
    res: &Resolution,
    from: DamageSourceRef,
    attachments: &'static baylee_cards_dsl::Filter,
    chosen: &[ObjectId],
) {
    let Some(&new_host) = chosen.first() else {
        return;
    };
    if !subjects::on_battlefield(state, from)
        || new_host == from.object
        || state
            .object(new_host)
            .is_none_or(|o| o.zone != Zone::Battlefield || o.status.contains(Status::PHASED_OUT))
    {
        return;
    }
    let you = res.controller;
    let to_move: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            state.object(id).is_some_and(|o| {
                o.attached_to == Some(from.object)
                    && super::matches_in_resolution(attachments, state, o, you, res, Some(you))
            })
        })
        .collect();
    for att in to_move {
        if can_attach_object(state, att, new_host) {
            state.attach(att, new_host);
        }
    }
}

/// Execute [`Effect::AttachLtbAttachments`] (Rhuk, Hexgold Nabber): attach every
/// permanent matching `filter` that is (or last was, via `ltb_attachments`)
/// attached to `of` onto `to`, optionally asking first.
pub(super) fn attach_ltb_attachments(
    state: &mut GameState,
    res: &mut Resolution,
    of: TargetSpec,
    filter: &'static baylee_cards_dsl::Filter,
    to: TargetSpec,
    optional: bool,
) -> Option<super::Pending> {
    let host = super::wave15b::resolve_spec_host(state, res, to)?;
    let of_id = match of {
        TargetSpec::ThisObject => Some(res.source),
        TargetSpec::EventObject => res.event_object,
        other => zones::spec_object(state, res, other),
    }?;
    let of_still_on_battlefield = match of {
        TargetSpec::ThisObject => {
            subjects::source(state, res).is_some_and(|r| subjects::on_battlefield(state, r))
        }
        TargetSpec::EventObject => {
            subjects::event(state, res).is_some_and(|r| subjects::on_battlefield(state, r))
        }
        _ => state
            .object(of_id)
            .is_some_and(|o| o.zone == Zone::Battlefield && !o.status.contains(Status::PHASED_OUT)),
    };
    let candidates: Vec<ObjectId> = if of_still_on_battlefield {
        state
            .battlefield_seen()
            .filter(|&id| {
                state
                    .object(id)
                    .is_some_and(|o| o.attached_to == Some(of_id))
            })
            .collect()
    } else {
        state
            .ltb_attachments
            .iter()
            .rfind(|(h, _)| *h == of_id)
            .map_or_else(Vec::new, |(_, worn)| worn.clone())
    };
    let you = res.controller;
    let objects: Vec<DamageSourceRef> = candidates
        .into_iter()
        .filter_map(|id| {
            let att_ref = state.source_identity(id)?;
            let obj = state.object(id)?;
            (can_attach_to_recipient(state, att_ref, host)
                && super::matches_in_resolution(filter, state, obj, you, res, Some(you)))
            .then_some(att_ref)
        })
        .collect();
    if objects.is_empty() {
        return None;
    }
    if optional {
        if state.has_left(you) {
            return None;
        }
        let source = super::resolving_ability(state, res);
        res.awaiting = Some(super::AwaitingOp::AttachSlot { objects, host });
        return Some(super::Pending::YesNo {
            player: you,
            prompt: crate::choice::YesNoPrompt::MayDo,
            source,
        });
    }
    attach_selected(state, &objects, host);
    None
}

/// Start [`Effect::AttachChosenThen`] (Unexpected Request, Magitek Scythe):
/// choose up to `max` matching permanents to attach to `to`; splice
/// `then_if_attached` only if at least one actually changed host (CR 701.3b).
pub(super) fn attach_chosen_then(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static baylee_cards_dsl::Filter,
    to: TargetSpec,
    max: baylee_cards_dsl::Amount,
    optional: bool,
    then_if_attached: &'static [Effect],
) -> Option<super::Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let host = super::wave15b::resolve_spec_host(state, res, to)?;
    let max_n = super::amount2(&max, state, you, res);
    if max_n == 0 {
        return None;
    }
    let options: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            let Some(att_ref) = state.source_identity(id) else {
                return false;
            };
            can_attach_to_recipient(state, att_ref, host)
                && state.object(id).is_some_and(|obj| {
                    super::matches_in_resolution(filter, state, obj, you, res, Some(you))
                })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    let avail = u8::try_from(options.len()).unwrap_or(u8::MAX);
    let max_clamped = u8::try_from(max_n).unwrap_or(u8::MAX).min(avail);
    let min = if optional { 0 } else { max_clamped };
    res.awaiting = Some(super::AwaitingOp::AttachChosenThen {
        host,
        then_if_attached,
    });
    Some(super::Pending::ChooseCards {
        player: you,
        options,
        min,
        max: max_clamped,
        prompt: crate::choice::ChoicePrompt::Generic,
        total: None,
    })
}

/// Resume [`Effect::AttachChosenThen`]: attach each selected object to `host`,
/// record actually-moved attachments in `res.event_object` / `res.subject.found`,
/// and splice `then_if_attached` only if at least one changed host (CR 701.3b).
pub(super) fn resume_attach_chosen_then(
    state: &mut GameState,
    res: &mut Resolution,
    host: TargetRef,
    then_if_attached: &'static [Effect],
    chosen: &[ObjectId],
) {
    let mut moved: Vec<DamageSourceRef> = Vec::new();
    for &id in chosen {
        let Some(att_ref) = state.source_identity(id) else {
            continue;
        };
        if !can_attach_to_recipient(state, att_ref, host) {
            continue;
        }
        let already_there = state.object(id).is_some_and(|o| match host {
            TargetRef::Object(h) => o.attached_to == Some(h.object),
            TargetRef::Player(p) => o.attached_player() == Some(p),
        });
        if already_there {
            continue;
        }
        match host {
            TargetRef::Object(target) => state.attach(id, target.object),
            TargetRef::Player(player) => state.attach_to_player(id, player),
        }
        let now_there = state.object(id).is_some_and(|o| match host {
            TargetRef::Object(h) => o.attached_to == Some(h.object),
            TargetRef::Player(p) => o.attached_player() == Some(p),
        });
        if now_there {
            moved.push(att_ref);
        }
    }
    if !moved.is_empty() {
        res.event_object = Some(moved[0].object);
        res.subject.found = moved.into_iter().collect();
        if !then_if_attached.is_empty() {
            let next = res.pc + 1;
            res.effects
                .splice(next..next, super::flatten(then_if_attached));
        }
    }
}

/// Start [`Effect::RelocateAttachment`] (Steam Vines, Kudzu, Nettlevine Blight,
/// Aura Graft): resolve `attachment` (including `AttachmentSource::GrantedSource`),
/// prompt `chooser` for a legal recipient matching `recipient_filter`, and
/// splice `then_if_moved` only when the attachment actually changes host.
#[allow(clippy::too_many_arguments)]
pub(super) fn relocate_attachment(
    state: &mut GameState,
    res: &mut Resolution,
    attachment: baylee_cards_dsl::AttachmentSource,
    chooser: baylee_cards_dsl::PlayerRel,
    recipient_filter: &'static baylee_cards_dsl::Filter,
    must_change_recipient: bool,
    optional: bool,
    then_if_moved: &'static [Effect],
) -> Option<super::Pending> {
    let att_ref = match attachment {
        baylee_cards_dsl::AttachmentSource::GrantedSource => {
            super::granted_source(state, res.on_stack)
                .filter(|r| subjects::on_battlefield(state, *r))?
        }
        baylee_cards_dsl::AttachmentSource::Spec(spec) => {
            resolve_spec_attachments(state, res, spec)
                .into_iter()
                .next()?
        }
    };
    let chooser_player = super::players_of(chooser, state, res.controller, res)
        .into_iter()
        .find(|&p| !state.has_left(p))?;
    let (current_host, is_aura) = {
        let obj = state.object(att_ref.object)?;
        (
            obj.attached_to,
            obj.characteristics()
                .subtypes
                .contains(subtypes::enchantment::AURA),
        )
    };
    let ctx = crate::text_changes::RuleContext {
        reference: current_host,
        ..res.rule_context()
    };
    let you = res.controller;
    let options: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            if must_change_recipient && current_host == Some(id) {
                return false;
            }
            can_attach_object(state, att_ref.object, id)
                && state.object(id).is_some_and(|host_obj| {
                    crate::eval::matches_with_context(recipient_filter, state, host_obj, you, ctx)
                })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    let prompt = if is_aura {
        crate::choice::ChoicePrompt::NewHost {
            aura: att_ref.object,
        }
    } else {
        crate::choice::ChoicePrompt::Generic
    };
    res.awaiting = Some(super::AwaitingOp::RelocateAttachment {
        attachment: att_ref,
        then_if_moved,
    });
    Some(super::Pending::ChooseCards {
        player: chooser_player,
        options,
        min: u8::from(!optional),
        max: 1,
        prompt,
        total: None,
    })
}

/// Resume [`Effect::RelocateAttachment`]: move `attachment` to `chosen[0]` if
/// legal and different from its current host (CR 701.3b), then splice `then_if_moved`.
pub(super) fn resume_relocate_attachment(
    state: &mut GameState,
    res: &mut Resolution,
    attachment: DamageSourceRef,
    then_if_moved: &'static [Effect],
    chosen: &[ObjectId],
) {
    let Some(&host) = chosen.first() else {
        return;
    };
    if !subjects::on_battlefield(state, attachment)
        || state
            .object(host)
            .is_none_or(|o| o.zone != Zone::Battlefield || o.status.contains(Status::PHASED_OUT))
        || !can_attach_object(state, attachment.object, host)
    {
        return;
    }
    let prev_host = state.object(attachment.object).and_then(|o| o.attached_to);
    if prev_host == Some(host) {
        return;
    }
    state.attach(attachment.object, host);
    if state.object(attachment.object).and_then(|o| o.attached_to) == Some(host)
        && !then_if_moved.is_empty()
    {
        let next = res.pc + 1;
        res.effects
            .splice(next..next, super::flatten(then_if_moved));
    }
}
