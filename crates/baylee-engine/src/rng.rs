//! Seeded, reproducible randomness.
//!
//! Every random decision in a game (shuffles, coin flips, dice for custom
//! modes) goes through [`GameRng`] — a `ChaCha8` stream seeded from the
//! preset. Same seed + same action sequence ⇒ identical stream, which is
//! what makes replays and determinism tests possible.

use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};

/// The game's single source of randomness.
#[derive(Clone, Debug)]
pub struct GameRng {
    rng: ChaCha8Rng,
    calls: u64,
}

impl GameRng {
    /// A stream seeded from the preset's seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            calls: 0,
        }
    }

    /// Raw 64-bit draw.
    pub fn next_u64(&mut self) -> u64 {
        self.calls += 1;
        self.rng.next_u64()
    }

    /// Uniform value in `[0, n)` via Lemire multiply-high (no modulo bias).
    ///
    /// # Panics
    /// When `n == 0`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0, "below(0) is meaningless");
        ((u128::from(self.next_u64()) * u128::from(n)) >> 64) as u64
    }

    /// A die roll in `[1, sides]` (custom modes).
    pub fn roll(&mut self, sides: u32) -> u32 {
        self.below(u64::from(sides)) as u32 + 1
    }

    /// Fisher–Yates shuffle driven by the seeded stream.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.below(i as u64 + 1) as usize;
            slice.swap(i, j);
        }
    }

    /// Number of draws so far (journal cross-checks).
    #[must_use]
    pub fn calls(&self) -> u64 {
        self.calls
    }

    /// The raw seed state (snapshot hashing).
    #[must_use]
    pub fn seed(&self) -> [u8; 32] {
        self.rng.get_seed()
    }

    /// The stream position (snapshot hashing).
    #[must_use]
    pub fn word_pos(&self) -> u128 {
        self.rng.get_word_pos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determinism() {
        let mut rng_a = GameRng::new(42);
        let mut rng_b = GameRng::new(42);
        let mut rng_c = GameRng::new(43);
        for _ in 0..100 {
            assert_eq!(rng_a.next_u64(), rng_b.next_u64());
        }
        // Different seeds produce different streams with overwhelming odds.
        assert_ne!(rng_a.next_u64(), rng_c.next_u64());
    }

    #[test]
    fn shuffle_is_deterministic() {
        let mut deck_a: Vec<u32> = (0..60).collect();
        let mut deck_b = deck_a.clone();
        GameRng::new(7).shuffle(&mut deck_a);
        GameRng::new(7).shuffle(&mut deck_b);
        assert_eq!(deck_a, deck_b);
        assert_ne!(deck_a, (0..60).collect::<Vec<_>>());
    }

    #[test]
    fn dice_range() {
        let mut rng = GameRng::new(1);
        for _ in 0..1000 {
            let d = rng.roll(6);
            assert!((1..=6).contains(&d));
        }
    }
}
