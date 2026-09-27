//! Storing a game's record (#315) as its engine sends it.
//!
//! The engine sends its record in pieces over the engine link
//! (`GameRecordChunk`); this keeps them in the database as they come, never
//! reading inside them. The writes go through one task per game behind a
//! channel, so a slow database never holds up the seat frames the same
//! socket carries. The task outlives the socket until it has written what it
//! was handed.
//!
//! Nothing here is reachable from a seat or the lobby: pieces go in, and
//! the one way out is `baylee_db::records::for_seated`, which only a bug
//! report from a player who sat at the game asks.

use baylee_protocol::v1;
use tokio::sync::mpsc;
use uuid::Uuid;

/// The most one game's record may take in the database, compressed. A
/// two-seat game of the acceptance decks sends 17–29 KB; this is a bound on
/// an engine that misbehaves, not on a long game.
pub const MAX_RECORD_BYTES: usize = 64 * 1024 * 1024;

/// One game's record on its way into the database.
pub struct Sink {
    game_id: String,
    tx: mpsc::UnboundedSender<v1::GameRecordChunk>,
    taken: usize,
    full: bool,
}

impl Sink {
    /// Starts the game's record with who sits where (`None` for a chair the
    /// house plays) and the task that writes it.
    pub fn open(db: sea_orm::DatabaseConnection, game_id: &str, seats: Vec<Option<Uuid>>) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<v1::GameRecordChunk>();
        let id = game_id.to_owned();
        tokio::spawn(async move {
            if let Err(e) = baylee_db::records::open(&db, &id, &seats).await {
                tracing::error!(game_id = id, "starting the game's record: {e}");
                return;
            }
            while let Some(piece) = rx.recv().await {
                if let Err(e) =
                    baylee_db::records::append(&db, &id, piece.seq, piece.data, piece.last).await
                {
                    tracing::error!(
                        game_id = id,
                        seq = piece.seq,
                        "storing the game's record: {e}"
                    );
                }
            }
        });
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
        let _ = self.tx.send(piece);
    }
}
