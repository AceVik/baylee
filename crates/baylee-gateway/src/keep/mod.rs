//! The running games this gateway keeps in PostgreSQL, so that a restart of
//! the gateway (a deploy, a reboot, a crash) does not end them
//! (`docs/proposals/games-survive-restarts.md` §"Step 3b").
//!
//! One task writes every row ([`Keeper`]), always the state the lobby holds
//! when it writes, so a later state is never overwritten by an earlier one.
//! It writes a game when it is asked to ([`Keeper::touch`], and awaited,
//! [`Keeper::sync`]) and every [`SWEEP`] whatever changed, so no change
//! is left out for long because nobody asked. A change to what opens a chair
//! is awaited before its route answers: restoring an older seat token
//! would reopen a chair that was taken back.
//!
//! A game is kept from its record's first stored piece (`record.rs` asks)
//! until it ends; the last piece takes its row in the same statement
//! (`baylee_db::records::append`). A stop writes every game once more and
//! then nothing ([`Keeper::settle`]): the links that close as the process
//! ends must not end the games the stop is keeping.
//!
//! [`restore`] reads the rows back as the gateway starts.

mod restore;
mod row;
#[cfg(test)]
mod tests;

pub(crate) use restore::{order_restored, restore};

use crate::Shared;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::hash::{Hash as _, Hasher as _};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// How often every running game is written whatever asked: the bound on how
/// long a change nobody announced waits to be kept.
const SWEEP: Duration = Duration::from_secs(5);

/// How long an awaited write is tried before its route is told it failed.
const SYNC_WITHIN: Duration = Duration::from_secs(5);

/// What the writer is asked.
enum Ask {
    /// Write this game now.
    Touch(String),
    /// Write this game now and say whether it is kept as it stands.
    Sync(String, oneshot::Sender<Result<(), String>>),
    /// Write every game as it stands, then nothing more: the gateway stops.
    Settle(oneshot::Sender<()>),
}

/// The gateway's end of the writer.
pub(crate) struct Keeper {
    tx: mpsc::UnboundedSender<Ask>,
    rx: Mutex<Option<mpsc::UnboundedReceiver<Ask>>>,
    frozen: AtomicBool,
    /// Every row the start read, resumed or not: the writer has each, and
    /// its first sweep forgets those whose game is not in the lobby.
    found: Mutex<Vec<String>>,
}

impl Default for Keeper {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: Mutex::new(Some(rx)),
            frozen: AtomicBool::new(false),
            found: Mutex::new(Vec::new()),
        }
    }
}

impl Keeper {
    /// Asks for `game_id` to be written as it stands now, without waiting.
    pub(crate) fn touch(&self, game_id: &str) {
        let _ = self.tx.send(Ask::Touch(game_id.to_owned()));
    }

    /// Writes `game_id` as it stands now and waits until it is: what a route
    /// that changed who may open a chair calls before it answers.
    ///
    /// # Errors
    ///
    /// When the write did not go through within [`SYNC_WITHIN`]. The game
    /// is then written again at every later sweep until it does.
    pub(crate) async fn sync(&self, game_id: &str) -> Result<(), String> {
        let (done, written) = oneshot::channel();
        self.tx
            .send(Ask::Sync(game_id.to_owned(), done))
            .map_err(|_| "the writer of running games is gone".to_owned())?;
        match tokio::time::timeout(SYNC_WITHIN, written).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => Err("the writer of running games is gone".to_owned()),
            Err(_) => Err("keeping the game took too long".to_owned()),
        }
    }

    /// Writes every game as it stands and then nothing more: the gateway is
    /// stopping, and what is kept then is what the next start resumes. At
    /// most [`SYNC_WITHIN`]; nothing is written after it either way.
    pub(crate) async fn settle(&self) {
        let (done, settled) = oneshot::channel();
        if self.tx.send(Ask::Settle(done)).is_ok() {
            let _ = tokio::time::timeout(SYNC_WITHIN, settled).await;
        }
        self.freeze();
    }

    /// The rows the start read ([`restore`]), for the writer to own.
    fn found(&self, ids: impl IntoIterator<Item = String>) {
        self.found.lock().extend(ids);
    }

    /// Writes nothing more.
    fn freeze(&self) {
        self.frozen.store(true, Ordering::Release);
    }

    fn frozen(&self) -> bool {
        self.frozen.load(Ordering::Acquire)
    }
}

