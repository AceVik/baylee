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

/// What is meant to sit in a seat.
///
/// Separate from whether anyone *has*: a human seat with no account is a
/// chair waiting for someone, and an AI seat is filled the moment it is
/// configured. Collapsing the two would make "is this table full?" ask the
/// wrong question.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeatKind {
    /// A person, once one takes it.
    Human,
    /// The house AI, at a named difficulty.
    Ai,
}

/// A seat in a lobby game.
#[derive(Clone, Debug)]
pub struct LobbySeat {
    /// Seat index.
    pub seat: usize,
    /// Whether this chair is for a person or for the AI.
    pub kind: SeatKind,
    /// The AI's difficulty profile, by the key `AIProfile::NAMED` lists.
    /// `None` for a human seat.
    pub ai: Option<String>,
    /// Account id when a human took the seat (`None` = AI or open).
    pub account_id: Option<String>,
    /// SHA-256 of the seat token (empty until issued).
    pub seat_token_hash: Option<String>,
    /// The deck the seat plays.
    pub deck_name: String,
    /// The seat's full deck. Present once the seat has one — a human's own
    /// choice, or the deck the host gave an AI — and what the preset is built
    /// from when the game starts.
    pub deck: Option<crate::store::Deck>,
}

impl LobbySeat {
    /// An empty chair for a person.
    #[must_use]
    pub fn open(seat: usize) -> Self {
        Self {
            seat,
            kind: SeatKind::Human,
            ai: None,
            account_id: None,
            seat_token_hash: None,
            deck_name: String::new(),
            deck: None,
        }
    }

    /// Whether the seat is settled enough for the game to start: somebody is
    /// in it, and they have something to play.
    #[must_use]
    pub fn ready(&self) -> bool {
        match self.kind {
            SeatKind::Human => self.account_id.is_some() && self.deck.is_some(),
            // An AI the host gave no deck plays the house deck, so there is
            // nothing left to wait for.
            SeatKind::Ai => true,
        }
    }
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
    /// The account that opened the room and configures it. `None` for the
    /// two-seat tables that predate rooms.
    pub host: Option<String>,
    /// What the host called the table. Empty is fine; the list falls back to
    /// the host's name.
    pub name: String,
    /// Seats, in turn order. Two or more.
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
    /// A room: `chairs` seats, the host in the first one, the rest open.
    ///
    /// The host owns the table until it starts — who else may sit, which
    /// chairs the AI takes and at what difficulty. Everyone else configures
    /// exactly one thing, which is the deck they themselves will play.
    #[must_use]
    pub fn room(
        id: String,
        account_id: String,
        deck_name: String,
        deck: crate::store::Deck,
        chairs: usize,
        name: String,
        created_at: u64,
    ) -> Self {
        let mut seats: Vec<LobbySeat> = (0..chairs).map(LobbySeat::open).collect();
        if let Some(first) = seats.first_mut() {
            first.account_id = Some(account_id.clone());
            first.deck_name = deck_name;
            first.deck = Some(deck);
        }
        Self {
            state: LobbyState::Waiting,
            seats,
            host: Some(account_id),
            name,
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
            host: None,
            name: String::new(),
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
        // `send_replace` for the same reason as the attach path: a game that
        // ends while nobody is watching must still read "not ready".
        self.ready.send_replace(false);
    }
}

/// The lobby registry.
#[derive(Default)]
pub struct Lobby {
    /// Games by id.
    pub games: HashMap<String, LobbyGame>,
}

impl Lobby {
    /// Every account id sitting at a visible table.
    ///
    /// The caller resolves these to display names against the store, which
    /// this module has no business locking.
    #[must_use]
    pub fn seated_accounts(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .games
            .values()
            .filter(|g| g.state != LobbyState::Over)
            .flat_map(|g| g.seats.iter().filter_map(|s| s.account_id.clone()))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Games visible in the lobby (waiting or playing).
    ///
    /// `me` is the account asking, so a seat can say whether it is theirs
    /// without the answer having to carry anyone's account id. `names` maps
    /// the ids from [`Lobby::seated_accounts`] to display names: a room is
    /// arranged in the open, and "who is that" is answered with a name.
    #[must_use]
    pub fn list_for(&self, me: &str, names: &HashMap<String, String>) -> Vec<serde_json::Value> {
        self.games
            .values()
            .filter(|g| g.state != LobbyState::Over)
            .map(|g| {
                serde_json::json!({
                    "id": g.id,
                    "name": g.name,
                    "host": g.host.as_ref().and_then(|h| names.get(h)),
                    "yours": g.host.as_deref() == Some(me),
                    "state": match g.state {
                        LobbyState::Waiting => "waiting",
                        LobbyState::Playing => "playing",
                        LobbyState::Over => "over",
                    },
                    // Everything a player needs to decide whether to sit
                    // down: how many chairs, which are people, which are the
                    // AI and how hard, and what everyone brought. A room is
                    // configured in the open — that is what makes it a room
                    // rather than a matchmaking queue.
                    "seats": g.seats.iter().map(|s| {
                        serde_json::json!({
                            "seat": s.seat,
                            "kind": s.kind,
                            "ai": s.ai,
                            "taken": s.account_id.is_some(),
                            // A name, never an account id: the listing is
                            // public to every signed-in player, and knowing
                            // who is at a table does not require knowing
                            // their account.
                            "player": s.account_id.as_ref().and_then(|a| names.get(a)),
                            "you": s.account_id.as_deref() == Some(me),
                            "deck": s.deck_name,
                            "ready": s.ready(),
                        })
                    }).collect::<Vec<_>>(),
                })
            })
            .collect()
    }
}
