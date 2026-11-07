//! How one seat's view differs from its last one.

use std::collections::{BTreeMap, BTreeSet};

use super::fields::{ViewField, changed};
use super::{Keyed, ViewPatch, Zone, ZoneEdit, digest};
use crate::objects::PublicObject;
use crate::player_view::PlayerView;
use crate::seats::SeatView;
use baylee_core::ids::ObjectId;

/// What `new` is, said against `old`: `apply(old, diff(old, new)) == new` for
/// any two views, however unrelated.
///
/// Both must be views of one seat ([`super`], "Hidden information").
#[must_use]
pub fn diff(old: &PlayerView, new: &PlayerView) -> ViewPatch {
    let mut edits = Edits::default();
    changed(old, new, &mut edits.fields);
    edits.zone(Zone::Battlefield, &old.battlefield, &new.battlefield);
    edits.zone(Zone::Stack, &old.stack, &new.stack);
    if !edits.family(Zone::Graveyard, &old.graveyards, &new.graveyards) {
        let whole = ViewField::Graveyards(new.graveyards.clone());
        edits.fields.push(whole);
    }
    if !edits.family(Zone::Exile, &old.exile, &new.exile) {
        edits.fields.push(ViewField::Exile(new.exile.clone()));
    }
    if !edits.family(Zone::Command, &old.command, &new.command) {
        edits.fields.push(ViewField::Command(new.command.clone()));
    }
    edits.zone(Zone::LookingAt, &old.looking_at, &new.looking_at);
    edits.zone(Zone::LibraryTops, &old.library_tops, &new.library_tops);
    let hand = if old.hand == new.hand {
        None
    } else {
        let edit = edit_of(&old.hand, &new.hand);
        if edit.is_none() {
            edits.fields.push(ViewField::Hand(new.hand.clone()));
        }
        edit
    };
    let seats = changed_seats(&old.seats, &new.seats).unwrap_or_else(|| {
        edits.fields.push(ViewField::Seats(new.seats.clone()));
        Vec::new()
    });
    ViewPatch {
        base: old.seq,
        seq: new.seq,
        zones: edits.zones,
        hand,
        seats,
        fields: edits.fields,
        digest: digest(new),
    }
}

/// A patch's zones and fields as they gather.
#[derive(Default)]
struct Edits {
    zones: Vec<(Zone, ZoneEdit<PublicObject>)>,
    fields: Vec<ViewField>,
}

impl Edits {
    /// `new` against `old` as an edit, or whole where ids do not name its
    /// objects.
    fn zone(&mut self, zone: Zone, old: &[PublicObject], new: &[PublicObject]) {
        if old == new {
            return;
        }
        match edit_of(old, new) {
            Some(edit) => self.zones.push((zone, edit)),
            None => self.fields.push(ViewField::Zone(zone, new.to_vec())),
        }
    }

    /// A zone per seat, seat by seat. `false`, and nothing done, when the two
    /// do not have the same seats, or more than a seat's byte can name.
    fn family(
        &mut self,
        zone: fn(u8) -> Zone,
        old: &[Vec<PublicObject>],
        new: &[Vec<PublicObject>],
    ) -> bool {
        if old.len() != new.len() || new.len() > usize::from(u8::MAX) {
            return false;
        }
        for (seat, (old, new)) in (0..=u8::MAX).zip(old.iter().zip(new)) {
            self.zone(zone(seat), old, new);
        }
        true
    }
}

/// Where each object of a zone is; `None` when an id comes up twice.
fn positions<T: Keyed>(zone: &[T]) -> Option<BTreeMap<ObjectId, usize>> {
    let mut at = BTreeMap::new();
    for (place, object) in zone.iter().enumerate() {
        if at.insert(object.key(), place).is_some() {
            return None;
        }
    }
    Some(at)
}

/// The edit that makes `old` into `new`; `None` when either holds an id twice.
fn edit_of<T: Keyed + PartialEq + Clone>(old: &[T], new: &[T]) -> Option<ZoneEdit<T>> {
    let was = positions(old)?;
    let is = positions(new)?;
    let removed = old
        .iter()
        .map(Keyed::key)
        .filter(|id| !is.contains_key(id))
        .collect();
    let upserts = new
        .iter()
        .filter(|object| {
            was.get(&object.key())
                .is_none_or(|&place| old[place] != **object)
        })
        .cloned()
        .collect();
    // What `apply` leaves without being told an order.
    let left = old
        .iter()
        .map(Keyed::key)
        .filter(|id| is.contains_key(id))
        .chain(
            new.iter()
                .map(Keyed::key)
                .filter(|id| !was.contains_key(id)),
        );
    let order =
        (!left.eq(new.iter().map(Keyed::key))).then(|| new.iter().map(Keyed::key).collect());
    Some(ZoneEdit {
        removed,
        upserts,
        order,
    })
}

/// The lines of `new` that differ from `old`'s; `None` unless both have the
/// same seats in the same order, each once, so a line's player names its place.
fn changed_seats(old: &[SeatView], new: &[SeatView]) -> Option<Vec<SeatView>> {
    let mut players = BTreeSet::new();
    let same = old.len() == new.len()
        && old
            .iter()
            .zip(new)
            .all(|(old, new)| old.player == new.player && players.insert(new.player));
    same.then(|| {
        old.iter()
            .zip(new)
            .filter(|(old, new)| old != new)
            .map(|(_, new)| new.clone())
            .collect()
    })
}
