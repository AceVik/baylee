//! Seeded, reproducible randomness.
//!
//! Every random decision in a game (shuffles, coin flips, dice for custom
//! modes) goes through [`GameRng`] — a `ChaCha8` stream seeded from the
//! preset. Same seed + same action sequence ⇒ identical stream, which is
//! what makes replays and determinism tests possible.
//!
//! One exception: a mulligan's shuffle runs on the seat's own stream
//! ([`GameRng::for_seat`]), because the mulligans are answered in whatever
//! order they arrive; see `docs/engine-internals.md` §"The replay contract".

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};

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

    /// The stream `seat` shuffles its opening mulligans with: the same seed
    /// on `ChaCha8` stream `seat + 1`, the table's being stream 0.
    ///
    /// The mulligans are taken all at once (`engine::mulligan`), so on the
    /// table's stream a seat's new hand would depend on how many shuffles
    /// the other seats had asked for before it, which is to say on who
    /// clicked first. On its own stream it depends only on its own answers,
    /// and the table's stream leaves the window where it entered it however
    /// many mulligans were taken.
    #[must_use]
    pub fn for_seat(&self, seat: u8) -> Self {
        let mut rng = ChaCha8Rng::from_seed(self.rng.get_seed());
        rng.set_stream(u64::from(seat) + 1);
        Self { rng, calls: 0 }
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

/// Written out because `ChaCha8Rng` has no `Hash` of its own: the seed, the
/// stream and the position in it are the whole of what it draws next.
/// Every field is named, so a new one does not compile until it is hashed
/// or left out on purpose (#122).
impl std::hash::Hash for GameRng {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self { rng, calls } = self;
        rng.get_seed().hash(state);
        rng.get_stream().hash(state);
        rng.get_word_pos().hash(state);
        calls.hash(state);
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

    /// A seat's stream is made of the seed and the seat, not of where the
    /// table's stream stands: drawing from the table first changes nothing
    /// about it, and no two streams at the table are the same.
    #[test]
    fn a_seats_stream_depends_on_the_seed_and_the_seat_only() {
        let draws = |mut rng: GameRng| -> Vec<u64> { (0..5).map(|_| rng.next_u64()).collect() };
        let table = GameRng::new(9);
        let mut drawn = GameRng::new(9);
        for _ in 0..17 {
            drawn.next_u64();
        }
        let seat = draws(table.for_seat(1));
        assert_eq!(seat, draws(drawn.for_seat(1)));
        for other in [
            table.clone(),
            table.for_seat(0),
            table.for_seat(2),
            GameRng::new(10).for_seat(1),
        ] {
            assert_ne!(seat, draws(other));
        }
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
    /// not hold enough. This settles the first with the deck **as it stood**:
    /// an opening seven out of 99 cards, 31 of them lands, holds 2.19 lands on
    /// average, which is what that ratio says and nothing to do with the
    /// shuffle. The numbers stay 99/31 after the deck was fixed, because they
    /// are the measurement that acquitted the shuffle — not a copy of a deck
    /// list that will drift.
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

    /// Every draw is counted and moves the stream, whichever door it came
    /// through: `calls()` is what a journal cross-checks a replay against and
    /// `word_pos()` is what a snapshot hash reads, so a door that drew
    /// without counting would make a replay that diverges look faithful. A
    /// shuffle draws once per swap, so a deck of *n* costs `n - 1`.
    ///
    /// The empty and one-card cases are the ones worth pinning. One stream
    /// serves the whole table, so a shuffle of nothing that drew anyway would
    /// move it for everybody — a player with an empty library would change
    /// what their opponent draws.
    #[test]
    fn every_draw_is_counted_and_moves_the_stream() {
        let mut rng = GameRng::new(11);
        assert_eq!(rng.calls(), 0);
        let before = rng.word_pos();
        rng.next_u64();
        assert_eq!(rng.calls(), 1);
        assert!(rng.word_pos() > before, "a draw moves the stream position");
        rng.below(10);
        rng.roll(20);
        assert_eq!(rng.calls(), 3, "below and roll are draws like any other");

        for len in [0_usize, 1, 2, 7, 60] {
            let mut rng = GameRng::new(11);
            let mut deck: Vec<usize> = (0..len).collect();
            rng.shuffle(&mut deck);
            assert_eq!(
                rng.calls(),
                u64::try_from(len.saturating_sub(1)).expect("a small deck"),
                "a shuffle of {len} cards is one draw per swap"
            );
            let mut back = deck.clone();
            back.sort_unstable();
            assert_eq!(
                back,
                (0..len).collect::<Vec<_>>(),
                "and it is a permutation: no card lost, none dealt twice"
            );
        }

        let mut idle = GameRng::new(11);
        idle.shuffle(&mut Vec::<u8>::new());
        assert_eq!(
            idle.word_pos(),
            GameRng::new(11).word_pos(),
            "shuffling nothing leaves the stream where every other seat \
             expects to find it"
        );
    }

    /// `below` is exclusive on its bound and free of modulo bias: the
    /// multiply-high keeps the top 64 bits of a 128-bit product, so `n`
    /// itself is unreachable by construction rather than by a rejection
    /// loop, and `below(1)` is a constant 0 that costs one draw.
    #[test]
    fn a_bounded_draw_stays_under_its_bound_and_reaches_all_of_it() {
        let mut rng = GameRng::new(3);
        for _ in 0..500 {
            assert_eq!(rng.below(1), 0, "the only value under one");
        }

        let mut coin = [0_u32; 2];
        for _ in 0..2_000 {
            let v = rng.below(2);
            coin[usize::try_from(v).expect("under two")] += 1;
        }
        assert!(
            coin.iter().all(|&n| n > 800),
            "both halves come up, and neither is the whole coin: {coin:?}"
        );

        let mut top = 0_u64;
        for _ in 0..2_000 {
            let v = rng.below(u64::MAX);
            assert!(v < u64::MAX, "the bound itself is never handed out");
            top = top.max(v);
        }
        assert!(
            top > u64::MAX / 2,
            "and the whole range is reachable, not just its floor"
        );
    }

    /// Zero is refused rather than wrapped. A `below(0)` is a caller asking
    /// for a card out of an empty zone, and answering it with 0 would hand
    /// back an index into nothing.
    #[test]
    #[should_panic(expected = "below(0) is meaningless")]
    fn a_draw_below_nothing_is_refused() {
        let mut rng = GameRng::new(1);
        let _ = rng.below(0);
    }

    /// A clone is the same stream from the same place, which is what lets a
    /// game position be snapshot and played on from. The seed does **not**
    /// move, so a hash reading only `seed()` would call two different
    /// positions of one game identical — `word_pos()` is the half that says
    /// how far along it is, and both are read for that reason.
    #[test]
    fn a_clone_is_the_same_stream_from_the_same_place() {
        let mut rng = GameRng::new(99);
        for _ in 0..17 {
            rng.next_u64();
        }
        let mut copy = rng.clone();
        assert_eq!(copy.word_pos(), rng.word_pos());
        assert_eq!(copy.calls(), rng.calls(), "and the count travels with it");
        let taken: Vec<u64> = (0..8).map(|_| rng.next_u64()).collect();
        let again: Vec<u64> = (0..8).map(|_| copy.next_u64()).collect();
        assert_eq!(taken, again);

        let fresh = GameRng::new(99);
        assert_eq!(
            fresh.seed(),
            rng.seed(),
            "the seed belongs to the game, not to the position"
        );
        assert_ne!(
            fresh.word_pos(),
            rng.word_pos(),
            "the position is the half that moved"
        );
        assert_ne!(
            GameRng::new(98).seed(),
            fresh.seed(),
            "and another game's seed is another stream"
        );
    }
}
