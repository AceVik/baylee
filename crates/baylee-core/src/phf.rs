//! A perfect hash over a fixed set of strings, computed when the code is
//! generated and never at runtime.
//!
//! # Why this lives in `baylee-core`
//!
//! A perfect hash is two halves of one arithmetic: a generator that places
//! the keys and a lookup that finds them again. Written twice they drift,
//! and the drift is silent — a table built with one hash and read with
//! another answers `None` for every key, which looks exactly like a table
//! that was never filled in. So the arithmetic is written **once**, here,
//! in the one crate both halves already link: `baylee-cards-codegen` builds
//! a [`Table`] and `baylee-cards` reads it.
//!
//! It knows nothing about cards. What it answers is "which slot did the key
//! with this spelling get", and the caller is what turns a slot into an
//! answer — and what **verifies** the key, because a perfect hash is perfect
//! only over the keys it was built from. Any other string lands in some slot
//! too, and the caller has to compare the spelling it finds there before
//! believing it. That comparison is the whole difference between `None` and a
//! wrong card.
//!
//! # The scheme (CHD)
//!
//! Keys are dealt into `displacements.len()` buckets by the top bits of
//! their hash. Each bucket gets one displacement `d`, chosen by the
//! generator so that every key in it lands on a slot no earlier bucket has
//! taken — and buckets are placed **largest first**, which is what makes a
//! small `d` enough for almost all of them. The slot of a key is then
//!
//! ```text
//! (base + d * step) & (slots - 1)
//! ```
//!
//! with `base` and `step` two independent halves of the key's own hash.

/// FNV-1a over the bytes, seeded, then avalanched.
///
/// FNV alone is not good enough for this: its low bits barely move, and
/// every index here is taken with a mask. The finalizer is what spreads one
/// changed letter across all 64 bits.
#[must_use]
pub const fn hash(key: &[u8], seed: u64) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325 ^ seed;
    let mut i = 0;
    while i < key.len() {
        h ^= key[i] as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    fmix64(h)
}

/// Murmur3's 64-bit finalizer: the avalanche step, and nothing else.
#[must_use]
pub const fn fmix64(mut h: u64) -> u64 {
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^= h >> 33;
    h
}

/// The second seed, an odd constant away from the first, so `base` and
/// `step` come from two independent hashes rather than two ends of one.
pub const SEED2_OFFSET: u64 = 0x9e37_79b9_7f4a_7c15;

/// The three numbers a key contributes: which bucket it falls in, and the
/// arithmetic progression its slot is drawn from.
#[must_use]
pub const fn parts(key: &[u8], seed: u64, buckets: usize) -> (usize, u32, u32) {
    let h1 = hash(key, seed);
    let h2 = hash(key, seed ^ SEED2_OFFSET);
    let bucket = (h1 >> 32) as usize % buckets;
    let base = h1 as u32;
    // Odd, so the progression walks every residue and a bucket of two keys
    // can always be separated by *some* displacement.
    let step = h2 as u32 | 1;
    (bucket, base, step)
}

/// A placed key set: the displacement per bucket, and how many slots the
/// keys were placed into.
///
/// `slots` is a power of two, so the modulo is a mask.
#[derive(Debug, Clone, Copy)]
pub struct Table {
    /// One displacement per bucket, in bucket order.
    pub displacements: &'static [u32],
    /// How many slots the table has. A power of two.
    pub slots: usize,
    /// The seed the generator settled on.
    pub seed: u64,
}

