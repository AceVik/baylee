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

use baylee_gamehost::{ForesightStats, SearchCost, Session};
use serde_json::{Value, json};

/// Whether the run's house chairs search.
static ON: AtomicBool = AtomicBool::new(false);
/// Whether the net's own games are left unsearched (`--random-policy`).
static HOUSE_GAME_ONLY: AtomicBool = AtomicBool::new(false);
/// `--foresight-work`: the cap on one searched answer's work, 0 for none.
static WORK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// `--foresight-nodes`: the most nodes of one combat search in a fork under
/// that cap, 0 for the cap's usual share.
static NODES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// Every table's searches, summed; the times of all searched answers.
static TOTAL: Mutex<Option<ForesightStats>> = Mutex::new(None);

/// Sets the run's switch once, before any table is played.
pub(crate) fn configure(on: bool, house_game_only: bool, work: u64, nodes: u32) {
    WORK.store(work, Ordering::Relaxed);
    NODES.store(nodes, Ordering::Relaxed);
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
        let nodes = NODES.load(Ordering::Relaxed);
        session.set_foresight_nodes((nodes > 0).then_some(nodes));
        if on
            && QUESTION_LOG
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_some()
        {
            let kinds = [0, 1, 2].map(|kind| QUESTION_KINDS[kind].load(Ordering::Relaxed));
            session.set_foresight_questions(kinds, QUESTIONS_REPLACED.load(Ordering::Relaxed));
        }
        let every = PROBE_EVERY.load(Ordering::Relaxed);
        session.set_foresight_probe((on && every > 0).then_some(every));
        session.set_foresight_clock(thread_cpu);
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
        write_probe(&self.0.take_foresight_probe());
        write_questions(&self.0.take_foresight_questions());
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
        total.costs.extend_from_slice(&stats.costs);
    }
}

/// The CPU time of the calling thread: what a searched answer is timed by,
/// so that a time is the answer's own work and not that of whatever else
/// the machine runs beside a game. Zero where the clock cannot be read.
fn thread_cpu() -> std::time::Duration {
    let now = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
    std::time::Duration::new(
        u64::try_from(now.tv_sec).unwrap_or(0),
        u32::try_from(now.tv_nsec).unwrap_or(0),
    )
}

/// One searched answer's cost as the summary lists it.
fn cost_json(cost: &SearchCost) -> Value {
    json!({
        "kind": (["priority", "attack", "block"].get(usize::from(cost.kind))),
        "battlefield": cost.battlefield,
        "forks": cost.forks,
        "combat_answers": cost.combat_answers,
        "other_answers": cost.other_answers,
        "nodes": cost.nodes,
        "ms": f64::from(cost.micros) / 1000.0,
        "fork_ms": f64::from(cost.fork_micros) / 1000.0,
        "combat_ms": f64::from(cost.combat_micros) / 1000.0,
        "other_ms": f64::from(cost.other_micros) / 1000.0,
        "apply_ms": f64::from(cost.apply_micros) / 1000.0,
        "leaf_ms": f64::from(cost.leaf_micros) / 1000.0,
        "longest_answer_ms": f64::from(cost.longest_micros) / 1000.0,
        "work": cost.work,
        "board": cost.board,
        "cut": cost.cut,
    })
}

/// The run's searches for the summary: counts, and what a searched answer
/// cost in CPU time of its game's thread at the median, the 95th and 99th
/// percentiles and the worst, in milliseconds, with where the time of all
/// of them and of the slowest hundredth went and the twenty slowest one by
/// one. `null` where nothing searched.
pub(crate) fn summary(games: u64) -> Value {
    let mut total = TOTAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(total) = total.as_mut() else {
        return Value::Null;
    };
    total.costs.sort_unstable_by_key(|cost| cost.micros);
    let costs = &total.costs;
    let index = |q: f64| {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let index = ((costs.len().saturating_sub(1)) as f64 * q) as usize;
        index
    };
    let at = |q: f64| {
        costs
            .get(index(q))
            .map_or(0.0, |cost| f64::from(cost.micros) / 1000.0)
    };
    let split = |costs: &[SearchCost]| {
        let sum = |of: fn(&SearchCost) -> u64| costs.iter().map(of).sum::<u64>();
        #[allow(clippy::cast_precision_loss)]
        let seconds = |of: fn(&SearchCost) -> u64| sum(of) as f64 / 1e6;
        json!({
            "searches": costs.len(),
            "seconds": seconds(|c| u64::from(c.micros)),
            "fork_seconds": seconds(|c| u64::from(c.fork_micros)),
            "combat_seconds": seconds(|c| u64::from(c.combat_micros)),
            "other_seconds": seconds(|c| u64::from(c.other_micros)),
            "apply_seconds": seconds(|c| u64::from(c.apply_micros)),
            "leaf_seconds": seconds(|c| u64::from(c.leaf_micros)),
            "forks": sum(|c| u64::from(c.forks)),
            "combat_answers": sum(|c| u64::from(c.combat_answers)),
            "other_answers": sum(|c| u64::from(c.other_answers)),
            "nodes": sum(|c| c.nodes),
            "work": sum(|c| c.work),
            "board": sum(|c| c.board),
        })
    };
    let seconds: f64 = costs.iter().map(|c| f64::from(c.micros) / 1e6).sum();
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
        "ms_p99": at(0.99),
        "ms_p999": at(0.999),
        "ms_max": at(1.0),
        "cost": split(costs),
        "cost_slowest_hundredth": split(&costs[index(0.99)..]),
        "slowest": costs.iter().rev().take(20).map(cost_json).collect::<Vec<_>>(),
        "cut": costs.iter().filter(|cost| cost.cut).count(),
        "work": work_json(costs),
        "by_board": by_board(costs),
    })
}

