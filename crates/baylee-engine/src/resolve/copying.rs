//! Copying exact spell and ability incarnations and their announced decisions.

use super::{
    GameObject, GameState, ObjectId, ObjectKind, Pending, PlayerId, Resolution, ZoneLocation,
    ZonePosition, retarget, tokens,
};

/// "Copy target activated or triggered ability you control. You may choose
/// new targets for the copy." (CR 707.10, 707.10c.)
///
/// The copy is the original object cloned, which is what "copies both the
/// characteristics of the spell or ability and all decisions made for it"
/// asks: its mode, targets, X, chosen player, the list of abilities it
/// resolves from, its event object and what paid its costs all come along,
/// and so does its source (CR 707.10b). It is put on the stack under `you`,
/// newly timestamped, and journals no `AbilityTriggered`: a copy is neither
/// activated nor triggered (CR 707.10), and `record_new_targets` journals
/// what it targets once its targets are settled, so "becomes the target"
/// sees it exactly once.
///
/// Its requirement is written onto the copy (`ability_target_req`), because
/// an ability pushed from its definition carries none and the question
/// about new targets is asked against it.
pub(super) fn copy_target_ability(
    state: &mut GameState,
    res: &mut Resolution,
    you: PlayerId,
) -> Option<Pending> {
    let &original = res.targets.first()?;
    let identity = state
        .object(original)
        .filter(|o| o.zone == crate::zone::Zone::Stack && o.kind == ObjectKind::AbilityOnStack)
        .and_then(|_| state.source_identity(original))?;
    let copy = copy_ability(state, identity, you)?;
    retarget::start_copy(state, res, copy)
}

/// Exact ability snapshot shared by targeted copies and delayed mana copies.
fn copy_ability(
    state: &mut GameState,
    original: baylee_core::ids::DamageSourceRef,
    you: PlayerId,
) -> Option<ObjectId> {
    state.capture_source_references();
    let mut copy = state
        .source_object(original)
        .filter(|object| object.kind == ObjectKind::AbilityOnStack)?
        .clone();
    let loc = copy.ability?;
    if !copy.riders.contains(&crate::object::Rider::AbilityCopy) {
        copy.riders.push(crate::object::Rider::AbilityCopy);
    }
    if copy.target_req.is_none()
        && loc.index != baylee_core::ids::AbilityRef::SYNTHETIC
        && crate::choice::granted_slot(loc.index).is_none()
    {
        copy.target_req = copy
            .own_abilities
            .as_ref()
            .and_then(|list| crate::object::ability_target_req(list, loc.index, copy.mode_index));
    }
    if copy.second_target_req().is_none()
        && loc.index != baylee_core::ids::AbilityRef::SYNTHETIC
        && crate::choice::granted_slot(loc.index).is_none()
        && let Some(second_req) = copy.own_abilities.as_ref().and_then(|list| {
            crate::object::ability_second_target_req(list, loc.index, copy.mode_index)
        })
    {
        let existing = smallvec::SmallVec::from_slice(copy.second_targets());
        copy.set_second(existing, Some(second_req));
    }
    let timestamp = state.next_timestamp();
    let id = state.arena.insert_with(|id| {
        copy.id = id;
        copy.owner = you;
        copy.controller = you;
        copy.base_controller = you;
        copy.timestamp = timestamp;
        copy.controlled_since = timestamp;
        copy.cache = crate::object::CachedChar::default();
        copy
    });
    state
        .zones
        .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
    if loc.index == baylee_core::ids::AbilityRef::SYNTHETIC
        || crate::choice::granted_slot(loc.index).is_some()
    {
        state.synthetic_copies.push((original.object, id));
    }
    state.copy_source_references(original, id);
    copy_division(state, original, id);
    Some(id)
}

pub(super) fn copy_event(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    let original = super::subjects::event(state, res)?;
    let copy = if state.source_object(original)?.kind == ObjectKind::AbilityOnStack {
        copy_ability(state, original, res.controller)?
    } else {
        copy_spell(state, original, res.controller, &[], res.text)?
    };
    retarget::start_copy(state, res, copy)
}

pub(super) fn targets_copy_event(state: &mut GameState, res: &Resolution) {
    let Some(original) = super::subjects::event(state, res) else {
        return;
    };
    for &target in &res.targets {
        if !super::subjects::target(state, res, target)
            .is_some_and(|target| super::subjects::on_battlefield(state, target))
        {
            continue;
        }
        crate::copiable_abilities::apply_copy_from_reference(
            state,
            crate::copiable_abilities::CopyApplication {
                source: target,
                target: original.object,
                mods: &[],
                copy_index: 0,
                until_eot: true,
                resolving: None,
                text: res.text,
            },
            original,
        );
    }
}

