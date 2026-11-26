//! Commit entries and simultaneous sacrifice payments.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn sacrifice(state: &mut GameState, chosen: &[ObjectId]) {
    let since = state.journal.last_seq();
    let departures: Vec<_> = chosen
        .iter()
        .map(|id| state.departure_snapshot(*id))
        .collect();
    for (&id, departure) in chosen.iter().zip(departures) {
        let Some(_owner) = state
            .object(id)
            .filter(|o| o.zone == Zone::Battlefield)
            .map(|o| o.owner)
        else {
            continue;
        };
        let _ = state.sacrifice_object_with_departure(id, Cause::Effect, departure);
    }
    crate::graveyard_order::capture(state, since);
}

#[allow(
    clippy::too_many_lines,
    reason = "commits permanent entry, counters, and triggers atomically"
)]
pub(super) fn finish(state: &mut GameState, entry: Attempt) {
    state.entries.pending_objects.remove(&entry.object);
    let Some(object) = state.object(entry.object) else {
        return;
    };
    if object.version != entry.version {
        return;
    }
    let owner = object.owner;
    if let Some(dest) = entry.instead_destination {
        state.restore_tapped(entry.object, false);
        state
            .effects
            .remove_where(|effect| entry.preview_effects.contains(&effect.id));
        state
            .copy_snapshots
            .retain(|(id, _)| !entry.preview_effects.contains(id));
        let already_exiled = state
            .object(entry.object)
            .is_some_and(|o| o.zone == Zone::Exile);
        let is_card = state
            .object(entry.object)
            .is_some_and(crate::object::GameObject::is_card);
        if let Some(object) = state.object_mut(entry.object) {
            if is_card {
                object.kind = crate::object::ObjectKind::Card;
            }
            if let Some(base) = object.original_base.take() {
                object.base = base;
            }
            object.drop_own_abilities();
            object.own_abilities_until_eot = false;
        }
        if already_exiled && dest == baylee_cards_dsl::EntryInsteadZone::Exile {
            return;
        }
        let alt = match dest {
            baylee_cards_dsl::EntryInsteadZone::Exile => ZoneLocation::Exile(owner),
            baylee_cards_dsl::EntryInsteadZone::ShuffleIntoOwnersLibrary => {
                ZoneLocation::Library(owner)
            }
        };
        let _ = state.move_object(entry.object, alt, ZonePosition::Top, entry.cause);
        if dest == baylee_cards_dsl::EntryInsteadZone::ShuffleIntoOwnersLibrary {
            state.shuffle_library(owner);
        }
        return;
    }
    if entry.sacrifices_after > 0 {
        state.entries.after.push_back(AfterEntry {
            player: entry.putter,
            remaining: entry.sacrifices_after,
        });
    }
    let destination = if entry.refusal.is_some() {
        ZoneLocation::Graveyard(owner)
    } else {
        ZoneLocation::Battlefield
    };
    if entry.refusal.is_some() && clean_refused_entry(state, &entry) {
        return;
    }
    if entry.copy == CopyStatus::Handled && entry.refusal.is_none() {
        state
            .object_mut(entry.object)
            .expect("current entry")
            .riders
            .push(crate::object::Rider::EntryCopyApplied);
    }
    if entry.refusal.is_none()
        && let Some(number) = entry.chosen_number
    {
        let acquisition = state
            .object(entry.object)
            .map_or(0, crate::object::GameObject::rules_acquisition);
        super::set_chosen_number(state, entry.object, number, acquisition);
    }
    if entry.refusal.is_none() {
        if let Some(mode) = entry.chosen_mode {
            state
                .object_mut(entry.object)
                .expect("current entry")
                .riders
                .push(crate::object::Rider::ChosenMode(mode));
        }
        for &(chooser, mode) in &entry.player_chosen_modes {
            state
                .object_mut(entry.object)
                .expect("current entry")
                .riders
                .push(crate::object::Rider::PlayerChosenMode(chooser, mode));
        }
    }
    if entry.refusal.is_none() {
        if let Some(ctrl) = entry.controller_override
            && let Some(obj) = state.object_mut(entry.object)
        {
            obj.set_controller(ctrl);
        }
        if !entry.entry_type_pt.is_empty()
            && let Some(obj) = state.object_mut(entry.object)
        {
            if obj.original_base.is_none() {
                obj.original_base = Some(obj.base.clone());
            }
            let mut chars = (*obj.base).clone();
            for &(add_types, add_subtypes, power, toughness) in &entry.entry_type_pt {
                chars.types = chars.types.union(add_types);
                for &st in add_subtypes {
                    chars.subtypes.insert(st);
                }
                chars.power = Some(power);
                chars.toughness = Some(toughness);
            }
            obj.base = std::sync::Arc::new(chars);
            if !obj.riders.contains(&crate::object::Rider::EntryCopyApplied) {
                obj.riders.push(crate::object::Rider::EntryCopyApplied);
            }
        }
        state.restore_tapped(entry.object, entry.tapped);
        if entry.kw_choices.riot_haste {
            state
                .object_mut(entry.object)
                .expect("current entry")
                .riders
                .push(crate::object::Rider::RiotHaste);
        }
    }
    state.entries.committing = Some(entry.object);
    let _ = state.move_object(entry.object, destination, entry.position, entry.cause);
    state.entries.committing = None;
    if state
        .object(entry.object)
        .is_some_and(|o| o.zone == Zone::Battlefield)
    {
        if entry.zone_play_rider != baylee_cards_dsl::ZonePlayCastRider::None
            && let Some(object) = state.object_mut(entry.object)
        {
            super::attach_zone_play_rider(object, entry.zone_play_rider);
        }
        if let Some(player) = entry.battle_protector
            && let Some(reference) = state.source_identity(entry.object)
        {
            crate::battle::set_protector(state, reference, player);
        }
        state
            .object_mut(entry.object)
            .expect("successful entry")
            .riders
            .push(crate::object::Rider::EntryTappingApplied);
        if let Some(count) = entry.sacrifices {
            state
                .object_mut(entry.object)
                .expect("entered object")
                .riders
                .push(crate::object::Rider::EntrySacrifices(count));
            state.invalidate_projections();
        }
        let designated = apply_entry_designations(state, &entry);
        let riot_haste = entry.kw_choices.riot_haste;
        if entry.tribute == super::TributeState::Paid {
            state
                .object_mut(entry.object)
                .expect("entered object")
                .riders
                .push(crate::object::Rider::TributePaid);
            state.invalidate_projections();
        }
        state
            .object_mut(entry.object)
            .expect("successful entry")
            .riders
            .retain(|rider| !matches!(rider, crate::object::Rider::EntryP1P1(_)));
        let was_spell = entry.cause == Cause::Spell;
        let object_id = entry.object;
        super::counters::commit(state, entry);
        if designated || riot_haste {
            state.invalidate_projections();
            state.refresh_characteristics();
        }
        if was_spell {
            crate::resolve::finish_sneak_enter_attacking(state, object_id);
        }
    }
}

