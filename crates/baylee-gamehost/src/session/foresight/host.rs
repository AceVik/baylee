//! What a host lets the house's searches spend beyond the count each level
//! bounds itself to (`docs/house-ai.md` §"Foresight at a host").
//!
//! A level's bound is a count of work (`Foresight::work`), so a search answers
//! alike wherever it runs. A host has three things a count does not see: how
//! many of its tables search at once, how loaded its cores are, and a profile
//! from a generation that bounds nothing itself. [`HostForesight`] holds its
//! answer to each:
//!
//! - **a share and a ceiling** of the work: counts again, so a table under
//!   them answers alike on every machine, as a level does under its own;
//! - **permits** ([`SearchPermits`]): a search that finds none free is not
//!   made and the level's own answer stands; nothing waits for one, so no
//!   queue grows;
//! - **a wall guard** ([`WallGuard`]): a search still running when the host's
//!   clock says it has taken too long is given up and the level's own answer
//!   stands.
//!
//! The last two read the host and not the game, so the same position may be
//! answered with or without a search. That is safe for the record: a house
//! answer is an input the record holds, and a replay or a resumed table
//! applies the recorded answers without asking the house again
//! (`session/resume.rs`, `record::replay`).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// A host's bounds on its tables' house searches
/// (`Session::set_foresight_host`). The default bounds nothing beyond each
/// level's own count, which is how a harness plays.
#[derive(Clone)]
pub struct HostForesight {
    /// The share of each level's own bound (`Foresight::work`) one search may
    /// spend, in percent. A host only ever lowers a level's bound: above 100
    /// counts as 100, and 0 as 1.
    pub percent: u8,
    /// The most work any search may spend at this host, whatever its level
    /// says: a level from before `AIProfile::BOUNDED_SEARCH` bounds nothing
    /// itself. `u64::MAX` for none.
    pub ceiling: u64,
    /// Gives a search up once it has taken too long by the host's clock.
    pub wall: Option<WallGuard>,
    /// The searches the host lets run at once, across every table that
    /// shares the pool; `None` for no limit.
    pub permits: Option<Arc<dyn SearchPermits>>,
    /// Ends a pump slice after each searched answer, so that the host's
    /// event loop reads its sockets between two searches.
    pub yields: bool,
}

impl Default for HostForesight {
    fn default() -> Self {
        Self {
            percent: 100,
            ceiling: u64::MAX,
            wall: None,
            permits: None,
            yields: false,
        }
    }
}

impl HostForesight {
    /// `work`, a level's bound already held to the table's, as this host lets
    /// one search spend it: no more than the ceiling, and its share of that.
    /// Unbounded stays unbounded.
    #[must_use]
    pub fn work(&self, work: u64) -> u64 {
        let work = work.min(self.ceiling);
        if work == u64::MAX {
            return work;
        }
        let percent = u128::from(self.percent.clamp(1, 100));
        u64::try_from(u128::from(work) * percent / 100).unwrap_or(work)
    }
}

/// The host's guard on the wall time of one search.
///
/// The level's bound is in units of work, which a loaded host stretches: a
/// search of a second's work may take many seconds of wall time when it waits
/// for a core. The guard reads the host's clock where a search reads its
/// meter (before each fork, after each answer in one), and a search past
/// `limit` is given up there: the level's own answer is given, the table
/// counts it (`ForesightStats::walled`) and says so in its log. One step of
/// the engine is not interrupted, so a search may run past `limit` by the
/// step it was in.
#[derive(Clone, Copy, Debug)]
pub struct WallGuard {
    /// The host's monotonic clock: the time since any fixed origin.
    pub clock: fn() -> Duration,
    /// How long one search may take on it.
    pub limit: Duration,
}

/// Where a host's searches get leave to run: at most so many at once.
///
/// A search asks once, before it makes its first fork, and holds what it is
/// given until it has answered. One that is given nothing is not made: the
/// level's own answer stands (`ForesightStats::busy`). Never blocks.
pub trait SearchPermits: Send + Sync {
    /// Leave for one search, or `None` when the host has none to spare now.
    fn try_acquire(&self) -> Option<SearchPermit>;
}