/// Damage distribution is an announcement decision, so it belongs to the
/// copy too. Retargeting later moves each share with its corresponding slot.
fn copy_division(
    state: &mut GameState,
    original: baylee_core::ids::DamageSourceRef,
    copy: ObjectId,
) {
    if let Some(shares) = state.source_memory.divisions.get(&original) {
        state.divided.push((copy, shares.clone()));
    }
    if let Some(copy_ref) = state.source_identity(copy) {
        let copied_groups: Vec<((u8, u32), PlayerId)> = state
            .source_memory
            .target_groups
            .iter()
            .filter_map(|(&(h, group, slot), &player)| {
                (h == original).then_some(((group, slot), player))
            })
            .collect();
        for ((group, slot), player) in copied_groups {
            state
                .source_memory
                .target_groups
                .insert((copy_ref, group, slot), player);
        }
    }
}

/// Puts a copy of the spell `original` onto the stack under `you`'s control
/// and returns it: the one constructor of a spell copy, for "copy target
/// spell" and for the copies a replicate trigger makes.
///
/// A copy copies the spell's characteristics, as `mods` change them, and
/// every decision made for it (CR 707.10): its targets in both instances of
/// the word and the requirement they answer (which a later change of targets
/// reads), the players it targets, its mode or set of modes (CR 700.2g),
/// X, the face it was cast as,
/// whether it was kicked and how many times replicate was paid. Nothing that
/// happened to the *card* comes with it: no rider it was cast with, no mana
/// spent (a copy is not cast), and no
/// `SpellCast` event (CR 707.10). Journalling one made every copy re-trigger
/// "whenever you cast" abilities — Jin-Gitaxias copied its own copy without
/// end — and made copies count towards Storm of Saruman's "second spell each
/// turn".
///
/// The original need not still be on the stack. Its exact spell incarnation
/// is archived before it leaves (`GameState::move_object`), so a trigger whose
/// spell was countered in response copies it as it last existed (CR 608.2h,
/// 113.7a), even if the card has since been cast again. Until this was one
/// function the X, the mode, the face, the
/// kicker and the player targets were left behind: a copied Fireball was
/// cast for X = 0 and a copied Lightning Bolt aimed at a player had no
/// target at all.
fn copied_spell_riders(riders: &[crate::object::Rider]) -> Vec<crate::object::Rider> {
    use crate::object::Rider as R;
    riders
        .iter()
        .copied()
        .filter(|rider| {
            matches!(
                rider,
                R::KickedMask(_)
                    | R::Buyback
                    | R::Bargained
                    | R::SquadPaid(_)
                    | R::OffspringPaid
                    | R::CasualtyPaid
                    | R::ImpendingPaid
                    | R::GiftPromisedTo(_)
                    | R::Fused
                    | R::PrepareSpellFace(_)
                    | R::WarpedSpell
                    | R::Spliced { .. }
                    | R::Awakened(_)
                    | R::RepeatedModes(_)
                    | R::OptionalCostPaid
                    | R::ControlledSubtypeOnCast(_)
                    | R::BeheldCostPower(_, _, _)
                    | R::MayhemPaid
            )
        })
        .collect()
}

