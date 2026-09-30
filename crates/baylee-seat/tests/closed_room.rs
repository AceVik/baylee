//! A room that closes without a word: the seat leaves it.
//!
//! Two endings reach a seat as silence. A table that could not start (a
//! seat never finished loading before the curtain) is ended by its engine
//! with one `Error` to the seats already there, and a game whose engine was
//! lost with nothing at all; either way the gateway keeps the seat socket
//! open, and the room only leaves the lobby. A socket opened to a finished
//! game is upgraded and then held until the gateway gives up waiting for
//! an engine, and closes. Found on a live `xtask dev-table --bridge`: the
//! bridge sat at a table that had never started, on a socket that would
//! never speak again.
//!
//! A real gateway takes two minutes to stage the first (the curtain) and
//! cannot stage the second on demand, so these tables are a stand-in: a
//! ticket, a lobby row and a seat socket that does what the test says.

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use baylee_protocol::v1;
use baylee_seat::bridge::{self, PlayOptions, Played};
use baylee_seat::deck::Deck;
use baylee_seat::link::SeatLink;
use baylee_seat::lobby::{Chair, Lobby, Session};
use baylee_seat::{BridgeConfig, Disclosure, ScriptedMind, SeatCore, Transcript};
use prost::Message as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const GAME: &str = "01a0ef91-74c7-7712-8cf4-b3a82c245611";

/// What the engine says to a seat when its table could not start.
const COULD_NOT_START: &str = "The table could not start: not every player finished loading. Return to the lobby and try again.";

/// What the stand-in's seat socket does once it is open.
#[derive(Clone, Copy)]
enum SocketPlay {
    /// Says the table could not start, and then nothing, and stays open.
    CouldNotStart,
    /// Closes at once, as a finished game's socket does.
    HangsUp,
}

/// The stand-in gateway, and what it counted.
#[derive(Clone)]
struct Stage {
    socket: SocketPlay,
    /// Lobby reads that still list the room as playing; later ones do not
    /// list it at all, as the lobby lists no finished room.
    playing_for: usize,
    lobby_reads: Arc<AtomicUsize>,
    sockets: Arc<AtomicUsize>,
}

impl Stage {
    fn new(socket: SocketPlay, playing_for: usize) -> Self {
        Self {
            socket,
            playing_for,
            lobby_reads: Arc::default(),
            sockets: Arc::default(),
        }
    }

    fn lobby_reads(&self) -> usize {
        self.lobby_reads.load(Ordering::SeqCst)
    }

    fn sockets(&self) -> usize {
        self.sockets.load(Ordering::SeqCst)
    }

    /// Serves the stand-in on a free loopback port; its address.
    async fn open(&self) -> String {
        let app = Router::new()
            .route("/ws-ticket", post(ticket))
            .route("/lobby/games", get(listing))
            .route("/games/{id}/ws", get(seat_socket))
            .with_state(self.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await });
        address
    }
}

async fn ticket() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ticket": "a-ticket" }))
}

async fn listing(State(stage): State<Stage>) -> Json<serde_json::Value> {
    let read = stage.lobby_reads.fetch_add(1, Ordering::SeqCst);
    let games = if read < stage.playing_for {
        serde_json::json!([{ "id": GAME, "state": "playing" }])
    } else {
        serde_json::json!([])
    };
    Json(serde_json::json!({ "games": games }))
}

async fn seat_socket(ws: WebSocketUpgrade, State(stage): State<Stage>) -> Response {
    ws.on_upgrade(move |socket| play_socket(socket, stage))
}

async fn play_socket(mut socket: WebSocket, stage: Stage) {
    stage.sockets.fetch_add(1, Ordering::SeqCst);
    match stage.socket {
        SocketPlay::CouldNotStart => {
            let said = v1::Envelope {
                msg: Some(v1::envelope::Msg::Error(v1::Error {
                    code: 1,
                    message: COULD_NOT_START.into(),
                })),
            };
            let bytes = said.encode_to_vec();
            if socket.send(Message::Binary(bytes.into())).await.is_err() {
                return;
            }
            // Open, and silent, until the seat hangs up.
            while let Some(Ok(_)) = socket.recv().await {}
        }
        SocketPlay::HangsUp => {}
    }
}

/// Plays the stand-in's table to the seat's end, or fails the test when the
/// seat is still sitting there long after the room closed.
async fn sit_at(stage: &Stage) -> (Played, Transcript) {
    let lobby = Lobby::new(&stage.open().await);
    let chair = Chair {
        game_id: GAME.into(),
        seat: 1,
        seat_token: "a-seat-token".into(),
    };
    let mut link = SeatLink::new(lobby, chair, Some(Session::from_token("a-session".into())));
    let deck = Deck::acceptance("Victory").unwrap();
    let core = SeatCore::new(BridgeConfig::default(), deck.list, Disclosure::Llm);
    let mind = Arc::new(ScriptedMind::idle());
    let options = PlayOptions {
        min_think: Duration::ZERO,
        room_check: Duration::from_millis(50),
        ..PlayOptions::default()
    };
    let mut transcript = Transcript::memory();
    let played = tokio::time::timeout(
        Duration::from_secs(20),
        bridge::play(&mut link, core, mind, &mut transcript, &options),
    )
    .await
    .expect("the seat was still at a closed room's table")
    .expect("the seat left without an error");
    (played, transcript)
}

fn wrote(transcript: &Transcript, event: &str) -> bool {
    let tag = format!("\"event\":\"{event}\"");
    transcript.lines().iter().any(|line| line.contains(&tag))
}

/// The engine's curtain failure: one `Error`, then an open socket that says
/// nothing more. The seat hears it, waits while the lobby still lists the
/// room as playing, and leaves once the room is gone, with no result.
#[tokio::test]
async fn a_table_that_could_not_start_is_left_once_its_room_is_gone() {
    let stage = Stage::new(SocketPlay::CouldNotStart, 3);
    let (played, transcript) = sit_at(&stage).await;
    assert!(played.result.is_none(), "{played:?}");
    assert_eq!(played.stats.outcome, None);
    assert_eq!(played.dials, 1, "the socket never dropped");
    assert_eq!(stage.sockets(), 1);
    // Three quiet spells found the room still playing, the fourth found it
    // gone: a seat does not leave a table on its first silence.
    assert_eq!(stage.lobby_reads(), 4);
    assert!(wrote(&transcript, "table_said"), "{:?}", transcript.lines());
    assert!(wrote(&transcript, "closed"), "{:?}", transcript.lines());
}

/// A finished game's socket opens and closes. The seat asks the lobby
/// before each dial, dials again while the room is still listed, and stops
/// dialling once it is not.
#[tokio::test]
async fn a_finished_rooms_socket_is_not_dialled_again() {
    let stage = Stage::new(SocketPlay::HangsUp, 1);
    let (played, transcript) = sit_at(&stage).await;
    assert!(played.result.is_none(), "{played:?}");
    assert_eq!(played.dials, 2, "{played:?}");
    assert_eq!(stage.sockets(), 2);
    assert_eq!(stage.lobby_reads(), 2);
    assert!(wrote(&transcript, "closed"), "{:?}", transcript.lines());
}