/// Starts the writer. Once; a second call finds nothing to start.
pub(crate) fn spawn(state: Shared) {
    let Some(rx) = state.keeper.rx.lock().take() else {
        return;
    };
    tokio::spawn(run(state, rx));
}

/// What the writer last wrote of each game it keeps: a digest of the row,
/// and the games it could not write yet.
#[derive(Default)]
struct Written {
    rows: HashMap<String, u64>,
}

async fn run(state: Shared, mut rx: mpsc::UnboundedReceiver<Ask>) {
    let mut written = Written::default();
    // Every row the start read is the writer's: a game restored is kept
    // already, and the row of one that was not (it did not read, or the
    // start could not check its accounts) goes at the first sweep, which
    // runs at once, rather than coming back at the next restart.
    let found = std::mem::take(&mut *state.keeper.found.lock());
    for id in found.into_iter().chain(restore::restored_ids(&state)) {
        written.rows.insert(id, 0);
    }
    let mut sweep = tokio::time::interval(SWEEP);
    sweep.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            ask = rx.recv() => {
                let Some(ask) = ask else { return };
                match ask {
                    Ask::Touch(id) => {
                        if !state.keeper.frozen() {
                            let _ = write(&state, &mut written, &id).await;
                        }
                    }
                    Ask::Sync(id, done) => {
                        let outcome = if state.keeper.frozen() {
                            Ok(())
                        } else {
                            write(&state, &mut written, &id).await
                        };
                        let _ = done.send(outcome);
                    }
                    Ask::Settle(done) => {
                        if !state.keeper.frozen() {
                            sweep_all(&state, &mut written).await;
                        }
                        state.keeper.freeze();
                        let _ = done.send(());
                    }
                }
            }
            _ = sweep.tick() => {
                if !state.keeper.frozen() {
                    sweep_all(&state, &mut written).await;
                    // A restored game an agent could not take before (no
                    // room, or no agent yet) is asked for again.
                    restore::order_restored(&state);
                }
            }
        }
    }
}

/// Writes every game that changed since it was last written, and forgets
/// every game that is no longer kept.
async fn sweep_all(state: &Shared, written: &mut Written) {
    let mut ids: Vec<String> = {
        let lobby = state.lobby.lock();
        lobby
            .games
            .values()
            .filter(|game| row::keepable(game))
            .map(|game| game.id.clone())
            .collect()
    };
    ids.extend(written.rows.keys().cloned());
    ids.sort();
    ids.dedup();
    for id in ids {
        let _ = write(state, written, &id).await;
    }
}

/// Writes `id` as the lobby holds it now: its row when it is kept, none
/// when it is not (over, gone, or never one to keep).
async fn write(state: &Shared, written: &mut Written, id: &str) -> Result<(), String> {
    let row = {
        let lobby = state.lobby.lock();
        lobby.games.get(id).and_then(row::Row::of)
    };
    let Some(row) = row else {
        if written.rows.contains_key(id) {
            baylee_db::running::forget(&state.db, id)
                .await
                .map_err(|e| failed(id, &e.to_string()))?;
            written.rows.remove(id);
        }
        return Ok(());
    };
    let body = serde_json::to_string(&row).map_err(|e| failed(id, &e.to_string()))?;
    let digest = digest(&body);
    if written.rows.get(id) == Some(&digest) {
        return Ok(());
    }
    let kept = baylee_db::running::keep(&state.db, id, row::VERSION, &body)
        .await
        .map_err(|e| failed(id, &e.to_string()))?;
    // A game whose record has no piece yet has nothing to resume from: it
    // is written once its first piece is stored, which asks again.
    if kept {
        written.rows.insert(id.to_owned(), digest);
    }
    Ok(())
}

fn failed(id: &str, why: &str) -> String {
    tracing::warn!(game_id = id, "keeping a running game: {why}");
    "the running game could not be kept".to_owned()
}

/// A digest of a row's text, to skip writing what is already written. Not
/// stored, and only compared within this process.
fn digest(body: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    body.hash(&mut hasher);
    // 0 marks a row restored at the start, never written by this process.
    hasher.finish().max(1)
}
