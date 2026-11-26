//! Storing a game's record (#315) as its engine sends it.
//!
//! The engine sends its record in pieces over the engine link
//! (`GameRecordChunk`); this keeps them in the database as they come, never
//! reading inside them. The writes go through one task per game behind a
//! channel, so a slow database never holds up the seat frames the same
//! socket carries. The task outlives the socket until it has written what it
//! was handed.
//!
//! A bug report filed at a game that goes on asks the engine for its record
//! as it stands (#323, `FlushRecord`), and waits for it to be *stored*, not
//! merely sent: the engine's answer (`RecordFlushed`) comes after the piece
//! on the socket, and is handed down the same channel as the pieces, so the
//! writer resolves it only once every piece before it is written. [`Flushes`]
//! is where the report waits.
//!
//! Nothing here is reachable from a seat or the lobby: pieces go in, and
//! the one way out is `baylee_db::records::for_seated`, which only a bug
//! report from a player who sat at the game asks.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use baylee_protocol::v1;
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};

/// Keep rated-game reconciliation from voiding a game while its final record
/// and result are still being written. A failed writer drops the sender.
pub(crate) fn finalizing(state: &crate::Shared, game_id: &str, stored: oneshot::Receiver<()>) {
    if let Some(game) = state.lobby.lock().games.get_mut(game_id) {
        game.record_finalizing = true;
    }
    let state = state.clone();
    let game_id = game_id.to_owned();
    tokio::spawn(async move {
        let _ = stored.await;
        if let Some(game) = state.lobby.lock().games.get_mut(&game_id) {
            game.record_finalizing = false;
        }
    });
}

/// The most one game's record may take in the database, compressed. A
/// two-seat game of the acceptance decks sends 17–29 KB; this is a bound on
/// an engine that misbehaves, not on a long game.
pub const MAX_RECORD_BYTES: usize = 64 * 1024 * 1024;

/// What the writer is handed, in the order the engine said it.
#[derive(Debug)]
enum Entry {
    /// A piece to store.
    Piece(v1::GameRecordChunk),
    /// Everything before this is what a report waits for (#323): resolved
    /// once every piece handed in before it has been written.
    Flushed(oneshot::Sender<()>),
    /// How the game ended, from its engine's `GameEnded`: noted once the
    /// pieces before it are stored, so the row it updates exists.
    Ended(Ending),
    Challenge(central_api::challenges::Completion, oneshot::Sender<()>),
}

/// How a game ended, for the replay list (`docs/protocol.md` §"Replays").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ending {
    /// The winning seats; empty for a draw.
    pub winners: Vec<u8>,
    /// The turn it ended on, where the engine said.
    pub turn: Option<u32>,
    /// Why, as the engine names the reason.
    pub reason: String,
}

/// Where the writer puts the pieces: the database, or a test's list.
trait Store {
    /// Starts the game's record; called once, before its first piece.
    fn open(&mut self) -> impl Future<Output = Result<(), String>> + Send;
    /// Stores one piece.
    fn append(
        &mut self,
        piece: v1::GameRecordChunk,
    ) -> impl Future<Output = Result<(), String>> + Send;
    /// Notes how the game ended.
    fn finish(&mut self, ending: Ending) -> impl Future<Output = Result<(), String>> + Send;
    fn challenge(
        &mut self,
        result: central_api::challenges::Completion,
    ) -> impl Future<Output = Result<(), String>> + Send;
}

/// The gateway's database.
struct Database {
    db: sea_orm::DatabaseConnection,
    game_id: String,
    opening: baylee_db::records::Opening,
    seats: Vec<(baylee_db::records::Seat, baylee_db::records::SeatFacts)>,
}

impl Store for Database {
    async fn challenge(
        &mut self,
        result: central_api::challenges::Completion,
    ) -> Result<(), String> {
        baylee_db::challenges::complete(&self.db, &result)
            .await
            .map_err(|error| error.to_string())
    }
    async fn open(&mut self) -> Result<(), String> {
        baylee_db::records::open_with(&self.db, &self.game_id, &self.opening, &self.seats)
            .await
            .map_err(|e| e.to_string())
    }

    async fn finish(&mut self, ending: Ending) -> Result<(), String> {
        baylee_db::records::finish(
            &self.db,
            &self.game_id,
            &ending.winners,
            ending.turn,
            &ending.reason,
        )
        .await
        .map_err(|e| e.to_string())
    }

