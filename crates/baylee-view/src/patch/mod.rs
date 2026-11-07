//! A view as what changed since the last one (`docs/proposals/view-patches.md`).
//!
//! [`diff`] takes two finished views of one seat and says how the second
//! differs from the first; [`apply`] takes the first and that patch and gives
//! the second back, or says why it cannot. Both are here, once, so a host and
//! everything that reads a host work from the same two functions, and
//! everything above them goes on holding a whole [`PlayerView`].
//!
//! # Hidden information (CR 400.2)
//!
//! A patch is computed from a seat's finished view and from nothing else:
//! [`diff`] takes two views and has no way to a game state. What a
//! [`PlayerView`] cannot represent, a patch cannot carry, so the rule the view
//! is built on needs no second check here. Both views must be the same seat's:
//! a patch between two seats' views would be that seat's cards in the other's
//! frame, so a host keeps its last view per seat, as it keeps
//! [`WrittenObjects`](crate::WrittenObjects).
//!
//! # What a patch holds
//!
//! Objects are the unit: a new or changed object goes whole, a removed one by
//! its id. A zone lists only what changed in it, and says its order only where
//! taking the removed out and putting the new at the end does not give it. The
//! seat lines go whole, one line per seat that changed. Every other field of
//! the view goes on its own ([`ViewField`]).
//!
//! # Foreign bytes
//!
//! A client's patch comes from whoever it connected to, so [`apply`] treats
//! every patch as hostile. Nothing in it panics, its work is `n log n` in what
//! the view and the patch already hold, and it allocates nothing a patch merely
//! claims: every length it reads is the length of something already decoded. A
//! patch it refuses leaves the old view as it was, and the answer to a refusal
//! is a snapshot.
//!
//! [`PlayerView`]: crate::PlayerView

mod apply;
mod diff;
mod digest;
mod fields;

pub use apply::apply;
pub use diff::diff;
pub use digest::{ViewDigest, digest};
pub use fields::ViewField;

use crate::objects::{HandObject, PublicObject};
use crate::seats::SeatView;
use baylee_core::ids::{ObjectId, PlayerId};
use serde::{Deserialize, Serialize};

/// A zone of a [`PlayerView`](crate::PlayerView) that holds public objects.
///
/// The seat of a per-seat zone is its index in the view's list.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Zone {
    /// The battlefield.
    Battlefield,
    /// The stack.
    Stack,
    /// One seat's graveyard.
    Graveyard(u8),
    /// One seat's exile.
    Exile(u8),
    /// One seat's command zone.
    Command(u8),
    /// What this seat is being shown.
    LookingAt,
    /// The library tops revealed.
    LibraryTops,
}

/// What changed in one zone.
///
/// Read in the order of its fields: `removed` leaves, `upserts` replace the
/// object of their id where it is or join at the end in the order given, and
/// `order`, where present, then names every object of the zone in its place.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ZoneEdit<T> {
    /// Objects no longer here.
    pub removed: Vec<ObjectId>,
    /// New objects and changed ones, whole.
    pub upserts: Vec<T>,
    /// The zone's order, when the two lists above do not leave it so.
    pub order: Option<Vec<ObjectId>>,
}

/// A list said against its last state, for a field whose entries have no id:
/// `old[dropped..dropped + kept]`, then `added`, then the last `tail` of `old`.
///
/// Measured on hosted games, `target_objects` was 44 % of all patch bytes and
/// `policy_acts` 9 % while each went whole (`docs/perf-baseline.md`).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ListEdit<T> {
    /// How many entries leave the front.
    pub dropped: u32,
    /// How many stay after those.
    pub kept: u32,
    /// What comes after them.
    pub added: Vec<T>,
    /// How many of the last entries stay after that.
    pub tail: u32,
}

impl<T: PartialEq + Clone> ListEdit<T> {
    /// What makes `old` into `new`: the shorter of keeping both ends and of
    /// a window that slid. A list too long to count goes whole.
    fn of(old: &[T], new: &[T]) -> Self {
        let count = |n: usize| u32::try_from(n).ok();
        let keeping = |dropped, kept, added: &[T], tail| {
            Some(Self {
                dropped: count(dropped)?,
                kept: count(kept)?,
                added: added.to_vec(),
                tail: count(tail)?,
            })
        };
        let front = old.iter().zip(new).take_while(|(a, b)| a == b).count();
        let room = old.len().min(new.len()) - front;
        let back = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take(room)
            .take_while(|(a, b)| a == b)
            .count();
        // The window: `old` from where `new` starts, if it runs to `old`'s end.
        let slid = new.first().and_then(|first| {
            let dropped = old.iter().position(|entry| entry == first)?;
            let kept = old.len() - dropped;
            (kept <= new.len() && old[dropped..] == new[..kept]).then_some((dropped, kept))
        });
        let edit = match slid {
            Some((dropped, kept)) if kept > front + back => keeping(dropped, kept, &new[kept..], 0),
            _ => keeping(0, front, &new[front..new.len() - back], back),
        };
        edit.unwrap_or_else(|| Self {
            dropped: 0,
            kept: 0,
            added: new.to_vec(),
            tail: 0,
        })
    }

