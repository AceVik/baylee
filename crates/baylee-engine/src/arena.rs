//! Dense, generational object storage.
//!
//! Handles are [`ObjectId`]s (`slot:24 | generation:8`). Slots are
//! **never recycled** within a game: recycling is exactly what lets a
//! stale handle alias a brand-new object once the 8-bit generation
//! wraps (ABA, after 256 reuses of one slot — reachable in a long
//! token game). 24 bits of slot space is ~16.7M objects per game, far
//! beyond any real match, and removed slots hold no value, so the
//! footprint stays proportional to objects ever created.
//!
//! Clone is a flat `Vec` copy (AI lookahead). Iteration is slot-ordered —
//! always deterministic.

use baylee_core::ids::ObjectId;

#[derive(Clone, Debug)]
struct Slot<T> {
    generation: u8,
    value: Option<T>,
}

/// A dense arena with generational handles.
#[derive(Clone, Debug)]
pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    len: usize,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    /// An empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            len: 0,
        }
    }

    /// An empty arena with reserved space.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            len: 0,
        }
    }

    /// Number of live entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the arena is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Inserts a value built from its own id.
    ///
    /// # Panics
    /// When more than [`ObjectId::MAX_SLOT`] objects are created in one
    /// game.
    pub fn insert_with(&mut self, f: impl FnOnce(ObjectId) -> T) -> ObjectId {
        let slot = self.slots.len() as u32;
        assert!(slot <= ObjectId::MAX_SLOT, "arena slot overflow");
        self.slots.push(Slot {
            generation: 0,
            value: None,
        });
        let id = ObjectId::new(slot, 0);
        let s = &mut self.slots[id.slot() as usize];
        debug_assert!(s.value.is_none());
        s.value = Some(f(id));
        self.len += 1;
        id
    }

    /// Inserts a value.
    pub fn insert(&mut self, value: T) -> ObjectId {
        self.insert_with(|_| value)
    }

    /// Looks up a live entry.
    #[must_use]
    pub fn get(&self, id: ObjectId) -> Option<&T> {
        let s = self.slots.get(id.slot() as usize)?;
        if s.generation == id.generation() {
            s.value.as_ref()
        } else {
            None
        }
    }

    /// Looks up a live entry mutably.
    #[must_use]
    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut T> {
        let s = self.slots.get_mut(id.slot() as usize)?;
        if s.generation == id.generation() {
            s.value.as_mut()
        } else {
            None
        }
    }

    /// Removes a live entry, invalidating its handle. The slot is NOT
    /// recycled (see the module docs) — the generation bump is belt and
    /// braces for the snapshot hash.
    pub fn remove(&mut self, id: ObjectId) -> Option<T> {
        let s = self.slots.get_mut(id.slot() as usize)?;
        if s.generation != id.generation() {
            return None;
        }
        let value = s.value.take()?;
        s.generation = s.generation.wrapping_add(1);
        self.len -= 1;
        Some(value)
    }

    /// Iterates live entries in slot order (deterministic).
    pub fn iter(&self) -> impl Iterator<Item = (ObjectId, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| {
            s.value
                .as_ref()
                .map(|v| (ObjectId::new(i as u32, s.generation), v))
        })
    }

    /// Iterates all live values mutably (no ids — bulk maintenance only,
    /// e.g. clearing damage at cleanup).
    pub fn iter_mut_all(&mut self) -> impl Iterator<Item = &mut T> {
        self.slots.iter_mut().filter_map(|s| s.value.as_mut())
    }

    /// Raw slot triples `(slot, generation, value)` in slot order — the
    /// canonical traversal for snapshot hashing.
    pub fn slots(&self) -> impl Iterator<Item = (u32, u8, Option<&T>)> {
        self.slots
            .iter()
            .enumerate()
            .map(|(i, s)| (i as u32, s.generation, s.value.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_remove() {
        let mut arena: Arena<u32> = Arena::new();
        let a = arena.insert(10);
        let b = arena.insert(20);
        assert_eq!(arena.get(a), Some(&10));
        assert_eq!(arena.get(b), Some(&20));
        assert_eq!(arena.len(), 2);

        arena.remove(a);
        assert!(arena.get(a).is_none()); // stale generation rejected
        assert_eq!(arena.len(), 1);

        let c = arena.insert(30); // a FRESH slot — slots are never recycled
        assert_ne!(c.slot(), a.slot());
        assert_eq!(arena.get(c), Some(&30));
        assert!(arena.get(a).is_none());
    }

    #[test]
    fn a_stale_handle_can_never_alias_a_new_object() {
        // The ABA regression: with slot recycling, 256 remove/insert
        // cycles on one slot wrapped the 8-bit generation back to the
        // stale handle's value and `get` returned a DIFFERENT object.
        let mut arena: Arena<u32> = Arena::new();
        let stale = arena.insert(0);
        arena.remove(stale);
        for i in 1..=1000u32 {
            let id = arena.insert(i);
            arena.remove(id);
        }
        assert!(arena.get(stale).is_none());
    }

    #[test]
    fn iter_is_slot_ordered() {
        let mut arena: Arena<u32> = Arena::new();
        let ids: Vec<_> = (0..5).map(|i| arena.insert(i * 100)).collect();
        arena.remove(ids[2]);
        let seen: Vec<u32> = arena.iter().map(|(_, v)| *v).collect();
        assert_eq!(seen, vec![0, 100, 300, 400]);
    }

    /// Every object in this engine is built from its own id — a
    /// `GameObject` stores the handle it will be found under — so the
    /// closure is handed the id the value is about to live at, and not one
    /// the caller has to guess and then correct.
    #[test]
    fn a_value_is_built_from_the_id_it_will_answer_to() {
        let mut arena: Arena<ObjectId> = Arena::new();
        let a = arena.insert_with(|id| id);
        let b = arena.insert_with(|id| id);
        assert_eq!(arena.get(a), Some(&a));
        assert_eq!(arena.get(b), Some(&b));
        assert_ne!(a, b);
    }

    /// `slots` is the canonical traversal for the snapshot hash, and it is
    /// canonical because it walks **every** slot — including the empty ones,
    /// with the generation their removal bumped. Two states that reached the
    /// same board by different routes have to hash differently, and the
    /// removed slots are the only record that one of them made a token and
    /// lost it.
    #[test]
    fn the_snapshot_traversal_walks_the_empty_slots_too() {
        let mut arena: Arena<u32> = Arena::new();
        let a = arena.insert(1);
        arena.insert(2);
        arena.remove(a);

        let walked: Vec<(u32, u8, Option<u32>)> =
            arena.slots().map(|(i, g, v)| (i, g, v.copied())).collect();
        assert_eq!(
            walked,
            vec![(0, 1, None), (1, 0, Some(2))],
            "the emptied slot is still walked, and says it has been emptied"
        );
        assert_eq!(arena.len(), 1, "but it is not a live entry");
        assert_eq!(arena.iter().count(), 1, "and `iter` does not show it");
    }

    /// An id out of `iter` is an id `get` accepts: it carries the slot's
    /// **current** generation, so a caller that collects ids and then reads
    /// them back does not hand itself a stale handle.
    #[test]
    fn an_id_from_iter_is_one_get_still_answers() {
        let mut arena: Arena<u32> = Arena::new();
        let first = arena.insert(7);
        arena.insert(8);
        arena.remove(first);
        arena.insert(9);

        for (id, value) in arena.iter().map(|(id, v)| (id, *v)).collect::<Vec<_>>() {
            assert_eq!(arena.get(id), Some(&value));
        }
    }

    /// Removing twice takes nothing the second time, and takes nothing off
    /// the count either — a door that decremented on a handle it had
    /// already invalidated would make `len` drift from what `iter` finds.
    #[test]
    fn a_handle_can_only_be_spent_once() {
        let mut arena: Arena<u32> = Arena::new();
        let a = arena.insert(1);
        assert_eq!(arena.remove(a), Some(1));
        assert_eq!(arena.remove(a), None);
        assert_eq!(arena.len(), 0);
        assert!(arena.is_empty());
        assert_eq!(arena.iter().count(), 0);
    }
}