impl Table {
    /// The slot this key was placed in — or would be, if it is not a key.
    ///
    /// Always answers: every string lands somewhere. The caller compares the
    /// spelling it finds at that slot before believing it.
    #[must_use]
    pub const fn slot(&self, key: &[u8]) -> usize {
        let (bucket, base, step) = parts(key, self.seed, self.displacements.len());
        let d = self.displacements[bucket];
        (base.wrapping_add(d.wrapping_mul(step)) as usize) & (self.slots - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::{SEED2_OFFSET, Table, fmix64, hash, parts};

    /// **These four numbers are what every committed table was built
    /// with.** The generator places the keys with this arithmetic and the
    /// lookup finds them again with it, so changing it does not produce a
    /// worse table — it produces a table that answers `None` for every key,
    /// which looks exactly like one that was never filled in. Written down
    /// rather than derived, because a test that recomputes the hash agrees
    /// with whatever the hash has become.
    #[test]
    fn the_arithmetic_the_committed_tables_were_placed_with() {
        assert_eq!(fmix64(0), 0);
        assert_eq!(fmix64(1), 12_994_781_566_227_106_604);
        assert_eq!(hash(b"", 0), 17_280_346_270_528_514_342);
        assert_eq!(hash(b"Lightning Bolt", 0), 13_927_349_926_344_005_927);
        assert_eq!(
            hash(b"Lightning Bolt", SEED2_OFFSET),
            538_476_385_227_210_331,
            "the second seed is a different hash of the same key, not a \
             second reading of the first one"
        );
    }

    /// One changed letter moves all 64 bits — the finalizer is there for
    /// exactly this, and card names differ by a letter far more often than
    /// they differ wildly.
    #[test]
    fn one_changed_letter_is_a_different_hash_everywhere() {
        let a = hash(b"Lightning Bolt", 0);
        let b = hash(b"Lightning Bolu", 0);
        assert_ne!(a, b);
        let differing = (a ^ b).count_ones();
        assert!(
            (16..=48).contains(&differing),
            "{differing} of 64 bits differ, which is not an avalanche"
        );
        assert_ne!(a >> 32, b >> 32, "the half that picks the bucket moves");
    }

    /// The three numbers a key contributes: a bucket that is in range, and
    /// a progression whose step is **odd**, which is what lets some
    /// displacement separate any two keys sharing a bucket. An even step
    /// walks half the residues and a bucket of two can be unplaceable.
    #[test]
    fn a_keys_progression_has_an_odd_step_and_an_in_range_bucket() {
        for buckets in [1usize, 3, 64, 1000] {
            for key in [
                "",
                "a",
                "Lightning Bolt",
                "Sheoldred // The True Scriptures",
            ] {
                let (bucket, _base, step) = parts(key.as_bytes(), 0, buckets);
                assert!(bucket < buckets, "{key} fell outside {buckets} buckets");
                assert_eq!(step % 2, 1, "{key} got an even step");
            }
        }
    }

    /// A perfect hash is perfect only over the keys it was built from, so
    /// the lookup **always answers** and the caller compares the spelling it
    /// finds. A slot outside the table would be a panic in a `const fn`
    /// nobody writes a bounds check around.
    #[test]
    fn every_string_lands_in_a_slot_whether_it_is_a_key_or_not() {
        let table = Table {
            displacements: &[0, 1, 2, 3],
            slots: 8,
            seed: 7,
        };
        for key in ["", "not a card", "Лайтнинг", "稲妻", "\u{0}\u{1}"] {
            assert!(table.slot(key.as_bytes()) < table.slots, "{key}");
        }
    }

    /// The two halves are one arithmetic, which is the whole reason this
    /// module is in `baylee-core` rather than in the generator. Placed here
    /// the way the generator places — largest bucket first, each key onto a
    /// slot no earlier bucket took — the lookup finds every key again.
    #[test]
    fn a_table_placed_with_parts_is_read_back_by_slot() {
        const KEYS: &[&str] = &[
            "Forest",
            "Island",
            "Mountain",
            "Plains",
            "Swamp",
            "Lightning Bolt",
            "Mox Opal",
            "Force of Will",
            "Karakas",
            "Volrath's Stronghold",
        ];
        let buckets = 4usize;
        let slots = 16usize;
        let seed = 42u64;

        // Keys per bucket, largest bucket placed first.
        let mut order: Vec<usize> = (0..buckets).collect();
        let members = |b: usize| -> Vec<&'static str> {
            KEYS.iter()
                .copied()
                .filter(|k| parts(k.as_bytes(), seed, buckets).0 == b)
                .collect()
        };
        order.sort_by_key(|b| std::cmp::Reverse(members(*b).len()));

        let mut taken = vec![false; slots];
        let mut displacements = vec![0u32; buckets];
        for bucket in order {
            let keys = members(bucket);
            // Bounded rather than open: an unplaceable bucket is a
            // failed search and not a hang.
            let d = (0u32..4096).find(|d| {
                let mut seen: Vec<usize> = Vec::new();
                keys.iter().all(|k| {
                    let (_, base, step) = parts(k.as_bytes(), seed, buckets);
                    let slot = base.wrapping_add(d.wrapping_mul(step)) as usize & (slots - 1);
                    if taken[slot] || seen.contains(&slot) {
                        return false;
                    }
                    seen.push(slot);
                    true
                })
            });
            let d = d.expect("a displacement under 4096 places this bucket");
            displacements[bucket] = d;
            for k in keys {
                let (_, base, step) = parts(k.as_bytes(), seed, buckets);
                taken[base.wrapping_add(d.wrapping_mul(step)) as usize & (slots - 1)] = true;
            }
        }

        // `slot` is the only thing that reads it back, and it has to agree.
        let leaked: &'static [u32] = Box::leak(displacements.into_boxed_slice());
        let table = Table {
            displacements: leaked,
            slots,
            seed,
        };
        let mut found: Vec<usize> = KEYS.iter().map(|k| table.slot(k.as_bytes())).collect();
        assert_eq!(found.len(), KEYS.len());
        found.sort_unstable();
        found.dedup();
        assert_eq!(
            found.len(),
            KEYS.len(),
            "two keys share a slot: the lookup does not agree with the \
             placement it was given"
        );
    }
}
