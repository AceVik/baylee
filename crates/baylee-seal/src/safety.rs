//! Safety numbers (§4.4), after Signal's numeric fingerprint, version 0.
//!
//! Each account's half is 30 digits over the sorted identity keys of all its
//! devices and its tag; the pair's number is the two halves, the smaller
//! tag's first, so both ends read the same 60 digits.

use std::cmp::Ordering;
use std::fmt::Write as _;

use sha2::{Digest, Sha512};

use crate::KEY_LEN;

/// How many times the hash is applied after the first.
const ITERATIONS: usize = 5199;

/// The order of two tags: by length, then by their text. Tags are lower-case
/// hex without a sign, so this is their numeric order; it is also the order
/// a whisper's conversation id names its two tags in.
#[must_use]
pub fn tag_order(a: &str, b: &str) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

/// One account's 30 digits:
/// `h = SHA-512(u16be(0) ‖ iks ‖ tag)`, then 5199 times
/// `h = SHA-512(h ‖ iks)`, where `iks` is the identity keys of all the
/// account's devices sorted bytewise and concatenated. Each of the first six
/// 5-byte groups of `h`, read big-endian, gives five digits (mod 100 000).
/// The order the keys are given in doesn't matter.
#[must_use]
pub fn fingerprint(tag: &str, iks: &[[u8; KEY_LEN]]) -> String {
    let mut sorted = iks.to_vec();
    sorted.sort_unstable();
    let joined = sorted.concat();
    let mut h = Sha512::new()
        .chain_update(0u16.to_be_bytes())
        .chain_update(&joined)
        .chain_update(tag.as_bytes())
        .finalize();
    for _ in 0..ITERATIONS {
        h = Sha512::new()
            .chain_update(h)
            .chain_update(&joined)
            .finalize();
    }
    let mut out = String::with_capacity(30);
    for chunk in h[..30].as_chunks::<5>().0 {
        let value = chunk
            .iter()
            .fold(0u64, |acc, byte| acc << 8 | u64::from(*byte));
        let _ = write!(out, "{:05}", value % 100_000);
    }
    out
}

/// The 60-digit safety number of two accounts, each named by its tag and the
/// identity keys of its devices: the [`fingerprint`] of the account whose
/// tag comes first in [`tag_order`], then the other's. The same whichever
/// account is `a`.
#[must_use]
pub fn safety_number(a: (&str, &[[u8; KEY_LEN]]), b: (&str, &[[u8; KEY_LEN]])) -> String {
    let (first, second) = if tag_order(a.0, b.0) == Ordering::Greater {
        (b, a)
    } else {
        (a, b)
    };
    let mut out = fingerprint(first.0, first.1);
    out.push_str(&fingerprint(second.0, second.1));
    out
}