/// Leave for one search ([`SearchPermits`]): given back when dropped.
pub struct SearchPermit {
    _held: Box<dyn Send>,
}

impl SearchPermit {
    /// A permit that gives `held` back when it is dropped: whatever the pool
    /// releases on drop (a count, a lock on a file).
    #[must_use]
    pub fn new(held: impl Send + 'static) -> Self {
        Self {
            _held: Box::new(held),
        }
    }
}

/// A pool of permits in this process: at most `limit` searches at once among
/// the tables that share it.
///
/// For a process that hosts several tables (the client's own table, a test);
/// an engine server hosts one game and shares a pool with the other engines
/// on its machine instead (`baylee-engine-server`'s `slots`).
#[derive(Debug)]
pub struct Searches {
    running: Arc<AtomicUsize>,
    limit: usize,
}

impl Searches {
    /// A pool that lets `limit` searches run at once; `0` lets none.
    #[must_use]
    pub fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            running: Arc::new(AtomicUsize::new(0)),
            limit,
        })
    }

    /// How many searches hold a permit now.
    #[must_use]
    pub fn running(&self) -> usize {
        self.running.load(Ordering::Acquire)
    }
}

/// A permit of [`Searches`]: one search fewer running when dropped.
struct Running(Arc<AtomicUsize>);

impl Drop for Running {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

impl SearchPermits for Searches {
    fn try_acquire(&self) -> Option<SearchPermit> {
        self.running
            .try_update(Ordering::AcqRel, Ordering::Acquire, |running| {
                (running < self.limit).then_some(running + 1)
            })
            .ok()?;
        Some(SearchPermit::new(Running(Arc::clone(&self.running))))
    }
}

#[cfg(test)]
mod tests {
    use super::{HostForesight, SearchPermits, Searches};

    /// A pool hands out as many permits as it holds and then none, takes
    /// each back when it is dropped, and a pool of none never gives one.
    #[test]
    fn a_pool_lends_what_it_holds_and_no_more() {
        let pool = Searches::new(2);
        let first = pool.try_acquire().expect("one");
        let second = pool.try_acquire().expect("two");
        assert!(pool.try_acquire().is_none(), "a third while two run");
        assert_eq!(pool.running(), 2);
        drop(first);
        assert_eq!(pool.running(), 1);
        let third = pool.try_acquire().expect("one was given back");
        drop((second, third));
        assert_eq!(pool.running(), 0);
        assert!(Searches::new(0).try_acquire().is_none());
    }

    /// Threads racing for one permit never hold two at once.
    #[test]
    fn a_pool_of_one_holds_one_across_threads() {
        let pool = Searches::new(1);
        let most = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let (pool, most) = (&pool, &most);
                scope.spawn(move || {
                    for _ in 0..2_000 {
                        if let Some(permit) = pool.try_acquire() {
                            most.fetch_max(pool.running(), std::sync::atomic::Ordering::AcqRel);
                            drop(permit);
                        }
                    }
                });
            }
        });
        assert_eq!(most.load(std::sync::atomic::Ordering::Acquire), 1);
        assert_eq!(pool.running(), 0);
    }

    /// The share and the ceiling only ever lower a bound; an unbounded level
    /// is bounded by the ceiling alone, and a share outside 1 to 100 is held
    /// to it.
    #[test]
    fn a_host_share_and_ceiling_only_lower_a_bound() {
        let host = |percent, ceiling| HostForesight {
            percent,
            ceiling,
            ..HostForesight::default()
        };
        assert_eq!(HostForesight::default().work(8_000_000), 8_000_000);
        assert_eq!(HostForesight::default().work(u64::MAX), u64::MAX);
        assert_eq!(host(50, u64::MAX).work(8_000_000), 4_000_000);
        assert_eq!(host(100, 8_000_000).work(u64::MAX), 8_000_000);
        assert_eq!(host(50, 8_000_000).work(u64::MAX), 4_000_000);
        assert_eq!(host(100, 8_000_000).work(1_333_333), 1_333_333);
        assert_eq!(
            host(255, u64::MAX).work(8_000_000),
            8_000_000,
            "never raised"
        );
        assert_eq!(host(0, u64::MAX).work(8_000_000), 80_000, "0 counts as 1");
    }
}