/// `--leaf-log`: where the positions a leaf value is fitted on are written.
/// The file and the number of games written to it.
static LEAF_LOG: Mutex<Option<(std::io::BufWriter<std::fs::File>, u64)>> = Mutex::new(None);

/// Opens the leaf log; every house-against-house table then writes to it.
pub(crate) fn open_leaf_log(path: &std::path::Path) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    *LEAF_LOG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some((std::io::BufWriter::new(file), 0));
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

    /// Writes the noted positions with the game's number in the log and the
    /// result: 1 won, 0 lost, 0.5 drawn.
    pub(crate) fn finish(&self, winners: &[u8]) {
        use std::io::Write as _;
        let mut log = LEAF_LOG
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some((log, games)) = log.as_mut() else {
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
            let _ = writeln!(
                log,
                "{}",
                json!({"g": *games, "seat": seat, "y": won, "turn": turn, "f": features})
            );
        }
        *games += 1;
    }
}

/// `--foresight-probe`: where the probed leaves are written, and the number
/// of tables written to it.
static PROBE_LOG: Mutex<Option<(std::io::BufWriter<std::fs::File>, u64)>> = Mutex::new(None);
/// `--foresight-probe-every`, 0 while nothing is probed.
static PROBE_EVERY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Opens the probe log; every searching table then has one searched
/// question in `every` played on from its leaves and writes them here.
pub(crate) fn open_probe_log(path: &std::path::Path, every: u64) -> anyhow::Result<()> {
    anyhow::ensure!(every > 0, "--foresight-probe-every must be at least 1");
    let file = std::fs::File::create(path)?;
    *PROBE_LOG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some((std::io::BufWriter::new(file), 0));
    PROBE_EVERY.store(every, Ordering::Relaxed);
    Ok(())
}

/// Writes a table's probed leaves under the table's number in the log: `g`
/// the table, `r` the question, `w` the world, `c` the answer tried (0 the
/// level's own), `k` the question's kind (0 priority, 1 attack, 2 block),
/// `t` whether the game ended before the leaf, `turn`, `f` the features,
/// `v` the leaf's value and `y` how the game came out.
fn write_probe(leaves: &[baylee_gamehost::ProbedLeaf]) {
    use std::io::Write as _;
    if leaves.is_empty() {
        return;
    }
    let mut log = PROBE_LOG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some((log, tables)) = log.as_mut() else {
        return;
    };
    for leaf in leaves {
        let _ = writeln!(
            log,
            "{}",
            json!({
                "g": *tables,
                "r": leaf.root,
                "w": leaf.world,
                "c": leaf.candidate,
                "k": leaf.kind,
                "t": leaf.ended,
                "turn": leaf.turn,
                "f": leaf.features,
                "v": leaf.value,
                "y": leaf.played,
            })
        );
    }
    *tables += 1;
}

/// Flushes the leaf log and the probe log at the end of a run.
pub(crate) fn close_logs() {
    use std::io::Write as _;
    for log in [&LEAF_LOG, &PROBE_LOG, &QUESTION_LOG] {
        if let Some((log, _)) = log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_mut()
        {
            let _ = log.flush();
        }
    }
}

/// What the searches spent on their meters, and what a unit of it took: the
/// work of a searched answer at the median, the 95th and 99th percentiles
/// and the most, and over the searches of 100,000 units or more the
/// microseconds a unit took at the same points. A bound on the work bounds
/// the time by the last of these.
fn work_json(costs: &[SearchCost]) -> Value {
    let mut work: Vec<u64> = costs.iter().map(|cost| cost.work).collect();
    work.sort_unstable();
    #[allow(clippy::cast_precision_loss)]
    let mut per_unit: Vec<f64> = costs
        .iter()
        .filter(|cost| cost.work >= 100_000)
        .map(|cost| f64::from(cost.micros) / cost.work as f64)
        .collect();
    per_unit.sort_by(f64::total_cmp);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let index = |len: usize, q: f64| (len.saturating_sub(1) as f64 * q) as usize;
    let work_at = |q: f64| work.get(index(work.len(), q)).copied().unwrap_or(0);
    let unit_at = |q: f64| {
        per_unit
            .get(index(per_unit.len(), q))
            .copied()
            .unwrap_or(0.0)
    };
    json!({
        "p50": work_at(0.5),
        "p95": work_at(0.95),
        "p99": work_at(0.99),
        "max": work_at(1.0),
        "micros_per_unit_p50": unit_at(0.5),
        "micros_per_unit_p95": unit_at(0.95),
        "micros_per_unit_p99": unit_at(0.99),
        "micros_per_unit_max": unit_at(1.0),
    })
}