pub(super) fn copy_spell(
    state: &mut GameState,
    original: baylee_core::ids::DamageSourceRef,
    you: PlayerId,
    mods: &[baylee_cards_dsl::CopyMod],
    text: crate::text_changes::TextChangeMap,
) -> Option<ObjectId> {
    state.capture_source_references();
    let from = state.source_object(original)?;
    let mut base = (*from.base).clone();
    let card = from.card;
    let targets = from.targets.clone();
    let target_req = from.target_req;
    let second = from.second.clone();
    let extra_targets = from.extra_target_instances().to_vec();
    let target_players = from.target_players;
    let chosen_player = from.chosen_player;
    let x_value = from.x_value;
    let times_kicked = from.times_kicked;
    let copied_riders = copied_spell_riders(&from.riders);
    let replicated = from.replicated;
    let mode_index = from.mode_index;
    let modes = from.modes;
    let face_index = from.face_index;
    let paid = from.paid.as_ref().map(|paid| {
        Box::new(crate::object::PaidRecord {
            sacrificed_lki: paid.sacrificed_lki,
            sacrificed: paid.sacrificed,
            sacrificed_many: paid.sacrificed_many.clone(),
            source_after_cost: paid.source_after_cost,
            tapped: paid.tapped,
            ..crate::object::PaidRecord::default()
        })
    });
    for m in mods {
        tokens::apply_copy_mod_with_names(&mut base, m, text, Some(&mut state.names));
    }
    let ts = state.next_timestamp();
    let id = state.arena.insert_with(|oid| {
        let mut obj = GameObject::new_bare(oid, you, ObjectKind::Spell, base);
        obj.timestamp = ts;
        obj.controlled_since = ts;
        obj
    });
    {
        let obj = state.object_mut(id).expect("fresh copy");
        obj.card = card;
        // CR 707.10: a copy is put on the stack, not cast from hand.
        // Rebound must not schedule a vanished copy.
        obj.cast_history.set_from_hand(false);
        obj.cast_history.set_caster(None);
        obj.targets = targets;
        obj.target_req = target_req;
        obj.second = second;
        if !extra_targets.is_empty() {
            obj.set_extra_targets(extra_targets);
        }
        obj.target_players = target_players;
        obj.chosen_player = chosen_player;
        obj.x_value = x_value;
        obj.times_kicked = times_kicked;
        obj.replicated = replicated;
        obj.mode_index = mode_index;
        obj.modes = modes;
        obj.face_index = face_index;
        obj.paid = paid;
        obj.zone = crate::zone::Zone::Stack;
        // CR 704.5e: it stops existing the moment it is anywhere but the
        // stack or the battlefield. Carrying the copied card is what makes
        // the marker necessary — without it the copy resolved into a
        // graveyard and stayed there as a second, real card.
        obj.riders.push(crate::object::Rider::SpellCopy);
        obj.riders.extend(copied_riders);
    }
    state.put_new_spell_on_stack(id);
    state.copy_source_references(original, id);
    copy_division(state, original, id);
    state.journal.record(crate::event::GameEvent::SpellCopied {
        object: id,
        player: you,
    });
    Some(id)
}

/// Copy the currently resolving spell under the named participant's control.
/// Retargeting belongs to the copy's controller; the original resolution
/// retains its controller for all later instructions.
pub(super) fn copy_resolving(
    state: &mut GameState,
    res: &mut Resolution,
    controller: baylee_cards_dsl::PlayerRel,
) -> Option<Pending> {
    let player = super::players_of(controller, state, res.controller, res)
        .into_iter()
        .next()?;
    if state.has_left(player) {
        return None;
    }
    let original = super::subjects::source(state, res)?;
    if state.source_object(original)?.kind != ObjectKind::Spell {
        return None;
    }
    let copy = copy_spell(state, original, player, &[], res.text)?;
    retarget::start_copy(state, res, copy)
}

/// Actual cast provenance belongs to this exact spell, never to its copies.
pub(super) fn was_cast_from_graveyard(state: &GameState, res: &Resolution) -> bool {
    super::subjects::source(state, res)
        .and_then(|source| state.source_object(source))
        .is_some_and(|object| {
            object.kind == ObjectKind::Spell
                && object.cast_history.was_from_graveyard()
                && !object.riders.contains(&crate::object::Rider::SpellCopy)
        })
}

/// Parameters for `Effect::BecomeCopy`.
#[derive(Clone, Copy)]
pub(super) struct BecomeCopySpec {
    pub subject: baylee_cards_dsl::TargetSlot,
    pub subject_filter: Option<&'static baylee_cards_dsl::Filter>,
    pub original: baylee_cards_dsl::TargetSpec,
    pub original_is_second_target: bool,
    pub until_eot: bool,
    pub mods: &'static [baylee_cards_dsl::CopyMod],
}

/// Generalized `Effect::BecomeCopy` (CR 707.2, CR 707.9).
pub(super) fn become_copy(state: &mut GameState, res: &Resolution, spec: BecomeCopySpec) {
    let Some(original_ref) =
        resolve_copy_original(state, res, spec.original, spec.original_is_second_target)
    else {
        return;
    };
    let subjects = resolve_copy_subjects(
        state,
        res,
        spec.subject,
        spec.subject_filter,
        original_ref.object,
    );
    if subjects.is_empty() {
        return;
    }
    let loc = state.object(res.on_stack).and_then(|object| object.ability);
    let resolving = loc.and_then(|loc| {
        state
            .printed_ability_list(res.on_stack)?
            .entry(loc.index as usize)
    });
    let copy_index = loc.map_or(0, |loc| loc.index);
    for target_subject in subjects {
        crate::copiable_abilities::apply_copy_from_reference(
            state,
            crate::copiable_abilities::CopyApplication {
                source: target_subject,
                target: original_ref.object,
                mods: spec.mods,
                copy_index,
                until_eot: spec.until_eot,
                resolving,
                text: res.text,
            },
            original_ref,
        );
    }
}

