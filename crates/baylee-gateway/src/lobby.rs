//! The lobby: open games and seat bindings. A lobby game becomes a
//! hosted `Session` once its seats are filled; seat tokens (256-bit,
//! stored hashed) bind a websocket to exactly one seat of one account.

use baylee_core::preset::GamePreset;
use baylee_gamehost::Session;
use std::collections::HashMap;

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

/// A lobby game.
pub struct LobbyGame {
    /// Game id (`UUIDv7`).
    pub id: String,
    /// State.
    pub state: LobbyState,
    /// Seats (2).
    pub seats: Vec<LobbySeat>,
    /// The preset the session was built from (present when `Playing`).
    pub preset: Option<GamePreset>,
    /// The hosted game session (present when `Playing`).
    pub session: Option<Session>,
    /// Spectator-facing log seq (protocol v2).
    #[allow(dead_code)]
    pub created_at: u64,
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
            id,
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
            preset: None,
            session: None,
            created_at,
        }
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
