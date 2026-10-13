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
    if copy.target_req.is_none()
        && loc.index != baylee_core::ids::AbilityRef::SYNTHETIC
        && crate::choice::granted_slot(loc.index).is_none()
    {
        copy.target_req = copy
            .own_abilities
            .as_ref()
            .and_then(|list| crate::object::ability_target_req(list, loc.index, copy.mode_index));
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
    let target_players = from.target_players;
    let chosen_player = from.chosen_player;
    let x_value = from.x_value;
    let kicked = from.kicked;
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
        tokens::apply_copy_mod_with_text(&mut base, m, text);
    }
    let ts = state.next_timestamp();
    let prepare_spell = state.source_object(original).and_then(|object| {
        object
            .riders
            .iter()
            .find(|rider| matches!(rider, crate::object::Rider::PrepareSpellFace(_)))
            .copied()
    });
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
        obj.target_players = target_players;
        obj.chosen_player = chosen_player;
        obj.x_value = x_value;
        obj.kicked = kicked;
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
        if let Some(rider) = prepare_spell {
            obj.riders.push(rider);
        }
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