fn resolve_copy_original(
    state: &GameState,
    res: &Resolution,
    original: baylee_cards_dsl::TargetSpec,
    original_is_second_target: bool,
) -> Option<baylee_core::ids::DamageSourceRef> {
    use baylee_cards_dsl::TargetSpec;
    if original_is_second_target {
        let &id = res.second_targets.first()?;
        return super::subjects::target(state, res, id).or_else(|| state.source_identity(id));
    }
    match original {
        TargetSpec::ThisObject => {
            super::subjects::source(state, res).or_else(|| state.source_identity(res.source))
        }
        TargetSpec::EventObject => super::subjects::event(state, res)
            .or_else(|| res.event_object.and_then(|id| state.source_identity(id))),
        _ => {
            let &id = res.targets.first()?;
            super::subjects::target(state, res, id).or_else(|| state.source_identity(id))
        }
    }
}

fn resolve_copy_subjects(
    state: &GameState,
    res: &Resolution,
    subject: baylee_cards_dsl::TargetSlot,
    subject_filter: Option<&'static baylee_cards_dsl::Filter>,
    original_id: ObjectId,
) -> Vec<ObjectId> {
    use baylee_cards_dsl::TargetSlot;
    if let Some(filter) = subject_filter {
        let context = res.rule_context();
        return state
            .battlefield_seen()
            .filter(|&id| {
                id != original_id
                    && state.object(id).is_some_and(|obj| {
                        crate::eval::matches_with_context(
                            filter,
                            state,
                            obj,
                            res.controller,
                            context,
                        )
                    })
            })
            .collect();
    }
    match subject {
        TargetSlot::This => super::subjects::source(state, res)
            .filter(|s| super::subjects::on_battlefield(state, *s))
            .map(|s| s.object)
            .into_iter()
            .collect(),
        other => res
            .slot_targets(other)
            .iter()
            .copied()
            .filter(|&id| {
                super::subjects::target(state, res, id)
                    .is_some_and(|r| super::subjects::on_battlefield(state, r))
            })
            .collect(),
    }
}

/// Generalized `Effect::CopySpellOrAbility` (CR 707.10, CR 707.10c, CR 707.12).
pub(super) fn copy_spell_or_ability(
    state: &mut GameState,
    res: &mut Resolution,
    target: baylee_cards_dsl::TargetSpec,
    controller: baylee_cards_dsl::PlayerRel,
    amount: baylee_cards_dsl::Amount,
    mods: &'static [baylee_cards_dsl::CopyMod],
    may_choose_new_targets: bool,
) -> Option<Pending> {
    use baylee_cards_dsl::{Amount, Effect, TargetSpec};
    let count = super::amount2(&amount, state, res.controller, res);
    if count == 0 {
        return None;
    }
    let player = super::players_of(controller, state, res.controller, res)
        .into_iter()
        .next()?;
    if state.has_left(player) {
        return None;
    }
    let original = match target {
        TargetSpec::ThisObject => {
            super::subjects::source(state, res).or_else(|| state.source_identity(res.on_stack))?
        }
        TargetSpec::EventObject => super::subjects::event(state, res)?,
        _ => {
            let &id = res.targets.first()?;
            state
                .object(id)
                .filter(|o| o.zone == crate::zone::Zone::Stack)
                .and_then(|_| state.source_identity(id))?
        }
    };
    let is_ability = state.source_object(original)?.kind == ObjectKind::AbilityOnStack;
    if !may_choose_new_targets {
        for _ in 0..count {
            let copy = if is_ability {
                copy_ability(state, original, player)?
            } else {
                copy_spell(state, original, player, mods, res.text)?
            };
            retarget::keep_copy_targets(state, copy);
        }
        return None;
    }
    let copy = if is_ability {
        copy_ability(state, original, player)?
    } else {
        copy_spell(state, original, player, mods, res.text)?
    };
    if count > 1 {
        res.effects.insert(
            res.pc + 1,
            Effect::CopySpellOrAbility {
                target,
                controller,
                amount: Amount::Fixed(count - 1),
                mods,
                may_choose_new_targets: true,
            },
        );
    }
    retarget::start_copy(state, res, copy)
}
