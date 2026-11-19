//! One v4/v5 net's steps from every game of a process, run in batches on one
//! set of sessions: the GPU's way to play the trained AI from many threads
//! (`eval4 --inference-batch`).
//!
//! Games stay synchronous threads. A [`NetPlayer4`](super::NetPlayer4) built
//! on a [`BatchedNet4`] sends each step's packed row to the lane of its depth
//! graph and waits for its outputs. A lane is one session and one thread.
//! While a call runs, the steps of the other games queue up, and the next
//! call takes all of them: no step waits for a batch to fill. A call holds at
//! most `max_rows` rows and `token_budget` padded entity and candidate
//! tokens; queued rows are grouped by size first, so a menu of thousands of
//! candidates is not padded into every row.
//!
//! The process holds one copy of each graph instead of one per thread, which
//! is what let `eval4` run more games than the card has memory for copies.
//!
//! **Servers (the CPU's way).** A lane may hold several sessions of its
//! graph, each on a thread of its own ([`BatchedNet4::load_served`],
//! `eval4 --inference-servers`), that take calls from the one queue at
//! once: then each takes a single call's rows and leaves the rest to the
//! others. On the CPU a call of many rows costs about as much per row as a
//! row alone (r1 on a Ryzen 3900X, one intra-op thread: 32 ms an
//! observation row and 33–42 ms a decision row, whatever the call holds), so
//! steps get cheaper only by running on more cores at once, and a
//! single-threaded session per core does that better than intra-op threads
//! (11 ms a row on four of them at 24 rows a call, against 8 ms from four
//! servers). A server's call of one row runs exactly as the owned path's
//! single step does.
//!
//! **Determinism.** Rows never mix in the graph, but kernels depend on the
//! call's shapes, so a row's outputs can differ from the single-step path by
//! floating-point rounding, and which rows share a call depends on timing.
//! Runs are not bitwise reproducible; reports record the batch limit. With
//! calls of one row (`max_rows = 1`) every row is the single step's, bit for
//! bit, whichever server runs it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Instant;

use anyhow::anyhow;
use ort::session::Session;

use super::inference::{StepRow, run_rows};
use super::{StepOutputs, abi, identity, load_session, load_session_with_hardware};

/// The most sessions one lane serves from.
pub const MAX_SERVERS: usize = 64;

/// Limits on one batched call.
#[derive(Clone, Copy, Debug)]
pub struct BatchLimits4 {
    /// Rows per call.
    pub max_rows: usize,
    /// Padded entity plus candidate tokens per call; a larger single row
    /// still runs, alone.
    pub token_budget: usize,
}

impl BatchLimits4 {
    /// Padded tokens a row may cost on average before a call is split.
    const TOKENS_PER_ROW: usize = 512;

    /// At most `rows` rows of about a large board each.
    #[must_use]
    pub const fn for_rows(rows: usize) -> Self {
        Self {
            max_rows: rows,
            token_budget: rows * Self::TOKENS_PER_ROW,
        }
    }
}

impl Default for BatchLimits4 {
    fn default() -> Self {
        Self::for_rows(32)
    }
}

struct Job {
    row: StepRow,
    reply: SyncSender<anyhow::Result<StepOutputs>>,
}

#[derive(Default)]
struct Queue {
    jobs: Vec<Job>,
    closed: bool,
}

type SharedQueue = Arc<(Mutex<Queue>, Condvar)>;

struct Lane {
    queue: SharedQueue,
    threads: Vec<JoinHandle<()>>,
    /// What the lane's servers did together.
    counters: Counters,
}

/// Calls run, rows answered and nanoseconds spent in calls, by every server
/// of a lane together.
type Counters = Arc<[AtomicU64; 3]>;

/// What one depth graph's lane did: calls, rows and seconds in calls.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct LaneStats4 {
    /// Calls run.
    pub calls: u64,
    /// Rows answered.
    pub rows: u64,
    /// Seconds spent in calls, every server's added up.
    pub call_seconds: f64,
}

