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
//! Slots live in fixed-size chunks, each behind its own `Arc`. A clone
//! shares every chunk (one reference count per chunk, no object copied),
//! and the first write to a slot copies only the chunk holding it. A
//! decision checkpoint (`Engine::apply` takes one per answer) so pays for
//! the few chunks an answer touched. The single shared vector this
//! replaced was copied whole by the first write after every checkpoint,
//! and nearly every answer writes (`docs/perf-baseline.md` §"The arena is
//! copied a chunk at a time"). Iteration remains slot-ordered.

use baylee_core::ids::ObjectId;
use std::sync::Arc;

/// Slots per chunk, as a power of two. A smaller chunk copies less on a
/// write, and lets the snapshot hash's memo keep more of a board, but costs
/// one reference count per chunk on every clone. Two measured best over
/// 100 self-play games (one 50.4 G instructions, two 50.1 G, four 52.1 G,
/// eight 56.9 G); a 3 000-token board's clone pays for it, 8.0 µs against
/// 4.5 µs at eight, beside the milliseconds an answer on such a board
/// takes anyway (`docs/perf-baseline.md`).
const CHUNK_BITS: u32 = 1;
const CHUNK: usize = 1 << CHUNK_BITS;
const CHUNK_MASK: usize = CHUNK - 1;

#[derive(Clone, Debug)]
struct Slot<T> {
    generation: u8,
    value: Option<T>,
}

impl<T> Slot<T> {
    const UNISSUED: Self = Self {
        generation: 0,
        value: None,
    };
}

/// A dense arena with generational handles.
#[derive(Clone, Debug)]
pub struct Arena<T> {
    /// Every chunk is exactly [`CHUNK`] slots long. The slots of the last
    /// one past `issued` were never handed out and are empty.
    chunks: Vec<Arc<[Slot<T>]>>,
    /// Slots handed out so far (never recycled): where traversal ends.
    issued: usize,
    len: usize,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// One chunk as a reader saw it, to ask later whether it is still the same.
///
/// Holding the key holds a reference to the chunk, and that is what makes
/// the answer sound: every write to a chunk goes through `Arc::make_mut`,
/// which copies a chunk anyone else still references, so while a key is
/// held a write can never change the chunk it names in place — it makes a
/// new one at a new address. Same address and same issued count is
/// therefore same content. (Nothing in an arena value can change behind a
/// shared reference either: no `Cell`, no lock.)
pub(crate) struct ChunkKey<T> {
    chunk: Arc<[Slot<T>]>,
    issued: usize,
}

/// A chunk of an arena being read: its slots, and the key to remember it by.
pub(crate) struct ChunkView<'a, T> {
    chunk: &'a Arc<[Slot<T>]>,
    first: u32,
    issued: usize,
}

impl<'a, T> ChunkView<'a, T> {
    /// Whether this is the chunk `key` was taken of, unchanged.
    pub(crate) fn is(&self, key: &ChunkKey<T>) -> bool {
        Arc::ptr_eq(self.chunk, &key.chunk) && self.issued == key.issued
    }

    /// The key to remember this chunk by.
    pub(crate) fn key(&self) -> ChunkKey<T> {
        ChunkKey {
            chunk: Arc::clone(self.chunk),
            issued: self.issued,
        }
    }

