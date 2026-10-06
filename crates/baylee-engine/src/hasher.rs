//! The byte sink behind `GameState::snapshot_hash` and `loop_signature`.
//!
//! An xxh3 stream with a write buffer of its own in front. The hash walks
//! a few hundred bytes per object, most of them as single-byte and
//! single-word fields, and an `Xxh3::update` call per field cost more than
//! the bytes did. Here a field is a fixed-size store into the buffer and
//! the stream is fed [`BUF`] bytes at a time: 6 % fewer instructions over
//! 120 house-AI games, whose harness hashes after every input
//! (`docs/perf-baseline.md`).
//!
//! **The digest does not change.** xxh3's streaming digest depends only on
//! the bytes it was fed, never on how they were split into updates, and
//! every method writes exactly the bytes the unbuffered sink wrote. A game
//! record keeps the hash after every input and a replay checks it, so this
//! is load-bearing: `the_buffered_sink_digests_exactly_what_the_plain_one_did`
//! holds the two against each other and against one-shot `xxh3_64` on
//! random write sequences.
//!
//! Fixed byte order (little-endian) and word width (`usize` as 64 bits)
//! keep the stream the same on every target, wasm included. The one
//! exception is inherited, not chosen: `[u64; N]`'s own `Hash` writes
//! native-endian words, and [`Hasher::words`] writes exactly those.

use xxhash_rust::xxh3::Xxh3;

/// Bytes gathered before they are handed to the stream: four of xxh3's
/// 64-byte stripes per update.
const BUF: usize = 256;

pub(crate) struct Hasher {
    inner: Xxh3,
    len: usize,
    buf: [u8; BUF],
}

impl Hasher {
    pub(crate) fn new() -> Self {
        Self {
            inner: Xxh3::new(),
            len: 0,
            buf: [0; BUF],
        }
    }

    /// The digest of everything written.
    pub(crate) fn finish(mut self) -> u64 {
        self.flush();
        self.inner.digest()
    }

    fn flush(&mut self) {
        self.inner.update(&self.buf[..self.len]);
        self.len = 0;
    }

    /// A field of a size known where it is written: a fixed-size copy.
    #[inline(always)]
    pub(crate) fn put<const N: usize>(&mut self, bytes: [u8; N]) {
        if self.len + N > BUF {
            self.flush();
        }
        self.buf[self.len..self.len + N].copy_from_slice(&bytes);
        self.len += N;
    }

    pub(crate) fn bytes(&mut self, b: &[u8]) {
        if b.len() > BUF / 2 {
            // Large enough that copying it first only costs: hand the
            // stream what is gathered, then the slice itself.
            self.flush();
            self.inner.update(b);
            return;
        }
        if self.len + b.len() > BUF {
            self.flush();
        }
        self.buf[self.len..self.len + b.len()].copy_from_slice(b);
        self.len += b.len();
    }
    pub(crate) fn u8(&mut self, v: u8) {
        self.put([v]);
    }
    pub(crate) fn i8(&mut self, v: i8) {
        self.put(v.to_le_bytes());
    }
    pub(crate) fn u16(&mut self, v: u16) {
        self.put(v.to_le_bytes());
    }
    pub(crate) fn u32(&mut self, v: u32) {
        self.put(v.to_le_bytes());
    }
    pub(crate) fn i16(&mut self, v: i16) {
        self.put(v.to_le_bytes());
    }
    pub(crate) fn i32(&mut self, v: i32) {
        self.put(v.to_le_bytes());
    }
    pub(crate) fn u64(&mut self, v: u64) {
        self.put(v.to_le_bytes());
    }
    pub(crate) fn u128(&mut self, v: u128) {
        self.put(v.to_le_bytes());
    }
    pub(crate) fn usize(&mut self, v: usize) {
        self.put((v as u64).to_le_bytes());
    }
    /// What `[u64; N]`'s own `Hash` writes — its length, then the words'
    /// native bytes in one slice — as fixed-size writes.
    pub(crate) fn words(&mut self, words: &[u64; 16]) {
        self.usize(words.len());
        let mut bytes = [0u8; 128];
        for (out, word) in bytes.chunks_exact_mut(8).zip(words) {
            out.copy_from_slice(&word.to_ne_bytes());
        }
        self.put(bytes);
    }
    pub(crate) fn boolean(&mut self, v: bool) {
        self.u8(u8::from(v));
    }
    pub(crate) fn option_u32(&mut self, v: Option<u32>) {
        match v {
            Some(x) => {
                self.u8(1);
                self.u32(x);
            }
            None => self.u8(0),
        }
    }
}

