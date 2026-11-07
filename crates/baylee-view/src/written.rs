//! One seat's objects as the wire last carried them.
//!
//! A view is sent whole, and nearly every object in it is the object the seat
//! was sent the view before: on a live table writing them is a fifth of an
//! engine's time (`docs/perf-baseline.md`). [`WrittenObjects`] keeps each one
//! beside the JSON it came to, and writes that again while the object is equal.
//!
//! An object is taken from here only when it is equal, field by field, to the
//! one those bytes were written from. There is no revision to keep up and no
//! digest to collide: a view written here is byte for byte the view written
//! plainly, whatever the engine did in between.
//!
//! It belongs to one seat. What an object shows depends on who is looking
//! (CR 400.2), so a host keeps one of these per seat, for that seat's views, and
//! never shares one. The table is held by the host and reaches
//! [`PublicObject`]'s lists through a thread-local only for the length of one
//! [`WrittenObjects::write`], because the derived `Serialize` of
//! [`PlayerView`] offers no other way in.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use baylee_core::ids::ObjectId;
use serde::{Serialize, Serializer};
use serde_json::value::{RawValue, to_raw_value};

use crate::{PlayerView, PublicObject};

/// One object and the JSON it came to.
#[derive(Debug)]
struct Written {
    object: PublicObject,
    json: Box<RawValue>,
    /// The last view that showed it.
    view: u64,
}

/// What one seat's last view said of each object in it.
#[derive(Debug, Default)]
pub struct WrittenObjects {
    objects: BTreeMap<ObjectId, Written>,
    /// Views written so far, which tells an object still in play from one that
    /// has left: nothing is kept past the view after the one that showed it.
    views: u64,
}

thread_local! {
    /// The seat's objects while [`WrittenObjects::write`] runs, and only then.
    static WRITING: RefCell<Option<WrittenObjects>> = const { RefCell::new(None) };
}

/// Hands the objects back to their seat, also when a write unwinds: nothing may
/// stay in the thread for another seat's view to find.
struct Writing<'a>(&'a mut WrittenObjects);

impl Drop for Writing<'_> {
    fn drop(&mut self) {
        if let Some(objects) = WRITING.take() {
            *self.0 = objects;
        }
    }
}

impl WrittenObjects {
    /// `view` as JSON into `out`, the same bytes as `serde_json::to_writer`.
    ///
    /// # Errors
    /// When `view` does not serialize, or `out` refuses the bytes.
    pub fn write(
        &mut self,
        out: impl std::io::Write,
        view: &PlayerView,
    ) -> Result<(), serde_json::Error> {
        self.views += 1;
        let views = self.views;
        WRITING.set(Some(std::mem::take(self)));
        let writing = Writing(self);
        let written = serde_json::to_writer(out, view);
        drop(writing);
        self.objects.retain(|_, object| object.view == views);
        written
    }

    /// The JSON of `object`, from what this seat was last sent if that is equal.
    fn json_of(&mut self, object: &PublicObject) -> Result<&RawValue, serde_json::Error> {
        let view = self.views;
        Ok(match self.objects.entry(object.id) {
            Entry::Occupied(entry) => {
                let written = entry.into_mut();
                if written.object != *object {
                    written.json = to_raw_value(object)?;
                    written.object.clone_from(object);
                }
                written.view = view;
                &written.json
            }
            Entry::Vacant(entry) => {
                &entry
                    .insert(Written {
                        object: object.clone(),
                        json: to_raw_value(object)?,
                        view,
                    })
                    .json
            }
        })
    }
}

/// One object of a zone: from the seat's table during a write, else plainly.
struct Object<'a>(&'a PublicObject);

impl Serialize for Object<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        WRITING.with_borrow_mut(|writing| match writing {
            Some(objects) => objects
                .json_of(self.0)
                .map_err(serde::ser::Error::custom)?
                .serialize(serializer),
            None => self.0.serialize(serializer),
        })
    }
}

/// A zone of a [`PlayerView`] (`#[serde(serialize_with)]`).
pub(crate) fn zone<S: Serializer>(
    objects: &[PublicObject],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(objects.iter().map(Object))
}

/// A zone per seat of a [`PlayerView`] (`#[serde(serialize_with)]`).
pub(crate) fn zones<S: Serializer>(
    seats: &[Vec<PublicObject>],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    /// One seat's zone.
    struct Zone<'a>(&'a [PublicObject]);

    impl Serialize for Zone<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            zone(self.0, serializer)
        }
    }

    serializer.collect_seq(seats.iter().map(|objects| Zone(objects)))
}

#[cfg(test)]
mod tests;
