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

    /// A shuffle puts every card everywhere, about equally often.
    ///
    /// Determinism was the only thing asserted about the shuffle, and a
    /// *broken* shuffle is deterministic too — the loop that runs `1..len`
    /// instead of `(1..len).rev()`, or `below(len)` instead of `below(i + 1)`,
    /// both pass `shuffle_is_deterministic` and both leave the deck biased.
    /// This is the property that says the algorithm is the right one: over
    /// many deals, the card that started on top finishes in each of the 60
    /// places about as often as any other.
    ///
    /// Bounded on both sides. Buckets that are all *too even* would mean the
    /// stream is not random either.
    #[test]
    fn a_shuffle_puts_every_card_everywhere() {
        const DECK: usize = 60;
        const DEALS: u64 = 60_000;
        /// `DEALS / DECK` — how often each place should come up.
        const WANT: f64 = 1_000.0;
        let mut seen = [0_u32; DECK];
        for seed in 0..DEALS {
            let mut deck: Vec<usize> = (0..DECK).collect();
            GameRng::new(seed).shuffle(&mut deck);
            seen[deck
                .iter()
                .position(|&c| c == 0)
                .expect("the card is in it")] += 1;
        }
        let want = WANT;
        // Chi-square over 59 degrees of freedom: 5% of correct shuffles land
        // above 77.9 and one in a thousand above 103.4, so 120 is a bound a
        // right algorithm passes and the two wrong loops above fail by
        // thousands.
        let chi: f64 = seen
            .iter()
            .map(|&n| {
                let d = f64::from(n) - want;
                d * d / want
            })
            .sum();
        assert!(
            chi < 120.0,
            "the top card lands unevenly: chi-square {chi:.1} over {DEALS} \
             deals, counts {seen:?}"
        );
        assert!(
            chi > 20.0,
            "chi-square {chi:.1} is *too* even for {DEALS} random deals — a \
             stream this regular is not one"
        );
    }

    /// And it deals lands out of a deck the way the deck holds them.
    ///
    /// The complaint this answers is "I never draw lands", and the two
    /// candidate causes are a shuffle that does not mix and a deck that does
    /// not hold enough. This settles the first: an opening seven out of 99
    /// cards, 31 of them lands, holds 2.19 lands on average, which is what
    /// the deck's own ratio says and nothing to do with the shuffle.
    #[test]
    fn an_opening_hand_holds_what_the_deck_holds() {
        const DECK: usize = 99;
        const LANDS: usize = 31;
        const HAND: usize = 7;
        const DEALS: u64 = 20_000;
        /// The same numbers as floats: `HAND * LANDS / DECK`, and `DEALS`.
        const WANT: f64 = 7.0 * 31.0 / 99.0;
        const DEALT: f64 = 20_000.0;
        let mut total = 0_u32;
        let mut landless = 0_u32;
        for seed in 0..DEALS {
            let mut deck: Vec<bool> = (0..DECK).map(|i| i < LANDS).collect();
            GameRng::new(seed ^ 0x9e37_79b9).shuffle(&mut deck);
            let drawn = u32::try_from(deck[..HAND].iter().filter(|&&land| land).count())
                .expect("seven cards");
            total += drawn;
            landless += u32::from(drawn == 0);
        }
        let mean = f64::from(total) / DEALT;
        let want = WANT;
        assert!(
            (mean - want).abs() < 0.05,
            "{mean:.3} lands per opening hand against the {want:.3} the deck \
             holds — the shuffle is not dealing what is in it"
        );
        // The hypergeometric answer is 7.2%, and it is here so the number is
        // written down: a deck this land-light hands out a landless seven
        // about one game in fourteen however well it is shuffled.
        let none = f64::from(landless) * 100.0 / DEALT;
        assert!(
            (5.5..9.0).contains(&none),
            "{none:.1}% of opening hands had no land at all, against the 7.2% \
             the deck's own ratio gives"
        );
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