    async fn append(&mut self, piece: v1::GameRecordChunk) -> Result<(), String> {
        baylee_db::records::append(&self.db, &self.game_id, piece.seq, piece.data, piece.last)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// Writes what `rx` hands over, in order, until the sink is gone.
///
/// Nothing is written until the first piece arrives, which is after the
/// table opened (or when a report asked for it): while seats attach and
/// load, the database belongs to them (a pool of two in the e2e tests was
/// enough for this write to hold up a seat's sign-in). A flush answered
/// before any piece resolves without starting the record: there is none.
///
/// A write that fails is tried again, waiting [`RETRY_FIRST`] and doubling
/// to [`RETRY_MAX`], for up to [`RETRY_FOR`]: a database that restarts is
/// back within seconds, and a game resumed after a restart is only as whole
/// as its stored record (`docs/proposals/games-survive-restarts.md`). Every
/// write is idempotent (`baylee_db::records`), so a write that went through
/// but whose answer was lost is not written twice. Only past that time does
/// the writer stop, as it did at the first failure before.
async fn write(game_id: String, mut rx: mpsc::UnboundedReceiver<Entry>, mut store: impl Store) {
    let mut opened = false;
    while let Some(entry) = rx.recv().await {
        match entry {
            Entry::Piece(piece) => {
                if !opened {
                    let mut patience = Patience::new();
                    while let Err(e) = store.open().await {
                        if !patience.again(&game_id, &e).await {
                            tracing::error!(game_id, "starting the game's record: {e}");
                            return;
                        }
                    }
                    patience.done(&game_id);
                    opened = true;
                }
                let seq = piece.seq;
                let mut patience = Patience::new();
                while let Err(e) = store.append(piece.clone()).await {
                    if !patience.again(&game_id, &e).await {
                        tracing::error!(game_id, seq, "storing the game's record: {e}");
                        return;
                    }
                }
                patience.done(&game_id);
            }
            // Nobody waiting any more (the report gave up) is fine.
            Entry::Flushed(done) => {
                let _ = done.send(());
            }
            // A game that stored nothing has no row to note it on.
            Entry::Ended(ending) => {
                if !opened {
                    continue;
                }
                let mut patience = Patience::new();
                while let Err(e) = store.finish(ending.clone()).await {
                    if !patience.again(&game_id, &e).await {
                        tracing::error!(game_id, "noting how the game ended: {e}");
                        return;
                    }
                }
                patience.done(&game_id);
            }
            Entry::Challenge(result, done) => {
                let mut patience = Patience::new();
                while let Err(error) = store.challenge(result.clone()).await {
                    if !patience.again(&game_id, &error).await {
                        tracing::error!(game_id, "storing challenge outcome: {error}");
                        return;
                    }
                }
                patience.done(&game_id);
                let _ = done.send(());
            }
        }
    }
}

/// The first wait after a failed write.
const RETRY_FIRST: std::time::Duration = std::time::Duration::from_millis(250);
/// The longest wait between two tries of one write.
const RETRY_MAX: std::time::Duration = std::time::Duration::from_secs(10);
/// How long one write is tried before the writer gives the record up.
const RETRY_FOR: std::time::Duration = std::time::Duration::from_mins(15);

/// One write's tries: how long to wait before the next, and whether the
/// game's record has been said to be failing, which is said once a write
/// and not once a try.
struct Patience {
    since: tokio::time::Instant,
    wait: std::time::Duration,
    failed: bool,
}

impl Patience {
    fn new() -> Self {
        Self {
            since: tokio::time::Instant::now(),
            wait: RETRY_FIRST,
            failed: false,
        }
    }

    /// The write failed with `error`: waits and says to try again, or says
    /// to give up once [`RETRY_FOR`] has passed.
    async fn again(&mut self, game_id: &str, error: &str) -> bool {
        if self.since.elapsed() + self.wait > RETRY_FOR {
            return false;
        }
        if !self.failed {
            self.failed = true;
            tracing::warn!(
                game_id,
                "writing the game's record failed; trying again: {error}"
            );
        }
        tokio::time::sleep(self.wait).await;
        self.wait = (self.wait * 2).min(RETRY_MAX);
        true
    }

    /// The write went through, after `again` said to try once at least.
    fn done(&self, game_id: &str) {
        if self.failed {
            tracing::info!(game_id, "the game's record is written again");
        }
    }
}

/// One game's record on its way into the database.
pub struct Sink {
    game_id: String,
    tx: mpsc::UnboundedSender<Entry>,
    taken: usize,
    full: bool,
    next_seq: u32,
    last: bool,
}

impl Sink {
    /// Starts the game's record with who sits where (nobody for a chair the
    /// house plays; the host for its seat bridge's) and the task that writes
    /// it.
    pub fn open(
        db: sea_orm::DatabaseConnection,
        game_id: &str,
        opening: baylee_db::records::Opening,
        seats: Vec<(baylee_db::records::Seat, baylee_db::records::SeatFacts)>,
    ) -> Self {
        let store = Database {
            db,
            game_id: game_id.to_owned(),
            opening,
            seats,
        };
        Self::writing(game_id, store)
    }

    /// A sink whose task writes into `store`.
    fn writing(game_id: &str, store: impl Store + Send + 'static) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(write(game_id.to_owned(), rx, store));
        Self::feeding(game_id, tx)
    }

    /// A sink handing its entries to `tx`.
    fn feeding(game_id: &str, tx: mpsc::UnboundedSender<Entry>) -> Self {
        Self {
            game_id: game_id.to_owned(),
            tx,
            taken: 0,
            full: false,
            next_seq: 0,
            last: false,
        }
    }

    /// Hands a piece to the writer, unless it is for another game or the
    /// record is already at [`MAX_RECORD_BYTES`].
    pub fn take(&mut self, piece: v1::GameRecordChunk) {
        if piece.game_id != self.game_id || self.full {
            return;
        }
        if piece.seq != self.next_seq || self.last {
            self.full = true;
            return;
        }
        self.next_seq = self.next_seq.saturating_add(1);
        self.last = piece.last;
        self.taken = self.taken.saturating_add(piece.data.len());
        if self.taken > MAX_RECORD_BYTES {
            self.full = true;
            tracing::warn!(
                game_id = self.game_id,
                "the game's record is over its bound; the rest is dropped"
            );
            return;
        }
        let _ = self.tx.send(Entry::Piece(piece));
    }

    /// The engine has sent everything a report asked for (#323): `done`
    /// fires once every piece handed in before this call is written. Also
    /// past [`MAX_RECORD_BYTES`], where the answer is "that is all there
    /// will be", which is as true.
    pub fn flushed(&mut self, done: oneshot::Sender<()>) {
        let _ = self.tx.send(Entry::Flushed(done));
    }

    /// The game ended so (`GameEnded`): noted after every piece handed in
    /// before it is stored.
    pub fn ended(&mut self, ending: Ending) {
        let _ = self.tx.send(Entry::Ended(ending));
    }

    /// All chunks arrived once, in order, including the final chunk.
    pub(crate) fn complete(&self) -> bool {
        self.last && !self.full
    }

    /// Queue the result only after every contiguous piece, including `last`.
    /// A storage error closes the writer before it can store this outcome.
    pub(crate) fn challenge(
        &mut self,
        result: central_api::challenges::Completion,
    ) -> Option<oneshot::Receiver<()>> {
        if self.complete() {
            let (done, stored) = oneshot::channel();
            self.tx.send(Entry::Challenge(result, done)).ok()?;
            Some(stored)
        } else {
            None
        }
    }
}

/// Reports waiting for a game's engine to send its record as it stands
/// (#323), by the nonce each asked with. In memory only: a report waits
/// seconds, and a gateway that restarts has no engine link left to ask on.
#[derive(Default)]
pub struct Flushes {
    /// The last nonce handed out.
    last: AtomicU64,
    /// Each open ask: the game it was sent to, and who waits for it.
    waiting: Mutex<HashMap<u64, (String, oneshot::Sender<()>)>>,
}

/// How long a stopping gateway waits for the running games' records.
pub(crate) const FLUSH_ON_STOP: std::time::Duration = std::time::Duration::from_secs(5);

/// Every running game's engine is asked for its record as it stands
/// (`FlushRecord`, #323), and this waits until each answer is stored, for
/// `within` at most: what a gateway that is told to stop does before it
/// stops, so that a game resumed after the restart misses nothing
/// (`docs/proposals/games-survive-restarts.md`). Not asked: a Tower engine,
/// which keeps its record with the Tower's authority and ends its duel on
/// any other frame, and a ranked duel's protected engines, which move only
/// by their certified journal. How many games answered in time.
pub(crate) async fn flush_all(state: &crate::Shared, within: std::time::Duration) -> usize {
    let engines: Vec<(String, crate::lobby::EngineLink)> = {
        let lobby = state.lobby.lock();
        lobby
            .running()
            .filter(|game| !crate::report::flushes_nothing(game))
            .filter(|game| game.ranked.is_none_or(|binding| binding.replica.is_none()))
            .filter_map(|game| game.engine.clone().map(|engine| (game.id.clone(), engine)))
            .collect()
    };
    let flushes = &state.record_flushes;
    let mut asked = Vec::new();
    for (game_id, engine) in engines {
        let (ask, stored) = flushes.ask(&game_id);
        let flush = v1::Envelope {
            msg: Some(v1::envelope::Msg::FlushRecord(v1::FlushRecord {
                game_id,
                nonce: ask.nonce,
            })),
        };
        if engine.send(flush).is_ok() {
            asked.push((ask, stored));
        }
    }
    let deadline = tokio::time::Instant::now() + within;
    let mut answered = 0;
    for (_ask, stored) in asked {
        if matches!(tokio::time::timeout_at(deadline, stored).await, Ok(Ok(()))) {
            answered += 1;
        }
    }
    answered
}

/// On `SIGTERM` (a deploy, a reboot, `systemctl restart`) or Ctrl-C: the
/// running games' records are stored ([`flush_all`]), and then the process
/// ends, as the signal would have ended it. Before this a stop ended the
/// gateway at once and whatever an engine had not yet sent was lost.
pub(crate) fn flush_on_stop(state: crate::Shared) {
    tokio::spawn(async move {
        stopped().await;
        let answered = flush_all(&state, FLUSH_ON_STOP).await;
        tracing::info!(answered, "stopping: the running games' records are stored");
        std::process::exit(0);
    });
}

/// Resolves when the process is told to stop.
async fn stopped() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let Ok(mut terminate) = signal(SignalKind::terminate()) else {
            let _ = tokio::signal::ctrl_c().await;
            return;
        };
        tokio::select! {
            _ = terminate.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

impl Flushes {
    /// A new ask of `game_id`'s engine: the nonce to send it with, and what
    /// resolves once the record it answers with is stored. The ask is
    /// forgotten when the [`Asked`] is dropped, however the report ends,
    /// its request cancelled half way included, so an engine that never
    /// answers leaves nothing behind.
    pub fn ask(&self, game_id: &str) -> (Asked<'_>, oneshot::Receiver<()>) {
        let nonce = self.last.fetch_add(1, Ordering::Relaxed) + 1;
        let (tx, rx) = oneshot::channel();
        self.waiting.lock().insert(nonce, (game_id.to_owned(), tx));
        (
            Asked {
                flushes: self,
                nonce,
            },
            rx,
        )
    }

    /// The engine of `game_id` answered `nonce`: who waits for it, if
    /// anybody still does and the ask was made of that game. An engine's
    /// answer about another game's ask is not taken, and the ask stays.
    pub fn answered(&self, game_id: &str, nonce: u64) -> Option<oneshot::Sender<()>> {
        let mut waiting = self.waiting.lock();
        if waiting.get(&nonce).is_none_or(|(game, _)| game != game_id) {
            return None;
        }
        waiting.remove(&nonce).map(|(_, done)| done)
    }

    /// The report stopped waiting for `nonce`.
    fn forget(&self, nonce: u64) {
        self.waiting.lock().remove(&nonce);
    }

    /// How many asks are open.
    #[cfg(test)]
    fn open(&self) -> usize {
        self.waiting.lock().len()
    }
}

/// One open ask ([`Flushes::ask`]); forgotten when dropped.
pub struct Asked<'a> {
    flushes: &'a Flushes,
    /// What the engine is sent the ask with.
    pub nonce: u64,
}

impl Drop for Asked<'_> {
    fn drop(&mut self) {
        self.flushes.forget(self.nonce);
    }
}

#[cfg(test)]
mod tests;
