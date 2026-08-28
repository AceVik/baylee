//! Dense, generational object storage.
//!
//! Handles are [`ObjectId`]s (`slot:24 | generation:8`); the generation
//! guards against stale-slot reuse. Clone is a flat `Vec` copy (AI
//! lookahead). Iteration is slot-ordered — always deterministic.

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
    free: Vec<u32>,
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
            free: Vec::new(),
            len: 0,
        }
    }

    /// An empty arena with reserved space.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            free: Vec::new(),
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
    /// When more than [`ObjectId::MAX_SLOT`] objects are alive.
    pub fn insert_with(&mut self, f: impl FnOnce(ObjectId) -> T) -> ObjectId {
        let id = if let Some(slot) = self.free.pop() {
            ObjectId::new(slot, self.slots[slot as usize].generation)
        } else {
            let slot = self.slots.len() as u32;
            assert!(slot <= ObjectId::MAX_SLOT, "arena slot overflow");
            self.slots.push(Slot {
                generation: 0,
                value: None,
            });
            ObjectId::new(slot, 0)
        };
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

    /// Removes a live entry, invalidating its handle.
    pub fn remove(&mut self, id: ObjectId) -> Option<T> {
        let s = self.slots.get_mut(id.slot() as usize)?;
        if s.generation != id.generation() {
            return None;
        }
        let value = s.value.take()?;
        s.generation = s.generation.wrapping_add(1);
        self.free.push(id.slot());
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

        let c = arena.insert(30); // reuses the freed slot
        assert_eq!(c.slot(), a.slot());
        assert_ne!(c.generation(), a.generation());
        assert_eq!(arena.get(c), Some(&30));
        assert!(arena.get(a).is_none());
    }

    #[test]
    fn iter_is_slot_ordered() {
        let mut arena: Arena<u32> = Arena::new();
        let ids: Vec<_> = (0..5).map(|i| arena.insert(i * 100)).collect();
        arena.remove(ids[2]);
        let seen: Vec<u32> = arena.iter().map(|(_, v)| *v).collect();
        assert_eq!(seen, vec![0, 100, 300, 400]);
    }
}