impl LaneStats4 {
    fn of(counters: &[AtomicU64; 3]) -> Self {
        Self {
            calls: counters[0].load(Ordering::Relaxed),
            rows: counters[1].load(Ordering::Relaxed),
            // Nanoseconds, exact in an f64 for 104 days of calls.
            #[allow(clippy::cast_precision_loss, reason = "nanoseconds below 2^53")]
            call_seconds: counters[2].load(Ordering::Relaxed) as f64 * 1e-9,
        }
    }
}

/// The depth graphs of one model directory, shared by every player built on it.
pub struct BatchedNet4 {
    pub(super) model: super::ModelFiles,
    limits: BatchLimits4,
    hardware: Option<(baylee_driver::HardwareConfig, usize)>,
    /// Sessions per lane, each on its own thread.
    servers: usize,
    lanes: Mutex<BTreeMap<u8, Lane>>,
}

impl BatchedNet4 {
    /// Opens a supported V4/V5 model directory, as
    /// [`NetPlayer4::load_with`](super::NetPlayer4::load_with) does, for
    /// sharing.
    ///
    /// # Errors
    /// When the ONNX model or metadata cannot be loaded.
    pub fn load(
        path: &Path,
        limits: BatchLimits4,
        catalog: impl Into<super::CatalogRead4>,
    ) -> anyhow::Result<Arc<Self>> {
        Self::load_served(path, limits, catalog, 1)
    }

    /// [`Self::load`] with `servers` sessions of each depth graph, each on a
    /// thread of its own, answering calls at once (the module's
    /// "Servers"). One is [`Self::load`]'s single session per graph.
    ///
    /// # Errors
    /// When `servers` is outside `1..=`[`MAX_SERVERS`], or the ONNX model or
    /// metadata cannot be loaded.
    pub fn load_served(
        path: &Path,
        limits: BatchLimits4,
        catalog: impl Into<super::CatalogRead4>,
        servers: usize,
    ) -> anyhow::Result<Arc<Self>> {
        anyhow::ensure!(
            limits.max_rows >= 1 && limits.token_budget >= 1,
            "a batch needs at least one row and one token"
        );
        anyhow::ensure!(
            (1..=MAX_SERVERS).contains(&servers),
            "a lane is served by 1..={MAX_SERVERS} sessions"
        );
        let (model, depth, session) = super::ModelFiles::open(path, catalog.into())?;
        let net = Self {
            model,
            limits,
            hardware: None,
            servers,
            lanes: Mutex::new(BTreeMap::new()),
        };
        net.start_lane(&mut net.lanes(), depth, session)?;
        Ok(Arc::new(net))
    }

    /// Opens a supported V4/V5 model directory configured for `hardware` and
    /// `worker_slots`, resolving `fp16` vs `fp32` model directories and adaptive
    /// intra-op thread topology automatically.
    ///
    /// # Errors
    /// When the ONNX model or metadata cannot be loaded.
    pub fn load_with_hardware(
        path: &Path,
        limits: BatchLimits4,
        catalog: impl Into<super::CatalogRead4>,
        hardware: &baylee_driver::HardwareConfig,
        worker_slots: usize,
    ) -> anyhow::Result<Arc<Self>> {
        anyhow::ensure!(
            limits.max_rows >= 1 && limits.token_budget >= 1,
            "a batch needs at least one row and one token"
        );
        let (model, depth, session) =
            super::ModelFiles::open_with_hardware(path, catalog.into(), hardware, worker_slots)?;
        let net = Self {
            model,
            limits,
            hardware: Some((hardware.clone(), worker_slots)),
            servers: 1,
            lanes: Mutex::new(BTreeMap::new()),
        };
        net.start_lane(&mut net.lanes(), depth, session)?;
        Ok(Arc::new(net))
    }

    /// The limits every call keeps.
    #[must_use]
    pub const fn limits(&self) -> BatchLimits4 {
        self.limits
    }

    /// Sessions per depth graph, each answering calls on its own thread.
    #[must_use]
    pub const fn servers(&self) -> usize {
        self.servers
    }