    /// Its issued slots as [`Arena::slots`] gives them.
    pub(crate) fn slots(&self) -> impl Iterator<Item = (u32, u8, Option<&'a T>)> + use<'a, T> {
        let first = self.first;
        let chunk: &'a [Slot<T>] = self.chunk;
        chunk[..self.issued]
            .iter()
            .enumerate()
            .map(move |(i, s)| (first + i as u32, s.generation, s.value.as_ref()))
    }
}

/// A handle's chunk, and its place in the chunk.
const fn locate(id: ObjectId) -> (usize, usize) {
    let slot = id.slot() as usize;
    (slot >> CHUNK_BITS, slot & CHUNK_MASK)
}

impl<T> Arena<T> {
    /// An empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            issued: 0,
            len: 0,
        }
    }

    /// An empty arena with room reserved for `capacity` slots.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            chunks: Vec::with_capacity(capacity.div_ceil(CHUNK)),
            issued: 0,
            len: 0,
        }
    }

    /// The slot a handle names. A slot past `issued` is empty, so a stale
    /// or forged handle into one finds no value.
    fn slot(&self, id: ObjectId) -> Option<&Slot<T>> {
        let (chunk, at) = locate(id);
        self.chunks.get(chunk)?.get(at)
    }

    /// The slots handed out, in slot order.
    fn issued_slots(&self) -> impl Iterator<Item = &Slot<T>> {
        self.chunks.iter().flat_map(|c| c.iter()).take(self.issued)
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
    pub fn insert_with(&mut self, f: impl FnOnce(ObjectId) -> T) -> ObjectId
    where
        T: Clone,
    {
        let slot = u32::try_from(self.issued).unwrap_or(u32::MAX);
        assert!(slot <= ObjectId::MAX_SLOT, "arena slot overflow");
        let id = ObjectId::new(slot, 0);
        let (chunk, at) = locate(id);
        if chunk == self.chunks.len() {
            self.chunks
                .push(std::iter::repeat_with(|| Slot::UNISSUED).take(CHUNK).collect());
        }
        let s = &mut Arc::make_mut(&mut self.chunks[chunk])[at];
        debug_assert!(s.value.is_none() && s.generation == 0, "an issued slot");
        s.value = Some(f(id));
        self.issued += 1;
        self.len += 1;
        id
    }

    /// Inserts a value.
    pub fn insert(&mut self, value: T) -> ObjectId
    where
        T: Clone,
    {
        self.insert_with(|_| value)
    }

    /// Looks up a live entry.
    #[must_use]
    pub fn get(&self, id: ObjectId) -> Option<&T> {
        let s = self.slot(id)?;
        if s.generation == id.generation() {
            s.value.as_ref()
        } else {
            None
        }
    }

    /// Looks up a live entry mutably. Copies the entry's chunk first while
    /// a clone still shares it; a handle that finds nothing copies nothing.
    #[must_use]
    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut T>
    where
        T: Clone,
    {
        self.get(id)?;
        let (chunk, at) = locate(id);
        Arc::make_mut(&mut self.chunks[chunk])[at].value.as_mut()
    }

    /// Removes a live entry, invalidating its handle. The slot is NOT
    /// recycled (see the module docs) — the generation bump is belt and
    /// braces for the snapshot hash.
    pub fn remove(&mut self, id: ObjectId) -> Option<T>
    where
        T: Clone,
    {
        self.get(id)?;
        let (chunk, at) = locate(id);
        let s = &mut Arc::make_mut(&mut self.chunks[chunk])[at];
        let value = s.value.take()?;
        s.generation = s.generation.wrapping_add(1);
        self.len -= 1;
        Some(value)
    }

    /// Iterates live entries in slot order (deterministic).
    pub fn iter(&self) -> impl Iterator<Item = (ObjectId, &T)> {
        self.issued_slots().enumerate().filter_map(|(i, s)| {
            s.value
                .as_ref()
                .map(|v| (ObjectId::new(i as u32, s.generation), v))
        })
    }

    /// Changes every live value `wanted` picks, in slot order (no ids —
    /// bulk maintenance only, e.g. clearing damage at cleanup).
    ///
    /// Asks first and writes after: a chunk holding no wanted value stays
    /// shared with every clone, so a sweep with nothing to do copies
    /// nothing.
    pub fn update_where(&mut self, wanted: impl Fn(&T) -> bool, mut change: impl FnMut(&mut T))
    where
        T: Clone,
    {
        for chunk in &mut self.chunks {
            if !chunk.iter().any(|s| s.value.as_ref().is_some_and(&wanted)) {
                continue;
            }
            for s in Arc::make_mut(chunk).iter_mut() {
                if let Some(value) = s.value.as_mut().filter(|v| wanted(v)) {
                    change(value);
                }
            }
        }
    }

    /// Raw slot triples `(slot, generation, value)` in slot order — the
    /// canonical traversal for snapshot hashing.
    pub fn slots(&self) -> impl Iterator<Item = (u32, u8, Option<&T>)> {
        self.issued_slots()
            .enumerate()
            .map(|(i, s)| (i as u32, s.generation, s.value.as_ref()))
    }

    /// The chunks in slot order, for a reader that remembers what it read
    /// of each (`GameState::snapshot_hash`'s memo, [`ChunkKey`]).
    pub(crate) fn chunk_views(&self) -> impl Iterator<Item = ChunkView<'_, T>> {
        self.chunks.iter().enumerate().map(|(i, chunk)| ChunkView {
            chunk,
            first: (i * CHUNK) as u32,
            issued: (self.issued - i * CHUNK).min(CHUNK),
        })
    }

    /// How many chunks no clone shares. For the tests of the sharing.
    #[cfg(test)]
    fn unshared_chunks(&self) -> usize {
        self.chunks
            .iter()
            .filter(|c| Arc::strong_count(c) == 1)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_isolate_each_mutation_without_copying_read_only_slots() {
        let mut original = Arena::new();
        let first = original.insert(7);
        let second = original.insert(11);
        let snapshot = original.clone();
        assert_eq!(original.unshared_chunks(), 0, "a clone shares everything");
        assert!(original.get_mut(ObjectId::new(99, 0)).is_none());
        assert!(original.remove(ObjectId::new(99, 0)).is_none());
        // A handle into the issued chunk's unissued tail finds nothing
        // and copies nothing either.
        assert!(original.get_mut(ObjectId::new(5, 0)).is_none());
        assert_eq!(original.unshared_chunks(), 0, "a miss copies nothing");

        *original.get_mut(first).unwrap() = 13;
        assert_eq!(snapshot.get(first), Some(&7));
        assert_eq!(original.unshared_chunks(), 1);
        let mut removed = snapshot.clone();
        assert_eq!(removed.remove(second), Some(11));
        assert_eq!(snapshot.get(second), Some(&11));
        let mut inserted = snapshot.clone();
        let third = inserted.insert(17);
        assert!(snapshot.get(third).is_none());
        let mut bulk = snapshot.clone();
        bulk.update_where(|_| true, |value| *value += 1);
        assert_eq!(bulk.get(first), Some(&8));
        assert_eq!(snapshot.get(first), Some(&7));
        assert_eq!(snapshot.len(), 2);
    }

    /// Only the chunk a write lands in is copied: the rest of a big arena
    /// stays shared with the checkpoint it was cloned from, and a sweep
    /// that wants nothing copies nothing.
    #[test]
    fn a_write_copies_only_its_own_chunk() {
        let mut arena: Arena<u32> = Arena::new();
        let ids: Vec<ObjectId> = (0..10 * CHUNK as u32).map(|i| arena.insert(i)).collect();
        let snapshot = arena.clone();
        assert_eq!(arena.unshared_chunks(), 0);
        *arena.get_mut(ids[3 * CHUNK + 1]).unwrap() = 0;
        arena.remove(ids[7 * CHUNK]);
        assert_eq!(arena.unshared_chunks(), 2, "two chunks written, two copied");
        arena.update_where(|v| *v == u32::MAX, |v| *v = 0);
        assert_eq!(arena.unshared_chunks(), 2, "a sweep that wants nothing");
        arena.update_where(|v| *v == 5, |v| *v = 6);
        assert_eq!(arena.unshared_chunks(), 3, "and one that wants one value");
        assert_eq!(arena.get(ids[5]), Some(&6));
        assert_eq!(snapshot.get(ids[5]), Some(&5));
        assert_eq!(snapshot.get(ids[3 * CHUNK + 1]), Some(&(3 * CHUNK as u32 + 1)));
        assert_eq!(snapshot.get(ids[7 * CHUNK]), Some(&(7 * CHUNK as u32)));
    }

    /// The chunked arena against the plainest arena there is — a
    /// `Vec<(generation, Option<value>)>` — under random inserts, writes,
    /// removals, sweeps, clones and stale handles. Every observable answer
    /// (`get`, `len`, `iter`, `slots`) has to agree after every operation,
    /// in the arena and in every clone taken along the way, which is what
    /// shows a write never reaches a clone that shares the chunk.
    #[test]
    fn the_chunked_arena_answers_as_a_flat_vector_does() {
        use rand_core::{Rng, SeedableRng};
        type Model = Vec<(u8, Option<u64>)>;
        fn agree(arena: &Arena<u64>, model: &Model) {
            let slots: Vec<(u32, u8, Option<u64>)> =
                arena.slots().map(|(i, g, v)| (i, g, v.copied())).collect();
            let expected: Vec<(u32, u8, Option<u64>)> = model
                .iter()
                .enumerate()
                .map(|(i, (g, v))| (i as u32, *g, *v))
                .collect();
            assert_eq!(slots, expected);
            assert_eq!(arena.len(), model.iter().filter(|(_, v)| v.is_some()).count());
            let live: Vec<(ObjectId, u64)> = arena.iter().map(|(id, v)| (id, *v)).collect();
            let expected: Vec<(ObjectId, u64)> = model
                .iter()
                .enumerate()
                .filter_map(|(i, (g, v))| v.map(|v| (ObjectId::new(i as u32, *g), v)))
                .collect();
            assert_eq!(live, expected);
        }
        for seed in 0..64 {
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
            let mut arena: Arena<u64> = Arena::new();
            let mut model: Model = Vec::new();
            let mut clones: Vec<(Arena<u64>, Model)> = Vec::new();
            for step in 0..600_u64 {
                // A handle to a slot that was issued, one past the end, or
                // a stale generation of an issued one.
                let pick = |rng: &mut rand_chacha::ChaCha8Rng, model: &Model| {
                    let slot = (rng.next_u32() as usize) % (model.len() + 2);
                    let generation = model.get(slot).map_or(0, |(g, _)| *g);
                    let stale = rng.next_u32() % 8 == 0;
                    ObjectId::new(slot as u32, generation.wrapping_add(u8::from(stale)))
                };
                match rng.next_u32() % 10 {
                    0..=2 => {
                        let id = arena.insert(step);
                        assert_eq!(id, ObjectId::new(model.len() as u32, 0));
                        model.push((0, Some(step)));
                    }
                    3..=4 => {
                        let id = pick(&mut rng, &model);
                        let expected = model
                            .get_mut(id.slot() as usize)
                            .filter(|(g, _)| *g == id.generation())
                            .and_then(|(_, v)| v.as_mut());
                        match (arena.get_mut(id), expected) {
                            (Some(a), Some(m)) => {
                                *a += 1000;
                                *m += 1000;
                            }
                            (None, None) => {}
                            (a, m) => panic!("get_mut {id:?}: {a:?} against {m:?}"),
                        }
                    }
                    5..=6 => {
                        let id = pick(&mut rng, &model);
                        let expected = match model.get_mut(id.slot() as usize) {
                            Some((g, v)) if *g == id.generation() && v.is_some() => {
                                *g = g.wrapping_add(1);
                                v.take()
                            }
                            _ => None,
                        };
                        assert_eq!(arena.remove(id), expected);
                    }
                    7 => {
                        let modulus = 2 + u64::from(rng.next_u32() % 5);
                        arena.update_where(|v| v % modulus == 0, |v| *v += 1);
                        for (_, v) in &mut model {
                            if let Some(v) = v.as_mut().filter(|v| **v % modulus == 0) {
                                *v += 1;
                            }
                        }
                    }
                    8 => clones.push((arena.clone(), model.clone())),
                    _ => {
                        let id = pick(&mut rng, &model);
                        let expected = model
                            .get(id.slot() as usize)
                            .filter(|(g, _)| *g == id.generation())
                            .and_then(|(_, v)| *v);
                        assert_eq!(arena.get(id).copied(), expected);
                    }
                }
                agree(&arena, &model);
            }
            for (clone, model) in &clones {
                agree(clone, model);
            }
        }
    }

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