/// What a fork, an answer and a node took by how full the table was: the
/// searches grouped by the permanents and stack objects their forks'
/// answers were given over at the mean, with the microseconds a fork and
/// an answer that is no attack or block took (the engine's step included)
/// and the nanoseconds a combat-search node took. What the prices of a
/// fork and an answer on a search's meter are fitted to.
fn by_board(costs: &[SearchCost]) -> Value {
    const EDGES: [u64; 7] = [0, 8, 16, 24, 32, 48, u64::MAX];
    let rows: Vec<Value> = EDGES
        .windows(2)
        .map(|edge| {
            let of: Vec<&SearchCost> = costs
                .iter()
                .filter(|cost| {
                    let answers = u64::from(cost.combat_answers) + u64::from(cost.other_answers);
                    let board = cost.board / answers.max(1);
                    answers > 0 && board >= edge[0] && board < edge[1]
                })
                .collect();
            let sum = |get: fn(&SearchCost) -> u64| of.iter().map(|cost| get(cost)).sum::<u64>();
            #[allow(clippy::cast_precision_loss)]
            let ratio = |over: u64, under: u64| over as f64 / under.max(1) as f64;
            json!({
                "board_from": edge[0],
                "searches": of.len(),
                "fork_micros": ratio(sum(|c| u64::from(c.fork_micros)), sum(|c| u64::from(c.forks))),
                "answer_micros": ratio(
                    sum(|c| u64::from(c.other_micros) + u64::from(c.apply_micros)),
                    sum(|c| u64::from(c.other_answers) + u64::from(c.combat_answers)),
                ),
                "combat_answer_micros": ratio(
                    sum(|c| u64::from(c.combat_micros)),
                    sum(|c| u64::from(c.combat_answers)),
                ),
                "node_nanos": ratio(sum(|c| u64::from(c.combat_micros)) * 1000, sum(|c| c.nodes)),
                "longest_answer_ms": of
                    .iter()
                    .map(|cost| cost.longest_micros)
                    .max()
                    .map_or(0.0, |micros| f64::from(micros) / 1000.0),
            })
        })
        .collect();
    Value::Array(rows)
}

/// `--foresight-questions`: where the searched questions are written, and
/// the number of tables written to it.
static QUESTION_LOG: Mutex<Option<(std::io::BufWriter<std::fs::File>, u64)>> = Mutex::new(None);
/// The kinds of question written: priority, attack, block.
static QUESTION_KINDS: [AtomicBool; 3] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];
/// Whether only the questions the search answered otherwise are written.
static QUESTIONS_REPLACED: AtomicBool = AtomicBool::new(true);

/// Opens the question log; every searching table then keeps the questions
/// of `kinds` it searches (with `replaced`, only those the search answered
/// otherwise than the level) and writes them here.
pub(crate) fn open_question_log(
    path: &std::path::Path,
    kinds: &[String],
    replaced: bool,
) -> anyhow::Result<()> {
    for kind in kinds {
        let at = ["priority", "attack", "block"]
            .iter()
            .position(|name| name == kind)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "--foresight-question-kinds: `{kind}` is none of priority, attack, block"
                )
            })?;
        QUESTION_KINDS[at].store(true, Ordering::Relaxed);
    }
    QUESTIONS_REPLACED.store(replaced, Ordering::Relaxed);
    let file = std::fs::File::create(path)?;
    *QUESTION_LOG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some((std::io::BufWriter::new(file), 0));
    Ok(())
}

/// Writes a table's searched questions under the table's number in the
/// log: `g` the table, `view` and `pending` what the seat saw and was
/// asked, `tried` every answer tried (`a`, the target `then` named after
/// it, and `v` its value in each world, `null` where the engine refused
/// it; the first is the level's own) and `chosen` the index of the one
/// given.
fn write_questions(questions: &[baylee_gamehost::SearchedQuestion]) {
    use std::io::Write as _;
    if questions.is_empty() {
        return;
    }
    let mut log = QUESTION_LOG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some((log, tables)) = log.as_mut() else {
        return;
    };
    for question in questions {
        let tried: Vec<Value> = question
            .tried
            .iter()
            .map(|(answer, then, values)| json!({"a": answer, "then": then, "v": values}))
            .collect();
        let _ = writeln!(
            log,
            "{}",
            json!({
                "g": *tables,
                "view": question.view,
                "pending": question.pending,
                "tried": tried,
                "chosen": question.chosen,
            })
        );
    }
    *tables += 1;
}