    /// Exact model family, architecture and graph/metadata content hashes.
    #[must_use]
    pub const fn model_identity(&self) -> &super::ModelIdentity {
        &self.model.identity
    }

    /// Calls run and rows answered so far, by every lane.
    #[must_use]
    pub fn stats(&self) -> (u64, u64) {
        self.by_depth()
            .values()
            .fold((0, 0), |(calls, rows), lane| {
                (calls + lane.calls, rows + lane.rows)
            })
    }

    /// Seconds spent in calls so far, every lane's and server's added up:
    /// the net's share of a run's work.
    #[must_use]
    pub fn call_seconds(&self) -> f64 {
        self.by_depth().values().map(|lane| lane.call_seconds).sum()
    }

    /// What each depth graph's lane did so far: observation steps run on
    /// depth 0, decisions on their thinking depth.
    #[must_use]
    pub fn by_depth(&self) -> BTreeMap<u8, LaneStats4> {
        self.lanes()
            .iter()
            .map(|(depth, lane)| (*depth, LaneStats4::of(&lane.counters)))
            .collect()
    }

    fn lanes(&self) -> std::sync::MutexGuard<'_, BTreeMap<u8, Lane>> {
        self.lanes.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A session of `depth`'s graph, checked against the model.
    fn open_depth(&self, depth: u8) -> anyhow::Result<Session> {
        let path: PathBuf = self.model.model_dir.join(format!("model.k{depth}.onnx"));
        identity::verify_depth_graph(&path, depth, &self.model.identity)?;
        let session = match &self.hardware {
            Some((hw, slots)) => load_session_with_hardware(&path, hw, *slots)?,
            None => load_session(&path)?,
        };
        abi::validate(&session, self.model.encoder_version)?;
        identity::verify_depth_graph(&path, depth, &self.model.identity)?;
        Ok(session)
    }

    /// Starts `depth`'s lane on `session` and, with more than one server,
    /// on as many more sessions of its graph.
    fn start_lane(
        &self,
        lanes: &mut BTreeMap<u8, Lane>,
        depth: u8,
        session: Session,
    ) -> anyhow::Result<()> {
        let mut sessions = vec![session];
        for _ in 1..self.servers {
            sessions.push(self.open_depth(depth)?);
        }
        let queue: SharedQueue = Arc::default();
        let counters: Counters = Arc::default();
        let calls_at_once = self.servers > 1;
        let mut threads = Vec::with_capacity(sessions.len());
        for (server, session) in sessions.into_iter().enumerate() {
            let served = queue.clone();
            let limits = self.limits;
            let d_model = self.model.d_model;
            let counted = counters.clone();
            let thread = std::thread::Builder::new()
                .name(format!("netplay4-k{depth}-{server}"))
                .spawn(move || {
                    serve(session, &served, limits, d_model, &counted, calls_at_once);
                });
            match thread {
                Ok(thread) => threads.push(thread),
                Err(error) => {
                    stop(&queue, threads);
                    return Err(error.into());
                }
            }
        }
        lanes.insert(
            depth,
            Lane {
                queue,
                threads,
                counters,
            },
        );
        Ok(())
    }

    fn queue_for(&self, depth: u8) -> anyhow::Result<SharedQueue> {
        let mut lanes = self.lanes();
        if let Some(lane) = lanes.get(&depth) {
            return Ok(lane.queue.clone());
        }
        let session = self.open_depth(depth)?;
        self.start_lane(&mut lanes, depth, session)?;
        Ok(lanes[&depth].queue.clone())
    }

    /// Runs one packed step in the next call of its depth graph.
    pub(super) fn run(&self, row: StepRow) -> anyhow::Result<StepOutputs> {
        let queue = self.queue_for(super::depth_graph(row.k_val))?;
        let (reply, answer) = mpsc::sync_channel(1);
        {
            let (lock, ready) = &*queue;
            let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
            anyhow::ensure!(!q.closed, "the batched net stopped");
            q.jobs.push(Job { row, reply });
            ready.notify_one();
        }
        answer
            .recv()
            .map_err(|_| anyhow!("the batched net stopped"))?
    }
}

