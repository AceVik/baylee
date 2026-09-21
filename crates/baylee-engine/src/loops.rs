//! Detecting a game that has started repeating itself.
//!
//! # What counts as an endless loop
//!
//! The engine drives a *decision-free segment*: from the moment nobody is
//! being asked anything until the next choice is produced, it resolves the
//! stack, runs state-based actions and puts triggers on the stack without
//! ever stopping. Almost every segment is a handful of iterations long. Two
//! things can make one run away:
//!
//! - A **large but finite** pile of work — the ally deck's thousand rally
//!   triggers, a storm count, a mill. Every iteration strictly consumes
//!   something, so the situation never repeats. These must never be flagged;
//!   the game is simply busy.
//! - A **real endless loop** — mandatory abilities that recreate each other,
//!   so the game returns to a situation it has already been in and will keep
//!   returning to it. Nothing a player can do will stop it, because nobody is
//!   ever asked.
//!
//! The difference is *repetition*, not iteration count, so that is what this
//! module looks for.
//!
//! # Why not the snapshot hash
//!
//! [`crate::state::GameState::snapshot_hash`] cannot be used here.
//! Object slots are never recycled and timestamps only go up, so a permanent
//! that dies and comes back is a *different* object with a *later* timestamp:
//! a genuine endless loop never hashes the same twice. The detector compares
//! [`crate::state::GameState::loop_signature`] instead, which hashes the
//! rules-visible situation and is blind to identity and time.
//!
//! # How it looks for the repeat
//!
//! Brent's cycle-finding algorithm over the signature stream: keep one saved
//! signature (the tortoise), compare each new one against it, and re-save at
//! exponentially growing distances. That finds a cycle of length `lam` after
//! about `mu + 2·lam` samples with a *single* stored value — no history
//! buffer to bound, and no allocation in the engine's hot path.
//!
//! Two cheap guards keep the cost at zero for real games:
//!
//! - nothing is hashed until a segment has run [`LoopWatch::WATCH_AFTER`]
//!   iterations, which no ordinary segment reaches;
//! - after that, only every [`LoopWatch::SAMPLE_EVERY`]-th iteration is
//!   hashed. Sampling an eventually-periodic sequence leaves it eventually
//!   periodic, so the cycle is still found — just with a coarser period.
//!
//! A single match is not enough to act on: an unlucky sample pair could
//! agree by accident. The detector re-checks one full period later and only
//! reports a loop if the situation repeated again.

/// Watches one decision-free segment for repetition.
///
/// Created fresh per segment; `Default` is a watch that has seen nothing.
#[derive(Clone, Debug, Default)]
pub struct LoopWatch {
    /// Machine iterations seen in this segment.
    steps: u64,
    /// The saved signature Brent compares against.
    tortoise: u64,
    /// Samples taken since `tortoise` was saved.
    lam: u64,
    /// Distance at which `tortoise` is re-saved next.
    power: u64,
    /// Whether `tortoise` holds a signature yet.
    started: bool,
    /// A candidate period awaiting confirmation, with the countdown of
    /// samples until the re-check.
    candidate: Option<(u64, u64)>,
}

impl LoopWatch {
    /// Iterations a segment must run before anything is hashed at all.
    ///
    /// Ordinary segments are a handful of iterations; this is far above
    /// anything a real game produces, so the detector costs nothing until
    /// something has clearly gone wrong.
    pub const WATCH_AFTER: u64 = 4096;

    /// Only every n-th iteration past [`Self::WATCH_AFTER`] is hashed.
    ///
    /// Sampling preserves eventual periodicity, so a loop is still found;
    /// it just costs one signature per 256 iterations instead of one per
    /// iteration, which matters when the segment is a huge finite pile of
    /// work rather than a loop.
    pub const SAMPLE_EVERY: u64 = 256;

    /// Whether the caller should compute a signature for this iteration.
    ///
    /// Kept separate from [`Self::step`] so the engine never pays for a
    /// signature it will not use.
    #[must_use]
    pub const fn wants_sample(&self) -> bool {
        self.steps >= Self::WATCH_AFTER && self.steps.is_multiple_of(Self::SAMPLE_EVERY)
    }