// Fixed byte order and word width keep structural DSL hashing deterministic
// across native and wasm builds. References hash their contents, never addresses.
impl std::hash::Hasher for Hasher {
    /// The digest so far, without consuming the sink. Nothing in the engine
    /// asks it (the inherent [`Hasher::finish`] is the door); it is here
    /// because the trait wants it, and it answers what that one would.
    fn finish(&self) -> u64 {
        let mut inner = self.inner.clone();
        inner.update(&self.buf[..self.len]);
        inner.digest()
    }
    fn write(&mut self, bytes: &[u8]) {
        self.bytes(bytes);
    }
    fn write_u8(&mut self, value: u8) {
        self.u8(value);
    }
    fn write_u16(&mut self, value: u16) {
        self.u16(value);
    }
    fn write_u32(&mut self, value: u32) {
        self.u32(value);
    }
    fn write_u64(&mut self, value: u64) {
        self.u64(value);
    }
    fn write_u128(&mut self, value: u128) {
        self.u128(value);
    }
    fn write_i8(&mut self, value: i8) {
        self.i8(value);
    }
    fn write_i16(&mut self, value: i16) {
        self.i16(value);
    }
    fn write_i32(&mut self, value: i32) {
        self.i32(value);
    }
    fn write_i64(&mut self, value: i64) {
        self.put(value.to_le_bytes());
    }
    fn write_i128(&mut self, value: i128) {
        self.put(value.to_le_bytes());
    }
    fn write_usize(&mut self, value: usize) {
        self.usize(value);
    }
    fn write_isize(&mut self, value: isize) {
        self.put((value as i64).to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::{Rng, SeedableRng};
    use std::hash::Hasher as _;

    /// The sink as it was before the buffer: every write straight into the
    /// stream. The reference the buffered one is held to. It also keeps
    /// the bytes, for the one-shot digest.
    struct Plain(Xxh3, Vec<u8>);

    impl std::hash::Hasher for Plain {
        fn finish(&self) -> u64 {
            self.0.digest()
        }
        fn write(&mut self, bytes: &[u8]) {
            self.0.update(bytes);
            self.1.extend_from_slice(bytes);
        }
    }

    /// One write as the snapshot hash makes them, through the typed door
    /// (inherent methods and `std::hash::Hasher`'s) on the buffered sink
    /// and as the bytes the plain one was given.
    fn write_one(rng: &mut rand_chacha::ChaCha8Rng, buffered: &mut Hasher, plain: &mut Plain) {
        let v = rng.next_u64();
        match rng.next_u32() % 17 {
            16 => {
                let words: [u64; 16] = std::array::from_fn(|_| rng.next_u64());
                buffered.words(&words);
                std::hash::Hash::hash(&words, plain);
            }
            0 => {
                buffered.u8(v as u8);
                plain.write(&[v as u8]);
            }
            1 => {
                buffered.i8(v as i8);
                plain.write(&(v as i8).to_le_bytes());
            }
            2 => {
                buffered.u16(v as u16);
                plain.write(&(v as u16).to_le_bytes());
            }
            3 => {
                buffered.i16(v as i16);
                plain.write(&(v as i16).to_le_bytes());
            }
            4 => {
                buffered.u32(v as u32);
                plain.write(&(v as u32).to_le_bytes());
            }
            5 => {
                buffered.i32(v as i32);
                plain.write(&(v as i32).to_le_bytes());
            }
            6 => {
                buffered.u64(v);
                plain.write(&v.to_le_bytes());
            }
            7 => {
                let w = u128::from(v) << 64 | u128::from(rng.next_u64());
                buffered.u128(w);
                plain.write(&w.to_le_bytes());
            }
            8 => {
                buffered.usize(v as usize);
                plain.write(&v.to_le_bytes());
            }
            9 => {
                buffered.boolean(v & 1 == 1);
                plain.write(&[u8::from(v & 1 == 1)]);
            }
            10 => {
                let x = (v & 1 == 1).then_some(v as u32);
                buffered.option_u32(x);
                match x {
                    Some(x) => {
                        plain.write(&[1]);
                        plain.write(&x.to_le_bytes());
                    }
                    None => plain.write(&[0]),
                }
            }
            11 => {
                std::hash::Hasher::write_i64(buffered, v as i64);
                plain.write(&v.to_le_bytes());
            }
            12 => {
                std::hash::Hasher::write_isize(buffered, v as isize);
                plain.write(&v.to_le_bytes());
            }
            13 => {
                let w = i128::from(v as i64);
                std::hash::Hasher::write_i128(buffered, w);
                plain.write(&w.to_le_bytes());
            }
            _ => {
                // Slices of every length around the buffer's edges: empty,
                // short, exactly half, just over half, the whole buffer and
                // well past it.
                let len = match rng.next_u32() % 6 {
                    0 => 0,
                    1 => (v % 40) as usize,
                    2 => BUF / 2,
                    3 => BUF / 2 + 1,
                    4 => BUF,
                    _ => (v % 1500) as usize,
                };
                let bytes: Vec<u8> = (0..len).map(|_| rng.next_u32() as u8).collect();
                std::hash::Hasher::write(buffered, &bytes);
                plain.write(&bytes);
            }
        }
    }

    #[test]
    fn the_buffered_sink_digests_exactly_what_the_plain_one_did() {
        for seed in 0..400 {
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
            let mut buffered = Hasher::new();
            let mut plain = Plain(Xxh3::new(), Vec::new());
            for _ in 0..rng.next_u32() % 600 {
                write_one(&mut rng, &mut buffered, &mut plain);
            }
            let digest = plain.finish();
            assert_eq!(
                std::hash::Hasher::finish(&buffered),
                digest,
                "seed {seed}: the trait's digest mid-stream"
            );
            assert_eq!(buffered.finish(), digest, "seed {seed}");
            assert_eq!(
                xxhash_rust::xxh3::xxh3_64(&plain.1),
                digest,
                "seed {seed}: and the one-shot digest of the same bytes"
            );
        }
    }
}