/// Closes `queue` and waits for the `threads` serving it.
fn stop(queue: &SharedQueue, threads: Vec<JoinHandle<()>>) {
    let (lock, ready) = &**queue;
    lock.lock().unwrap_or_else(PoisonError::into_inner).closed = true;
    ready.notify_all();
    for thread in threads {
        let _ = thread.join();
    }
}

impl Drop for BatchedNet4 {
    /// Stops every lane and waits for it, so the sessions are gone before the
    /// process exits (ONNX Runtime's CUDA teardown aborts otherwise).
    fn drop(&mut self) {
        let lanes = std::mem::take(&mut *self.lanes());
        for lane in lanes.values() {
            let (lock, ready) = &*lane.queue;
            lock.lock().unwrap_or_else(PoisonError::into_inner).closed = true;
            ready.notify_all();
        }
        for (_, lane) in lanes {
            for thread in lane.threads {
                let _ = thread.join();
            }
        }
    }
}

/// A server's thread: takes what is queued, runs it in calls, answers. A
/// lane's only server takes everything queued; one of several takes the
/// rows of one call and leaves the rest to the others (`calls_at_once`).
fn serve(
    mut session: Session,
    queue: &(Mutex<Queue>, Condvar),
    limits: BatchLimits4,
    d_model: usize,
    counters: &[AtomicU64; 3],
    calls_at_once: bool,
) {
    let (lock, ready) = queue;
    loop {
        let jobs = {
            let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
            while q.jobs.is_empty() && !q.closed {
                q = ready.wait(q).unwrap_or_else(PoisonError::into_inner);
            }
            if q.jobs.is_empty() {
                return;
            }
            if calls_at_once {
                // The oldest rows first, as many as fit one call: no row
                // waits behind rows that came after it.
                let first = calls(&q.jobs, limits).first().map_or(0, |call| call.len());
                let rest = q.jobs.split_off(first);
                let call = std::mem::replace(&mut q.jobs, rest);
                if !q.jobs.is_empty() {
                    ready.notify_one();
                }
                call
            } else {
                // Similar sizes share a call, so little of it is padding.
                q.jobs.sort_by_key(|job| (job.row.k_val, job.row.tokens()));
                std::mem::take(&mut q.jobs)
            }
        };
        for call in calls(&jobs, limits) {
            let rows: Vec<&StepRow> = call.iter().map(|job| &job.row).collect();
            counters[0].fetch_add(1, Ordering::Relaxed);
            counters[1].fetch_add(rows.len() as u64, Ordering::Relaxed);
            let started = Instant::now();
            let outputs = run_rows(&mut session, &rows, d_model);
            let spent = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
            counters[2].fetch_add(spent, Ordering::Relaxed);
            match outputs {
                Ok(outputs) => {
                    for (job, out) in call.iter().zip(outputs) {
                        let _ = job.reply.send(Ok(out));
                    }
                }
                Err(e) => {
                    for job in call {
                        let _ = job.reply.send(Err(anyhow!("{e:#}")));
                    }
                }
            }
        }
    }
}

/// Splits sorted jobs into calls within `limits`; a call never mixes depths.
fn calls(jobs: &[Job], limits: BatchLimits4) -> Vec<&[Job]> {
    let mut calls = Vec::new();
    let mut start = 0;
    let mut widest = 0;
    for (i, job) in jobs.iter().enumerate() {
        let rows = i - start;
        let tokens = widest.max(job.row.tokens());
        let fits = rows < limits.max_rows
            && (rows + 1) * tokens <= limits.token_budget
            && jobs[start].row.k_val == job.row.k_val;
        if rows > 0 && !fits {
            calls.push(&jobs[start..i]);
            start = i;
            widest = job.row.tokens();
        } else {
            widest = tokens;
        }
    }
    if start < jobs.len() {
        calls.push(&jobs[start..]);
    }
    calls
}