    /// Records one machine iteration, with its signature if one was taken.
    ///
    /// Returns the confirmed period, in samples, when the segment has been
    /// shown to repeat.
    pub fn step(&mut self, signature: Option<u64>) -> Option<u64> {
        self.steps += 1;
        let sig = signature?;

        if !self.started {
            self.started = true;
            self.tortoise = sig;
            self.power = 1;
            self.lam = 0;
            return None;
        }
        self.lam += 1;

        // A candidate period is on probation: it only counts if the same
        // situation comes back again exactly one period later.
        if let Some((period, countdown)) = self.candidate {
            let left = countdown - 1;
            if left == 0 {
                self.candidate = None;
                if sig == self.tortoise {
                    return Some(period);
                }
            } else {
                self.candidate = Some((period, left));
            }
            return None;
        }

        if sig == self.tortoise {
            // First match: put it on probation rather than acting on it.
            self.candidate = Some((self.lam, self.lam));
            return None;
        }
        if self.lam == self.power {
            self.tortoise = sig;
            self.power *= 2;
            self.lam = 0;
        }
        None
    }

    /// Iterations seen so far in this segment.
    #[must_use]
    pub const fn steps(&self) -> u64 {
        self.steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives a watch with `next(i)` as the underlying signature stream and
    /// returns the period it reports, if any, within `limit` iterations.
    fn run(limit: u64, next: impl Fn(u64) -> u64) -> Option<u64> {
        let mut watch = LoopWatch::default();
        for i in 0..limit {
            let sig = watch.wants_sample().then(|| next(i));
            if let Some(period) = watch.step(sig) {
                return Some(period);
            }
        }
        None
    }

    /// A segment that keeps returning to the same situation is a loop.
    #[test]
    fn a_repeating_situation_is_reported() {
        let period = run(200_000, |i| i % 7).expect("a 7-cycle is a loop");
        assert!(period > 0);
    }

    /// A cycle whose length shares no factor with the sampling stride is
    /// still found: sampling an eventually-periodic sequence leaves it
    /// eventually periodic.
    #[test]
    fn sampling_does_not_hide_an_awkward_period() {
        for len in [1_u64, 2, 3, 5, 13, 64, 255, 257] {
            assert!(
                run(2_000_000, |i| i % len).is_some(),
                "a {len}-cycle went unnoticed"
            );
        }
    }

    /// The ally deck's thousand triggers: a huge but finite pile of work
    /// where every iteration consumes something. It must never be called a
    /// loop, however long it runs.
    #[test]
    fn a_large_but_finite_segment_is_never_flagged() {
        assert_eq!(run(1_000_000, |i| i), None);
    }

    /// A segment that wanders back to an earlier situation once, then moves
    /// on, is not a loop — the confirmation pass is what tells them apart.
    #[test]
    fn a_single_coincidence_is_not_a_loop() {
        // Distinct everywhere except one deliberate repeat of the very
        // first sampled value.
        let first_sample = LoopWatch::WATCH_AFTER;
        let repeat_at = first_sample + LoopWatch::SAMPLE_EVERY * 4;
        assert_eq!(
            run(500_000, |i| if i == repeat_at { first_sample } else { i }),
            None
        );
    }

    /// Nothing is hashed until a segment has clearly run away, so ordinary
    /// play never pays for the detector.
    #[test]
    fn short_segments_are_never_sampled() {
        let mut watch = LoopWatch::default();
        for _ in 0..LoopWatch::WATCH_AFTER {
            assert!(!watch.wants_sample(), "a short segment was sampled");
            assert_eq!(watch.step(None), None);
        }
        assert!(watch.wants_sample());
    }

    /// Greatest common divisor, for the sampled period below.
    fn gcd(a: u64, b: u64) -> u64 {
        if b == 0 { a } else { gcd(b, a % b) }
    }

    /// The number handed back is a real period of the stream the detector
    /// saw, not merely a non-zero report. Sampling every 256th iteration
    /// turns a cycle of `len` into a sampled cycle of `len / gcd(len, 256)`,
    /// so a period that is a multiple of that one is a distance the
    /// situation genuinely repeats at — and any other number would be a
    /// report the caller cannot act on.
    #[test]
    fn the_period_it_reports_is_a_distance_the_situation_repeats_at() {
        for len in [1_u64, 2, 3, 5, 7, 13, 64, 255, 257] {
            let sampled = len / gcd(len, LoopWatch::SAMPLE_EVERY);
            let period =
                run(2_000_000, |i| i % len).unwrap_or_else(|| panic!("a {len}-cycle is a loop"));
            assert!(period > 0, "a period of nought is no period");
            assert_eq!(
                period % sampled,
                0,
                "a {len}-cycle sampled every {stride} is a {sampled}-cycle, \
                 and {period} is not a multiple of it",
                stride = LoopWatch::SAMPLE_EVERY
            );
        }
    }

    /// The loop does not have to start at the beginning. A segment that
    /// grinds through a large finite pile and *then* falls into a cycle is
    /// the shape a real one takes — a mill that empties a library and leaves
    /// two abilities recreating each other — and Brent is chosen partly
    /// because it finds a cycle behind any prefix with one stored value.
    #[test]
    fn a_loop_behind_a_long_prefix_is_still_found() {
        const PREFIX: u64 = 400_000;
        for len in [2_u64, 3, 9, 100] {
            let period = run(4_000_000, |i| {
                if i < PREFIX {
                    i
                } else {
                    // Values from a different range, so the cycle cannot be
                    // matched against anything in the prefix by accident.
                    u64::MAX - (i % len)
                }
            });
            assert!(
                period.is_some(),
                "a {len}-cycle behind {PREFIX} iterations of finite work went \
                 unnoticed"
            );
        }
    }

    /// Samples are taken at a fixed stride from one fixed iteration, which
    /// is what makes the cost a constant rather than a share of the work:
    /// one signature per 256 iterations past the threshold, and none at all
    /// before it.
    #[test]
    fn a_sample_is_taken_at_a_fixed_stride_and_not_before() {
        const RUN: u64 = LoopWatch::WATCH_AFTER + LoopWatch::SAMPLE_EVERY * 10;
        let mut watch = LoopWatch::default();
        let mut sampled = Vec::new();
        for i in 0..RUN {
            if watch.wants_sample() {
                sampled.push(i);
            }
            watch.step(None);
        }
        assert_eq!(watch.steps(), RUN, "every iteration is counted");
        let want: Vec<u64> = (0..=10)
            .map(|k| LoopWatch::WATCH_AFTER + LoopWatch::SAMPLE_EVERY * k)
            .take_while(|i| *i < RUN)
            .collect();
        assert_eq!(sampled, want);
        assert_eq!(
            sampled.len(),
            10,
            "ten signatures for {RUN} iterations, and none for the first \
             {threshold}",
            threshold = LoopWatch::WATCH_AFTER
        );
    }

    /// An iteration the engine did not hash leaves the detector exactly
    /// where it was. `wants_sample` and `step` are two calls on purpose, so
    /// a caller that skips the signature must cost the watch nothing but a
    /// count — otherwise a detector fed a short stream would compare a
    /// signature against a hole and call the game repetitive.
    #[test]
    fn an_iteration_that_was_not_hashed_only_counts() {
        let mut watch = LoopWatch::default();
        for _ in 0..100_000 {
            assert_eq!(watch.step(None), None);
        }
        assert_eq!(watch.steps(), 100_000);

        // The very first signature it does see becomes the tortoise, so the
        // *second* identical one is a candidate and the third confirms it —
        // the unhashed hundred thousand did not take that first slot.
        assert_eq!(watch.step(Some(9)), None, "the first is only saved");
        assert_eq!(watch.step(Some(9)), None, "the second is on probation");
        assert_eq!(watch.step(Some(9)), Some(1), "the third confirms a 1-cycle");
    }
}
