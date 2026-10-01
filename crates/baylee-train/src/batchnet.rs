//! One net's decisions from every game of a process, run in batches on one
//! set of sessions: the GPU's way to play the trained AI
//! (`export_onnx3.py --batch`).
//!
//! Games stay synchronous threads. A [`crate::netplay3::NetPlayer3`] given a
//! server sends each decision's inputs down a channel and waits for its
//! tables. The server thread takes the first request, gathers others of any
//! bucket until one bucket holds a full batch or `wait` has passed, and runs
//! each bucket's requests.
//!
//! **Determinism.** Every run is exactly `batch` rows at the bucket's rows,
//! padded with copies of its first request, so every decision goes through
//! the same kernels whoever shares its batch and however the games are
//! timed. The rows of a batch never mix in the graph (every operation is
//! per decision), and the sessions run with deterministic compute and
//! without TF32. A replay therefore gets the same answers.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::time::{Duration, Instant};

use anyhow::{Context as _, anyhow, bail};
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;

use crate::netplay3::{Feeds, Tables, run};

struct Request {
    feeds: Feeds,
    reply: SyncSender<anyhow::Result<Tables>>,
}

enum Msg {
    Decide(Box<Request>),
    /// Ends the server: its sessions must be gone before the process
    /// exits, or ONNX Runtime's CUDA teardown aborts it.
    Stop,
}

/// A handle on one net's batch server; clones share it.
#[derive(Clone)]
pub struct BatchNet {
    tx: SyncSender<Msg>,
    stats: Arc<[AtomicU64; 2]>,
    batch: usize,
    thread: Arc<std::sync::Mutex<Option<std::thread::JoinHandle<()>>>>,
}

impl BatchNet {
    /// Loads `model`'s fixed-batch exports (`batched` in its
    /// `net.onnx.json`) and starts the thread that serves them. With the
    /// `onnx-cuda` feature they run on the GPU (`BAYLEE_EP=cpu` keeps them on
    /// the CPU), and a GPU that cannot be used is an error, not a quiet
    /// fallback.
    ///
    /// # Errors
    /// When the export has no batched files or ONNX Runtime cannot load them.
    pub fn spawn(model: &Path, wait: Duration) -> anyhow::Result<Self> {
        let meta: serde_json::Value = serde_json::from_slice(
            &std::fs::read(model.with_extension("onnx.json")).with_context(|| {
                format!("reading {}", model.with_extension("onnx.json").display())
            })?,
        )?;
        let listed = meta["batched"]
            .as_array()
            .context("the export has no batched files: run export_onnx3.py --batch")?;
        let mut sessions = BTreeMap::new();
        let mut batch = 0;
        // Only the full-size export: every session holds its own activation
        // memory, and a league of three nets at five sizes each filled the
        // card's 12 GB. Smaller decisions are padded up, which costs compute
        // the GPU has to spare and fills the batches better besides.
        let mut largest: Option<(usize, std::path::PathBuf)> = None;
        for b in listed {
            let (Some(rows), Some(size), Some(file)) = (
                b["entities"].as_u64(),
                b["batch"].as_u64(),
                b["file"].as_str(),
            ) else {
                bail!("a batched entry without entities, batch and file: {b}");
            };
            let size = usize::try_from(size)?;
            if batch != 0 && size != batch {
                bail!("the batched files mix batch sizes {batch} and {size}");
            }
            batch = size;
            let rows = usize::try_from(rows)?;
            if largest.as_ref().is_none_or(|(r, _)| rows > *r) {
                largest = Some((rows, model.with_file_name(file)));
            }
        }
        if let Some((rows, file)) = largest {
            sessions.insert(rows, open(&file)?);
        }
        let (tx, rx) = mpsc::sync_channel(4096);
        let stats = Arc::new([AtomicU64::new(0), AtomicU64::new(0)]);
        let counted = stats.clone();
        let thread = std::thread::Builder::new()
            .name("batchnet".into())
            .spawn(move || serve(sessions, batch, wait, &rx, &counted))?;
        Ok(Self {
            tx,
            stats,
            batch,
            thread: Arc::new(std::sync::Mutex::new(Some(thread))),
        })
    }

