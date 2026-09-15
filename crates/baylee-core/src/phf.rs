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
