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
//! **Determinism.** Rows never mix in the graph, but kernels depend on the
//! call's shapes, so a row's outputs can differ from the single-step path by
//! floating-point rounding, and which rows share a call depends on timing.
//! Runs are not bitwise reproducible; reports record the batch limit.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::JoinHandle;

use anyhow::anyhow;
use ort::session::Session;

use super::inference::{StepRow, run_rows};
use super::{StepOutputs, abi, identity, load_session};

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
    thread: Option<JoinHandle<()>>,
}

/// The depth graphs of one model directory, shared by every player built on it.
pub struct BatchedNet4 {
    pub(super) model: super::ModelFiles,
    limits: BatchLimits4,
    lanes: Mutex<BTreeMap<u8, Lane>>,
    /// Calls run and rows answered by every lane: their ratio is the mean
    /// batch size.
    stats: Arc<[AtomicU64; 2]>,
}

impl BatchedNet4 {
    /// Opens a supported V4/V5 model directory, as
    /// [`NetPlayer4::load_with`](super::NetPlayer4::load_with) does, for
    /// sharing.
    ///
    /// # Errors
    /// When the ONNX model or metadata cannot be loaded.
    pub fn load(
        path: &std::path::Path,
        limits: BatchLimits4,
        catalog: super::CatalogPolicy4,
    ) -> anyhow::Result<Arc<Self>> {
        anyhow::ensure!(
            limits.max_rows >= 1 && limits.token_budget >= 1,
            "a batch needs at least one row and one token"
        );
        let (model, depth, session) = super::ModelFiles::open(path, catalog)?;
        let net = Self {
            model,
            limits,
            lanes: Mutex::new(BTreeMap::new()),
            stats: Arc::new([AtomicU64::new(0), AtomicU64::new(0)]),
        };
        net.start_lane(&mut net.lanes(), depth, session)?;
        Ok(Arc::new(net))
    }

    /// The limits every call keeps.
    #[must_use]
    pub const fn limits(&self) -> BatchLimits4 {
        self.limits
    }

    /// Exact model family, architecture and graph/metadata content hashes.
    #[must_use]
    pub const fn model_identity(&self) -> &super::ModelIdentity {
        &self.model.identity
    }

    /// Calls run and rows answered so far.
    #[must_use]
    pub fn stats(&self) -> (u64, u64) {
        (
            self.stats[0].load(Ordering::Relaxed),
            self.stats[1].load(Ordering::Relaxed),
        )
    }

    fn lanes(&self) -> std::sync::MutexGuard<'_, BTreeMap<u8, Lane>> {
        self.lanes.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn start_lane(
        &self,
        lanes: &mut BTreeMap<u8, Lane>,
        depth: u8,
        session: Session,
    ) -> anyhow::Result<()> {
        let queue: SharedQueue = Arc::default();
        let served = queue.clone();
        let limits = self.limits;
        let d_model = self.model.d_model;
        let counted = self.stats.clone();
        let thread = std::thread::Builder::new()
            .name(format!("netplay4-k{depth}"))
            .spawn(move || serve(session, &served, limits, d_model, &counted))?;
        lanes.insert(
            depth,
            Lane {
                queue,
                thread: Some(thread),
            },
        );
        Ok(())
    }

    fn queue_for(&self, depth: u8) -> anyhow::Result<SharedQueue> {
        let mut lanes = self.lanes();
        if let Some(lane) = lanes.get(&depth) {
            return Ok(lane.queue.clone());
        }
        let path: PathBuf = self.model.model_dir.join(format!("model.k{depth}.onnx"));
        identity::verify_depth_graph(&path, depth, &self.model.identity)?;
        let session = load_session(&path)?;
        abi::validate(&session, self.model.encoder_version)?;
        identity::verify_depth_graph(&path, depth, &self.model.identity)?;
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

impl Drop for BatchedNet4 {
    /// Stops every lane and waits for it, so the sessions are gone before the
    /// process exits (ONNX Runtime's CUDA teardown aborts otherwise).
    fn drop(&mut self) {
        let mut lanes = std::mem::take(&mut *self.lanes());
        for lane in lanes.values() {
            let (lock, ready) = &*lane.queue;
            lock.lock().unwrap_or_else(PoisonError::into_inner).closed = true;
            ready.notify_all();
        }
        for lane in lanes.values_mut() {
            if let Some(thread) = lane.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

/// A lane's thread: takes everything queued, runs it in calls, answers.
fn serve(
    mut session: Session,
    queue: &(Mutex<Queue>, Condvar),
    limits: BatchLimits4,
    d_model: usize,
    counters: &[AtomicU64; 2],
) {
    let (lock, ready) = queue;
    loop {
        let mut jobs = {
            let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
            while q.jobs.is_empty() && !q.closed {
                q = ready.wait(q).unwrap_or_else(PoisonError::into_inner);
            }
            if q.jobs.is_empty() {
                return;
            }
            std::mem::take(&mut q.jobs)
        };
        // Similar sizes share a call, so little of it is padding.
        jobs.sort_by_key(|job| (job.row.k_val, job.row.tokens()));
        for call in calls(&jobs, limits) {
            let rows: Vec<&StepRow> = call.iter().map(|job| &job.row).collect();
            counters[0].fetch_add(1, Ordering::Relaxed);
            counters[1].fetch_add(rows.len() as u64, Ordering::Relaxed);
            match run_rows(&mut session, &rows, d_model) {
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
