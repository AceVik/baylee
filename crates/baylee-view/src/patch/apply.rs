//! A patch applied to the view it was made for.

use std::collections::BTreeMap;

use super::{Keyed, PatchError, ViewPatch, Zone, ZoneEdit, digest};
use crate::objects::PublicObject;
use crate::player_view::PlayerView;
use baylee_core::ids::ObjectId;

/// The view `patch` leaves when applied to `old`.
///
/// The result is checked against the patch's digest, so an `Ok` is the view the
/// patch was made from. `old` is left as it was on any error, which is what
/// lets a client go on drawing it while it asks for a snapshot.
///
/// # Errors
/// [`PatchError`]: the patch is not for `old`, names what `old` does not hold,
/// or does not leave the view it says it does.
pub fn apply(old: &PlayerView, patch: &ViewPatch) -> Result<PlayerView, PatchError> {
    if old.seat != patch.seat {
        return Err(PatchError::OtherSeat(patch.seat));
    }
    if old.seq != patch.base {
        return Err(PatchError::Gap {
            have: old.seq,
            base: patch.base,
        });
    }
    let mut view = old.clone();
    view.seq = patch.seq;
    for field in &patch.fields {
        field.set(&mut view)?;
    }
    for (zone, edit) in &patch.zones {
        let objects = view.zone_mut(*zone).ok_or(PatchError::NoZone(*zone))?;
        apply_edit(objects, edit)?;
    }
    if let Some(edit) = &patch.hand {
        apply_edit(&mut view.hand, edit)?;
    }
    if !patch.seats.is_empty() {
        let at: BTreeMap<_, _> = view
            .seats
            .iter()
            .enumerate()
            .map(|(place, seat)| (seat.player, place))
            .collect();
        for line in &patch.seats {
            let place = at
                .get(&line.player)
                .ok_or(PatchError::NoSeat(line.player))?;
            if let Some(seat) = view.seats.get_mut(*place) {
                seat.clone_from(line);
            }
        }
    }
    let found = digest(&view);
    if found != patch.digest {
        return Err(PatchError::Digest {
            expected: patch.digest,
            found,
        });
    }
    Ok(view)
}

impl PlayerView {
    /// The zone a patch names; `None` when this view has no such seat.
    pub(super) fn zone_mut(&mut self, zone: Zone) -> Option<&mut Vec<PublicObject>> {
        match zone {
            Zone::Battlefield => Some(&mut self.battlefield),
            Zone::Stack => Some(&mut self.stack),
            Zone::Graveyard(seat) => self.graveyards.get_mut(usize::from(seat)),
            Zone::Exile(seat) => self.exile.get_mut(usize::from(seat)),
            Zone::Command(seat) => self.command.get_mut(usize::from(seat)),
            Zone::LookingAt => Some(&mut self.looking_at),
            Zone::LibraryTops => Some(&mut self.library_tops),
        }
    }
}

/// Where each object of `zone` is, refusing a zone that holds an id twice.
fn places<T: Keyed>(zone: &[T]) -> Result<BTreeMap<ObjectId, usize>, PatchError> {
    let mut at = BTreeMap::new();
    for (place, object) in zone.iter().enumerate() {
        if at.insert(object.key(), place).is_some() {
            return Err(PatchError::Unkeyed(object.key()));
        }
    }
    Ok(at)
}

/// Applies one zone's edit, as [`ZoneEdit`] reads.
fn apply_edit<T: Keyed + Clone>(zone: &mut Vec<T>, edit: &ZoneEdit<T>) -> Result<(), PatchError> {
    let mut at = places(zone)?;
    let mut kept = vec![true; zone.len()];
    for id in &edit.removed {
        let place = at.remove(id).ok_or(PatchError::UnknownObject(*id))?;
        if let Some(keep) = kept.get_mut(place) {
            *keep = false;
        }
    }
    for object in &edit.upserts {
        if let Some(slot) = at.get(&object.key()).and_then(|&place| zone.get_mut(place)) {
            slot.clone_from(object);
        } else {
            at.insert(object.key(), zone.len());
            zone.push(object.clone());
            kept.push(true);
        }
    }
    let mut keep = kept.iter();
    zone.retain(|_| keep.next().copied().unwrap_or(true));
    if let Some(order) = &edit.order {
        reorder(zone, order)?;
    }
    Ok(())
}

/// Puts `zone` in `order`, which has to name each of its objects once.
fn reorder<T: Keyed>(zone: &mut Vec<T>, order: &[ObjectId]) -> Result<(), PatchError> {
    if order.len() != zone.len() {
        return Err(PatchError::BadOrder);
    }
    let mut left: BTreeMap<ObjectId, T> = zone.drain(..).map(|o| (o.key(), o)).collect();
    let mut sorted = Vec::with_capacity(left.len());
    for id in order {
        sorted.push(left.remove(id).ok_or(PatchError::UnknownObject(*id))?);
    }
    *zone = sorted;
    Ok(())
}