fn clean_refused_entry(state: &mut GameState, entry: &Attempt) -> bool {
    state.restore_tapped(entry.object, false);
    state
        .effects
        .remove_where(|effect| entry.preview_effects.contains(&effect.id));
    state
        .copy_snapshots
        .retain(|(id, _)| !entry.preview_effects.contains(id));
    if let Some(object) = state.object_mut(entry.object) {
        object.kind = crate::object::ObjectKind::Card;
        if let Some(base) = object.original_base.take() {
            object.base = base;
        }
        object.drop_own_abilities();
        object.own_abilities_until_eot = false;
        if object.zone == Zone::Graveyard
            || (entry.refusal == Some(Refusal::StayInZone) && object.zone != Zone::Stack)
        {
            return true;
        }
    }
    false
}

fn apply_entry_designations(state: &mut GameState, entry: &Attempt) -> bool {
    let mut designated = false;
    if entry.modifiers.contains(&EnterModifier::Suspected)
        && state.object(entry.object).is_some_and(|o| {
            o.characteristics()
                .types
                .contains(baylee_core::types::TypeSet::CREATURE)
                && !o.riders.contains(&crate::object::Rider::Suspected)
        })
    {
        state
            .object_mut(entry.object)
            .expect("entered object")
            .riders
            .push(crate::object::Rider::Suspected);
        designated = true;
    }
    if let Some(sub) = entry.chosen_subtype {
        state
            .object_mut(entry.object)
            .expect("entered object")
            .chosen_subtype = Some(sub);
        designated = true;
    }
    if let Some(chosen) = entry.chosen_player {
        state
            .object_mut(entry.object)
            .expect("entered object")
            .set_chosen_opponent(Some(chosen));
        designated = true;
    }
    if let Some(second) = entry.second_chosen_player {
        state
            .object_mut(entry.object)
            .expect("entered object")
            .set_second_chosen_player(Some(second));
        designated = true;
    }
    if let Some(secret) = entry.secret_chosen_player {
        state
            .object_mut(entry.object)
            .expect("entered object")
            .set_secret_chosen_player(secret);
        designated = true;
    }
    if let Some(number) = entry.secret_chosen_number {
        state
            .object_mut(entry.object)
            .expect("entered object")
            .set_secret_chosen_number(number);
        designated = true;
    }
    designated
}
