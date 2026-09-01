//! The lobby: open games and seat bindings.
//!
//! A lobby game becomes a *running* game once its seats are filled: the
//! gateway asks an agent to start an engine for it, that engine dials back,
//! and from then on the gateway only routes. Seat tokens (256-bit, stored
//! hashed) bind a websocket to exactly one seat of one account; the engine
//! token does the same for the one process allowed to play the game.

use baylee_core::preset::GamePreset;
use baylee_protocol::v1::Envelope;
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc, watch};

/// Lobby state of a game.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum LobbyState {
    /// Waiting for a second seat.
    Waiting,
    /// Both seats filled; the game is running.
    Playing,
    /// Finished.
    Over,
}

/// A seat in a lobby game.
#[derive(Clone, Debug)]
pub struct LobbySeat {
    /// Seat index (0/1).
    pub seat: usize,
    /// Account id when a human took the seat (`None` = AI or open).
    pub account_id: Option<String>,
    /// SHA-256 of the seat token (empty until issued).
    pub seat_token_hash: Option<String>,
    /// The deck the seat plays.
    #[allow(dead_code)]
    pub deck_name: String,
    /// The seat's full deck (present for human seats; used to build the
    /// preset when the game starts).
    pub deck: Option<crate::store::Deck>,
}

/// The gateway's end of one engine process.
///
/// Everything the gateway has to say to a game goes down this channel, so a
/// game with no link is a game nobody can play — which is exactly what a seat
/// socket waits for before it announces itself.
pub type EngineLink = mpsc::UnboundedSender<Envelope>;

/// A lobby game.
pub struct LobbyGame {
    /// Game id (`UUIDv7`).
    pub id: String,
    /// State.
    pub state: LobbyState,
    /// Seats (2).
    pub seats: Vec<LobbySeat>,
    /// The preset the engine is asked to build the game from (present once
    /// both seats are decided).
    pub preset: Option<GamePreset>,
    /// SHA-256 of the token the engine proves itself with. One game's worth
    /// of authority: it is issued when the engine is ordered and is useless
    /// for anything but attaching to this game.
    pub engine_token_hash: Option<String>,
    /// The agent that was asked to run this game, so the gateway knows who to
    /// tell when it is over.
    pub agent_id: Option<String>,
    /// The engine process, once it has dialled in.
    pub engine: Option<EngineLink>,
    /// Flips to true when an engine is attached. A seat socket may open the
    /// moment the lobby says "playing", which is before the engine exists;
    /// this is what it waits on rather than polling.
    pub ready: watch::Sender<bool>,
    /// Per-game update fan-out: every `(seat, encoded envelope)` the engine
    /// produces is broadcast here, so every connected seat socket receives
    /// its own messages — not just the seat that happened to act (human-vs-
    /// human depends on this; filtering per-socket used to drop the
    /// opponent's envelopes entirely).
    ///
    /// The payload is the encoded player-facing envelope, not a decoded one:
    /// the gateway forwards the bytes the engine handed it and never has to
    /// understand them.
    pub updates: broadcast::Sender<(u8, Vec<u8>)>,
    /// When the game was created (unix seconds).
    pub created_at: u64,
    /// When the game ended (unix seconds), for the cleanup grace period.
    pub finished_at: Option<u64>,
}

impl LobbyGame {
    /// A waiting game with the first seat taken.
    #[must_use]
    pub fn waiting(
        id: String,
        account_id: String,
        deck_name: String,
        deck: crate::store::Deck,
        created_at: u64,
    ) -> Self {
        Self {
            state: LobbyState::Waiting,
            seats: vec![
                LobbySeat {
                    seat: 0,
                    account_id: Some(account_id),
                    seat_token_hash: None,
                    deck_name,
                    deck: Some(deck),
                },
                LobbySeat {
                    seat: 1,
                    account_id: None,
                    seat_token_hash: None,
                    deck_name: String::new(),
                    deck: None,
                },
            ],
            ..Self::blank(id, created_at)
        }
    }

    /// A game whose seats are decided and whose engine has been ordered.
    #[must_use]
    pub fn playing(id: String, seats: Vec<LobbySeat>, preset: GamePreset, created_at: u64) -> Self {
        Self {
            state: LobbyState::Playing,
            seats,
            preset: Some(preset),
            ..Self::blank(id, created_at)
        }
    }

    /// The fields every game starts with, whatever else is true of it.
    fn blank(id: String, created_at: u64) -> Self {
        Self {
            id,
            state: LobbyState::Waiting,
            seats: Vec::new(),
            preset: None,
            engine_token_hash: None,
            agent_id: None,
            engine: None,
            ready: watch::channel(false).0,
            updates: broadcast::channel(256).0,
            created_at,
            finished_at: None,
        }
    }

    /// Marks the game finished. Idempotent: the engine says a game is over
    /// and its socket then closes, and both paths land here.
    pub fn finish(&mut self, now: u64) {
        if self.state != LobbyState::Over {
            self.state = LobbyState::Over;
            self.finished_at = Some(now);
        }
        self.engine = None;
        let _ = self.ready.send(false);
    }
}

/// The lobby registry.
#[derive(Default)]
pub struct Lobby {
    /// Games by id.
    pub games: HashMap<String, LobbyGame>,
}

impl Lobby {
    /// Games visible in the lobby (waiting or playing).
    #[must_use]
    pub fn list(&self) -> Vec<serde_json::Value> {
        self.games
            .values()
            .filter(|g| g.state != LobbyState::Over)
            .map(|g| {
                serde_json::json!({
                    "id": g.id,
                    "state": match g.state {
                        LobbyState::Waiting => "waiting",
                        LobbyState::Playing => "playing",
                        LobbyState::Over => "over",
                    },
                    "seats": g.seats.iter().map(|s| {
                        serde_json::json!({
                            "seat": s.seat,
                            "taken": s.account_id.is_some(),
                        })
                    }).collect::<Vec<_>>(),
                })
            })
            .collect()
    }
}