    /// Stops the server and waits for it, so its sessions are dropped
    /// before the process exits. Call it once every game is over; a
    /// decision sent after it is an error.
    pub fn shutdown(&self) {
        let _ = self.tx.send(Msg::Stop);
        let thread = self.thread.lock().ok().and_then(|mut t| t.take());
        if let Some(thread) = thread {
            let _ = thread.join();
        }
    }

    /// The tables for one decision, when its batch has run.
    pub(crate) fn tables(&self, feeds: Feeds) -> anyhow::Result<Tables> {
        let (reply, answer) = mpsc::sync_channel(1);
        self.tx
            .send(Msg::Decide(Box::new(Request { feeds, reply })))
            .map_err(|_| anyhow!("the batch server stopped"))?;
        answer
            .recv()
            .map_err(|_| anyhow!("the batch server stopped"))?
    }

    /// Batches run and decisions answered so far: their ratio over the batch
    /// size is how full the batches were.
    #[must_use]
    pub fn stats(&self) -> (u64, u64, usize) {
        (
            self.stats[0].load(Ordering::Relaxed),
            self.stats[1].load(Ordering::Relaxed),
            self.batch,
        )
    }
}

fn open(path: &Path) -> anyhow::Result<Session> {
    fn err(e: impl std::fmt::Display) -> anyhow::Error {
        anyhow!("{e}")
    }
    let builder = Session::builder().map_err(err)?;
    #[cfg(feature = "onnx-cuda")]
    let builder = if std::env::var("BAYLEE_EP").as_deref() == Ok("cpu") {
        builder
    } else {
        builder
            // Each session's arena grows by what it is asked for: the
            // default doubling ran a league of three nets (fifteen
            // sessions) out of the card's 12 GB.
            .with_execution_providers([ort::ep::CUDA::default()
                .with_tf32(false)
                .with_arena_extend_strategy(ort::ep::ArenaExtendStrategy::SameAsRequested)
                .build()
                .error_on_failure()])
            .map_err(err)?
    };
    builder
        .with_deterministic_compute(true)
        .map_err(err)?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(err)?
        .commit_from_file(path)
        .map_err(err)
        .with_context(|| format!("loading {}", path.display()))
}

fn serve(
    mut sessions: BTreeMap<usize, Session>,
    batch: usize,
    wait: Duration,
    rx: &Receiver<Msg>,
    stats: &[AtomicU64; 2],
) {
    let mut stopping = false;
    while !stopping {
        let Ok(Msg::Decide(first)) = rx.recv() else {
            return;
        };
        let mut waiting: BTreeMap<usize, Vec<Request>> = BTreeMap::new();
        waiting.entry(first.feeds.e).or_default().push(*first);
        let deadline = Instant::now() + wait;
        while waiting.values().all(|v| v.len() < batch) {
            match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(Msg::Decide(r)) => waiting.entry(r.feeds.e).or_default().push(*r),
                Ok(Msg::Stop) => {
                    stopping = true;
                    break;
                }
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        for (e, requests) in waiting {
            for chunk in requests.chunks(batch) {
                let result = match sessions.get_mut(&e) {
                    Some(session) => {
                        let rows: Vec<&Feeds> = chunk.iter().map(|r| &r.feeds).collect();
                        run(session, &rows, batch)
                    }
                    None => Err(anyhow!("no batched export for {e} rows")),
                };
                stats[0].fetch_add(1, Ordering::Relaxed);
                stats[1].fetch_add(chunk.len() as u64, Ordering::Relaxed);
                match result {
                    Ok(tables) => {
                        for (r, t) in chunk.iter().zip(tables) {
                            let _ = r.reply.send(Ok(t));
                        }
                    }
                    Err(e) => {
                        for r in chunk {
                            let _ = r.reply.send(Err(anyhow!("{e:#}")));
                        }
                    }
                }
            }
        }
    }
}
