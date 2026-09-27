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
use uuid::Uuid;

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
}

/// The gateway's database.
struct Database {
    db: sea_orm::DatabaseConnection,
    game_id: String,
    seats: Vec<Option<Uuid>>,
}

impl Store for Database {
    async fn open(&mut self) -> Result<(), String> {
        baylee_db::records::open(&self.db, &self.game_id, &self.seats)
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
async fn write(game_id: String, mut rx: mpsc::UnboundedReceiver<Entry>, mut store: impl Store) {
    let mut opened = false;
    while let Some(entry) = rx.recv().await {
        match entry {
            Entry::Piece(piece) => {
                if !opened {
                    if let Err(e) = store.open().await {
                        tracing::error!(game_id, "starting the game's record: {e}");
                        return;
                    }
                    opened = true;
                }
                let seq = piece.seq;
                if let Err(e) = store.append(piece).await {
                    tracing::error!(game_id, seq, "storing the game's record: {e}");
                }
            }
            // Nobody waiting any more (the report gave up) is fine.
            Entry::Flushed(done) => {
                let _ = done.send(());
            }
        }
    }
}

/// One game's record on its way into the database.
pub struct Sink {
    game_id: String,
    tx: mpsc::UnboundedSender<Entry>,
    taken: usize,
    full: bool,
}

impl Sink {
    /// Starts the game's record with who sits where (`None` for a chair the
    /// house plays) and the task that writes it.
    pub fn open(db: sea_orm::DatabaseConnection, game_id: &str, seats: Vec<Option<Uuid>>) -> Self {
        let store = Database {
            db,
            game_id: game_id.to_owned(),
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
        }
    }

    /// Hands a piece to the writer, unless it is for another game or the
    /// record is already at [`MAX_RECORD_BYTES`].
    pub fn take(&mut self, piece: v1::GameRecordChunk) {
        if piece.game_id != self.game_id || self.full {
            return;
        }
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
mod tests {
    use super::*;
    use std::sync::Arc;

    fn piece(game_id: &str, seq: u32, bytes: usize) -> v1::GameRecordChunk {
        v1::GameRecordChunk {
            game_id: game_id.to_owned(),
            seq,
            data: vec![0; bytes],
            last: false,
        }
    }

    fn handed(rx: &mut mpsc::UnboundedReceiver<Entry>) -> Vec<u32> {
        std::iter::from_fn(|| rx.try_recv().ok())
            .map(|e| match e {
                Entry::Piece(p) => p.seq,
                Entry::Flushed(_) => u32::MAX,
            })
            .collect()
    }

    /// Only this game's pieces go to the writer, and only up to
    /// [`MAX_RECORD_BYTES`] of them: the piece that crosses it and every one
    /// after are dropped, however small.
    #[test]
    fn a_sink_takes_its_own_game_up_to_the_bound() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut sink = Sink::feeding("g1", tx);
        sink.take(piece("g1", 0, 10));
        sink.take(piece("g2", 1, 10));
        sink.take(piece("", 2, 10));
        assert_eq!(handed(&mut rx), [0], "another game's piece is dropped");

        sink.take(piece("g1", 1, MAX_RECORD_BYTES - 10));
        assert_eq!(handed(&mut rx), [1], "exactly the bound fits");
        sink.take(piece("g1", 2, 1));
        sink.take(piece("g1", 3, 0));
        assert!(handed(&mut rx).is_empty(), "past the bound nothing goes");
        // A report still hears that this is all there is.
        sink.flushed(oneshot::channel().0);
        assert_eq!(handed(&mut rx), [u32::MAX]);
    }

    /// What a [`Store`] was asked to do, in order.
    #[derive(Clone, Default)]
    struct Log(Arc<Mutex<Vec<String>>>);

    impl Log {
        fn said(&self) -> Vec<String> {
            self.0.lock().clone()
        }
    }

    /// A store that gives the runtime away several times over every write,
    /// as a busy database does, so a flush that raced its pieces would be
    /// seen resolving first.
    struct Slow(Log);

    async fn dawdle() {
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
    }

    impl Store for Slow {
        async fn open(&mut self) -> Result<(), String> {
            dawdle().await;
            self.0.0.lock().push("open".to_owned());
            Ok(())
        }

        async fn append(&mut self, piece: v1::GameRecordChunk) -> Result<(), String> {
            dawdle().await;
            self.0.0.lock().push(format!("piece {}", piece.seq));
            Ok(())
        }
    }

    /// A flush resolves only once every piece handed in before it has been
    /// written (#323): that is what lets a report read the store the moment
    /// it resolves and find the moment it asked for. One handed in after it
    /// is not waited for.
    #[tokio::test]
    async fn a_flush_resolves_after_the_pieces_before_it_are_written() {
        let log = Log::default();
        let mut sink = Sink::writing("g1", Slow(log.clone()));
        sink.take(piece("g1", 0, 10));
        sink.take(piece("g1", 1, 10));
        let (done, flushed) = oneshot::channel();
        sink.flushed(done);
        sink.take(piece("g1", 2, 10));

        flushed.await.expect("the flush resolves");
        assert_eq!(log.said(), ["open", "piece 0", "piece 1"]);
        let (done, flushed) = oneshot::channel();
        sink.flushed(done);
        flushed.await.expect("the flush resolves");
        assert_eq!(log.said(), ["open", "piece 0", "piece 1", "piece 2"]);
    }

    /// A flush the engine answered with no piece (nothing was waiting)
    /// resolves at once and starts no record: there is none to start. The
    /// first piece after it does, once.
    #[tokio::test]
    async fn a_flush_before_any_piece_starts_no_record() {
        let log = Log::default();
        let mut sink = Sink::writing("g1", Slow(log.clone()));
        let (done, flushed) = oneshot::channel();
        sink.flushed(done);
        flushed.await.expect("the flush resolves");
        assert!(log.said().is_empty(), "{:?}", log.said());

        sink.take(piece("g1", 0, 10));
        let (done, flushed) = oneshot::channel();
        sink.flushed(done);
        sink.take(piece("g1", 1, 10));
        let (done, again) = oneshot::channel();
        sink.flushed(done);
        flushed.await.unwrap();
        again.await.unwrap();
        assert_eq!(log.said(), ["open", "piece 0", "piece 1"]);
    }

    /// An ask is answered only by the game it was made of, once, and a
    /// forgotten one by nobody.
    #[test]
    fn an_ask_is_answered_by_its_own_game_once() {
        let flushes = Flushes::default();
        let (a, _rx_a) = flushes.ask("g1");
        let (b, _rx_b) = flushes.ask("g2");
        let (a, b_nonce) = (a.nonce, b.nonce);
        assert_ne!(a, b_nonce, "every ask has its own nonce");
        assert!(
            flushes.answered("g2", a).is_none(),
            "another game's engine cannot answer it"
        );
        assert!(flushes.answered("g1", a).is_some());
        assert!(flushes.answered("g1", a).is_none(), "once");
        assert_eq!(flushes.open(), 1, "the other is still open");
        drop(b);
        assert!(
            flushes.answered("g2", b_nonce).is_none(),
            "an ask whose report stopped waiting"
        );
        assert_eq!(flushes.open(), 0, "nothing left behind");
    }
}
