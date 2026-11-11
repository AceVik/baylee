//! Counting what a search through copies of the game costs, so that a host
//! can bound it.
//!
//! The combat search visits blocking-tree nodes ([`crate::search`]), up to a
//! quarter of a million for one declaration at the deepest level. One answer
//! is bounded by the level's node budget; a host that asks for thousands of
//! answers in copies of a game (`baylee-gamehost`'s foresight) needs the sum
//! bounded as well, and counts its own steps into the same sum. A
//! [`WorkMeter`] is that sum and its bound: every agent counts the nodes it
//! visits into its meter and asks it how many it may still visit, and a host
//! hands the copies it seats one meter between them.
//!
//! A count, never a clock: the same questions asked in the same order spend
//! the same work on any machine, so a bound on it changes no answer from one
//! machine to the next. An agent a host has not given a meter has one of its
//! own without a bound, which changes nothing.

use std::sync::atomic::{AtomicU64, Ordering};

/// The work the holders of this meter have spent, and what they may spend.
///
/// The unit is one node of the combat search; a host prices its own steps in
/// the same unit ([`Self::spend`]).
#[derive(Debug)]
pub struct WorkMeter {
    spent: AtomicU64,
    limit: u64,
    /// The most nodes one combat search may visit, whatever is left.
    search: u32,
}

impl Default for WorkMeter {
    fn default() -> Self {
        Self::bounded(u64::MAX, u32::MAX)
    }
}

impl WorkMeter {
    /// A meter that lets its holders spend `limit` units between them and
    /// no more, and one combat search at most `search` nodes of them.
    #[must_use]
    pub const fn bounded(limit: u64, search: u32) -> Self {
        Self {
            spent: AtomicU64::new(0),
            limit,
            search,
        }
    }

    /// The units spent so far, by whoever spent them.
    #[must_use]
    pub fn spent(&self) -> u64 {
        self.spent.load(Ordering::Relaxed)
    }

    /// The units still to spend.
    #[must_use]
    pub fn left(&self) -> u64 {
        self.limit.saturating_sub(self.spent())
    }

    /// Whether the bound is reached.
    #[must_use]
    pub fn exhausted(&self) -> bool {
        self.left() == 0
    }

    /// Counts `units` as spent. Spending past the bound is counted too: the
    /// bound is what the meter's readers stop at, and one step of theirs may
    /// run over it by what that step costs.
    pub fn spend(&self, units: u64) {
        self.spent.fetch_add(units, Ordering::Relaxed);
    }

    /// How many of the `wanted` nodes one combat search may visit: no more
    /// than one search's share, and no more than is left.
    #[must_use]
    pub fn nodes(&self, wanted: u32) -> u32 {
        let wanted = wanted.min(self.search);
        u32::try_from(self.left()).map_or(wanted, |left| left.min(wanted))
    }
}

#[cfg(test)]
mod tests {
    use super::WorkMeter;

    /// A meter nobody bounded lets every search have its whole budget, and
    /// a bounded one hands out one search's share of what is left and then
    /// nothing.
    #[test]
    fn a_meter_hands_out_what_is_left() {
        let free = WorkMeter::default();
        free.spend(1 << 40);
        assert_eq!(free.nodes(262_144), 262_144);
        assert!(!free.exhausted());
        let meter = WorkMeter::bounded(1000, 300);
        assert_eq!(meter.nodes(262_144), 300, "one search's share");
        assert_eq!(meter.nodes(200), 200);
        meter.spend(800);
        assert_eq!(
            (meter.spent(), meter.left(), meter.nodes(262_144)),
            (800, 200, 200),
            "what is left"
        );
        meter.spend(700);
        assert_eq!(
            (meter.spent(), meter.left(), meter.nodes(500)),
            (1500, 0, 0)
        );
        assert!(meter.exhausted());
    }
}