    /// The list this leaves of `old`.
    fn applied(&self, old: &[T]) -> Result<Vec<T>, PatchError> {
        let (dropped, kept, tail) = (
            self.dropped as usize,
            self.kept as usize,
            self.tail as usize,
        );
        let head = dropped
            .checked_add(kept)
            .and_then(|end| old.get(dropped..end))
            .ok_or(PatchError::BadList)?;
        let last = old
            .len()
            .checked_sub(tail)
            .and_then(|start| old.get(start..))
            .ok_or(PatchError::BadList)?;
        let mut list = Vec::with_capacity(head.len() + self.added.len() + last.len());
        list.extend_from_slice(head);
        list.extend_from_slice(&self.added);
        list.extend_from_slice(last);
        Ok(list)
    }
}

/// One seat's view as what changed since its last one.
///
/// No field here is left out of the encoding while empty, and none is to be:
/// a reader of a format that does not name its fields has no other way to know
/// which one it is reading.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ViewPatch {
    /// The [`PlayerView::seq`](crate::PlayerView::seq) this applies to.
    pub base: u64,
    /// The [`PlayerView::seq`](crate::PlayerView::seq) it leaves.
    pub seq: u64,
    /// What changed in the zones of public objects, by zone.
    pub zones: Vec<(Zone, ZoneEdit<PublicObject>)>,
    /// What changed in this seat's hand.
    pub hand: Option<ZoneEdit<HandObject>>,
    /// The seat lines that changed, whole.
    pub seats: Vec<SeatView>,
    /// Every other field that changed.
    pub fields: Vec<ViewField>,
    /// The [`digest`] of the view this leaves.
    pub digest: ViewDigest,
}

impl ViewPatch {
    /// Whether the view it leaves differs from its base in nothing but `seq`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
            && self.hand.is_none()
            && self.seats.is_empty()
            && self.fields.is_empty()
    }
}

/// Why a patch does not apply. Every one of these is answered by a snapshot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PatchError {
    /// The patch was made for another view than the one held: a frame between
    /// the two was lost.
    Gap {
        /// The `seq` of the view held.
        have: u64,
        /// The `seq` the patch was made for.
        base: u64,
    },
    /// The patch names a zone the view does not have.
    NoZone(Zone),
    /// The patch names a seat the view has no line for.
    NoSeat(PlayerId),
    /// The patch removes or orders an object the zone does not hold.
    UnknownObject(ObjectId),
    /// A zone's order is no order of the objects it holds.
    BadOrder,
    /// A list edit keeps more of a list than the list holds.
    BadList,
    /// The zone patched holds one id twice, so an id does not name an object
    /// in it. Such a zone is only ever sent whole.
    Unkeyed(ObjectId),
    /// The view the patch leaves is not the view it was made from.
    Digest {
        /// The digest the patch carries.
        expected: ViewDigest,
        /// The digest of what it left.
        found: ViewDigest,
    },
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gap { have, base } => write!(f, "a patch for view {base} met view {have}"),
            Self::NoZone(zone) => write!(f, "a patch names no such zone: {zone:?}"),
            Self::NoSeat(seat) => write!(f, "a patch names no such seat: {seat:?}"),
            Self::UnknownObject(id) => write!(f, "a patch names an unknown object: {id:?}"),
            Self::BadOrder => write!(f, "a patch's order is no order of its zone"),
            Self::BadList => write!(f, "a patch keeps more of a list than it holds"),
            Self::Unkeyed(id) => write!(f, "a patched zone holds {id:?} twice"),
            Self::Digest { expected, found } => {
                write!(f, "a patch left {found:?}, not {expected:?}")
            }
        }
    }
}

impl std::error::Error for PatchError {}

/// What a zone's objects are named by.
trait Keyed {
    /// The object's handle.
    fn key(&self) -> ObjectId;
}

impl Keyed for PublicObject {
    fn key(&self) -> ObjectId {
        self.id
    }
}

impl Keyed for HandObject {
    fn key(&self) -> ObjectId {
        self.id
    }
}

#[cfg(test)]
mod tests;
