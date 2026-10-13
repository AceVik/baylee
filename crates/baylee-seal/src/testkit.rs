//! Seeded keys for tests on both sides of the wire: this crate's own
//! known-answer tests, the client core's conversations through a fake
//! gateway, and the gateway's end-to-end tests (device registration and
//! reports need real signatures).
//!
//! **Never for real keys.** [`Seeded`] is a deterministic stream: anyone
//! who knows the seed knows every key drawn from it. A real device draws
//! from the operating system.

use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};

use crate::{DevicePublic, DeviceSecrets, Entropy};

/// `ChaCha20` from a fixed seed, as [`Entropy`]. Tests only.
pub struct Seeded(ChaCha20Rng);

impl Seeded {
    /// The stream for `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(ChaCha20Rng::seed_from_u64(seed))
    }
}

impl Entropy for Seeded {
    fn fill(&mut self, out: &mut [u8]) {
        self.0.fill_bytes(out);
    }
}

/// A device's secrets drawn from the stream for `seed`. The same seed gives
/// the same device every time.
#[must_use]
pub fn device(seed: u64) -> DeviceSecrets {
    DeviceSecrets::generate(&mut Seeded::new(seed))
}

/// A device drawn from `seed`, with what it publishes for `tag`.
///
/// # Panics
///
/// When `tag` is longer than [`crate::MAX_TAG_LEN`] bytes.
#[must_use]
pub fn registered(tag: &str, seed: u64) -> (DeviceSecrets, DevicePublic) {
    let secrets = device(seed);
    let public = secrets.public(tag).expect("a tag of at most 255 bytes");
    (secrets, public)
}
