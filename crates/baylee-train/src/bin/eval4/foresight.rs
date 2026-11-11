//! `--house-foresight`: the house's chairs try their options in forks of
//! the game before they answer (`Session::set_foresight`,
//! `docs/house-ai.md` §"Foresight"), and what that cost is summed over the
//! run for the summary.
//!
//! Off without the flag: a run that seats the house as the net's opponent
//! or benchmark plays it as a hosted table does. With `--random-policy` the
//! net's own game is noise nobody reads, so only the house-against-house
//! game searches.

use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use baylee_gamehost::{ForesightStats, Session};
use serde_json::{Value, json};

/// Whether the run's house chairs search.
static ON: AtomicBool = AtomicBool::new(false);
/// Whether the net's own games are left unsearched (`--random-policy`).
static HOUSE_GAME_ONLY: AtomicBool = AtomicBool::new(false);
/// `--foresight-work`: the cap on one searched answer's work, 0 for none.
static WORK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Every table's searches, summed; the times of all searched answers.
static TOTAL: Mutex<Option<ForesightStats>> = Mutex::new(None);

/// Sets the run's switch once, before any table is played.
pub(crate) fn configure(on: bool, house_game_only: bool, work: u64) {
    WORK.store(work, Ordering::Relaxed);
    ON.store(on, Ordering::Relaxed);
    HOUSE_GAME_ONLY.store(house_game_only, Ordering::Relaxed);
}

/// A table whose searches are added to the run's total when it is put away.
pub(crate) struct Table(Session);

impl Table {
    /// `session`, with foresight switched as the run has it; `net_game`
    /// says the net sits at it.
    pub(crate) fn new(mut session: Session, net_game: bool) -> Self {
        let on =
            ON.load(Ordering::Relaxed) && !(net_game && HOUSE_GAME_ONLY.load(Ordering::Relaxed));
        session.set_foresight(on);
        let work = WORK.load(Ordering::Relaxed);
        session.set_foresight_work((work > 0).then_some(work));
        Self(session)
    }
}

impl Deref for Table {
    type Target = Session;
    fn deref(&self) -> &Session {
        &self.0
    }
}

impl DerefMut for Table {
    fn deref_mut(&mut self) -> &mut Session {
        &mut self.0
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        let stats = self.0.foresight_stats();
        if stats.asked == 0 {
            return;
        }
        let mut total = TOTAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let total = total.get_or_insert_with(ForesightStats::default);
        total.asked += stats.asked;
        total.searched += stats.searched;
        total.replaced += stats.replaced;
        for (reason, n) in &stats.refusals {
            *total.refusals.entry(reason.clone()).or_default() += n;
        }
        for (sum, kind) in total.by_kind.iter_mut().zip(stats.by_kind) {
            sum.0 += kind.0;
            sum.1 += kind.1;
        }
        total.refused += stats.refused;
        total.rollouts += stats.rollouts;
        total.answers += stats.answers;
        total.micros.extend_from_slice(&stats.micros);
    }
}

/// The run's searches for the summary: counts, and the wall time of a
/// searched answer at the median, the 95th percentile and the worst, in
/// milliseconds. `null` where nothing searched.
pub(crate) fn summary(games: u64) -> Value {
    let mut total = TOTAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(total) = total.as_mut() else {
        return Value::Null;
    };
    total.micros.sort_unstable();
    let at = |q: f64| {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let index = ((total.micros.len().saturating_sub(1)) as f64 * q) as usize;
        total
            .micros
            .get(index)
            .map_or(0.0, |micros| f64::from(*micros) / 1000.0)
    };
    let seconds: f64 = total.micros.iter().map(|m| f64::from(*m) / 1e6).sum();
    #[allow(clippy::cast_precision_loss)]
    let table_count = games.max(1) as f64;
    #[allow(clippy::cast_precision_loss)]
    let per_game = |n: u64| n as f64 / table_count;
    json!({
        "asked": total.asked,
        "searched": total.searched,
        "replaced": total.replaced,
        "priority_attack_block": total.by_kind,
        "refusals": total.refusals,
        "refused": total.refused,
        "rollouts": total.rollouts,
        "rollout_answers": total.answers,
        "searched_per_game": per_game(total.searched),
        "search_cpu_seconds": seconds,
        "search_cpu_seconds_per_game": seconds / table_count,
        "ms_p50": at(0.5),
        "ms_p95": at(0.95),
        "ms_max": at(1.0),
    })
}

/// `--leaf-log`: where the positions a leaf value is fitted on are written.
static LEAF_LOG: Mutex<Option<std::io::BufWriter<std::fs::File>>> = Mutex::new(None);

/// Opens the leaf log; every house-against-house table then writes to it.
pub(crate) fn open_leaf_log(path: &std::path::Path) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    *LEAF_LOG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(std::io::BufWriter::new(file));
    Ok(())
}

/// One table's positions, kept until the game says who won.
#[derive(Default)]
pub(crate) struct LeafTrace {
    turn: u32,
    rows: Vec<(u8, u32, [f64; baylee_ai::foresight::FEATURES])>,
}

impl LeafTrace {
    /// Notes every house chair's features once at the start of each turn.
    pub(crate) fn note(&mut self, session: &Session, seats: usize) {
        let turn = session.state().turn.number;
        if turn == self.turn {
            return;
        }
        self.turn = turn;
        for seat in 0..seats {
            #[allow(clippy::cast_possible_truncation)]
            let player = baylee_core::ids::PlayerId::new(seat as u8);
            if let Some(baylee_gamehost::SeatKind::Ai(agent)) = session.seat_kind(player) {
                let features = agent.foresight_features(&session.observe_view(player));
                self.rows.push((player.get(), turn, features));
            }
        }
    }

    /// Writes the noted positions with the result: 1 won, 0 lost, 0.5 drawn.
    pub(crate) fn finish(&self, winners: &[u8]) {
        use std::io::Write as _;
        let mut log = LEAF_LOG
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(log) = log.as_mut() else {
            return;
        };
        for (seat, turn, features) in &self.rows {
            let won = if winners.is_empty() {
                0.5
            } else if winners.contains(seat) {
                1.0
            } else {
                0.0
            };
            let _ = writeln!(log, "{}", json!({"y": won, "turn": turn, "f": features}));
        }
    }
}

/// Flushes the leaf log at the end of a run.
pub(crate) fn close_leaf_log() {
    use std::io::Write as _;
    if let Some(log) = LEAF_LOG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_mut()
    {
        let _ = log.flush();
    }
}
