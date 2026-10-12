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

pub(super) fn finish(state: &mut GameState, entry: Attempt) {
    state.entries.pending_objects.remove(&entry.object);
    let Some(object) = state.object(entry.object) else {
        return;
    };
    if object.version != entry.version {
        return;
    }
    let owner = object.owner;
    if entry.sacrifices_after > 0 {
        state.entries.after.push_back(AfterEntry {
            player: entry.putter,
            remaining: entry.sacrifices_after,
        });
    }
    let destination = if entry.denied {
        ZoneLocation::Graveyard(owner)
    } else {
        ZoneLocation::Battlefield
    };
    if entry.denied {
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
            if object.zone == Zone::Graveyard {
                return;
            }
        }
    }
    if entry.copy == CopyStatus::Handled && !entry.denied {
        state
            .object_mut(entry.object)
            .expect("current entry")
            .riders
            .push(crate::object::Rider::EntryCopyApplied);
    }
    if !entry.denied
        && let Some(number) = entry.chosen_number
    {
        let acquisition = state
            .object(entry.object)
            .map_or(0, crate::object::GameObject::rules_acquisition);
        super::set_chosen_number(state, entry.object, number, acquisition);
    }
    if !entry.denied {
        state.restore_tapped(entry.object, entry.tapped);
    }
    state.entries.committing = Some(entry.object);
    let _ = state.move_object(entry.object, destination, entry.position, entry.cause);
    state.entries.committing = None;
    if state
        .object(entry.object)
        .is_some_and(|o| o.zone == Zone::Battlefield)
    {
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
        for (by, kind, count) in entry.counters {
            let by = by.unwrap_or_else(|| {
                state
                    .object(entry.object)
                    .expect("entered object")
                    .controller
            });
            crate::replacement::put_counters_by(state, by, entry.object, kind, count);
        }
    }
}
