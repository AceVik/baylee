//! A chair taken on its host's ticket, refused for a while: the seat stays.
//!
//! The gateway answers `401` to a chair's token when it will not have the
//! chair any more (the host took it back, the host's sign-in ended) and also
//! when it could not check it (a failed Central check). Until 22.10.2026 a
//! host's LLM seat read the first `401` as its room closing and left the
//! table for good; with a check behind every socket every five seconds, the
//! owner's seat "keeps losing its connection". It now leaves only once the
//! refusal has lasted its patience.

use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use baylee_seat::bridge::{self, PlayOptions, Played};
use baylee_seat::deck::Deck;
use baylee_seat::link::SeatLink;
use baylee_seat::lobby::{Chair, Lobby};
use baylee_seat::{BridgeConfig, Mind, ScriptedMind, SeatCore, Transcript};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const GAME: &str = "01a0ef91-74c7-7712-8cf4-b3a82c245612";

/// What the stand-in answers, call by call.
#[derive(Clone, Copy)]
struct Script {
    /// Ticket calls (counted from 0) answered `401`: `from..until`.
    tickets_refused: (usize, usize),
    /// Chair reads answered `401`: `from..until`.
    chair_refused: (usize, usize),
    /// Chair reads from this one on find the room gone (`404`).
    gone_from: usize,
}

/// The stand-in gateway, and what it counted.
#[derive(Clone)]
struct Stage {
    script: Script,
    tickets: Arc<AtomicUsize>,
    chair_reads: Arc<AtomicUsize>,
    sockets: Arc<AtomicUsize>,
}

impl Stage {
    fn new(script: Script) -> Self {
        Self {
            script,
            tickets: Arc::default(),
            chair_reads: Arc::default(),
            sockets: Arc::default(),
        }
    }

    fn count(of: &AtomicUsize) -> usize {
        of.load(Ordering::SeqCst)
    }

    async fn open(&self) -> String {
        let app = Router::new()
            .route("/ws-ticket", post(ticket))
            .route("/lobby/games/{id}/chair", get(chair))
            .route("/games/{id}/ws", get(seat_socket))
            .with_state(self.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await });
        address
    }
}

fn within((from, until): (usize, usize), call: usize) -> bool {
    (from..until).contains(&call)
}

fn refused() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "the seat token is not good for this table" })),
    )
        .into_response()
}

async fn ticket(State(stage): State<Stage>) -> Response {
    let call = stage.tickets.fetch_add(1, Ordering::SeqCst);
    if within(stage.script.tickets_refused, call) {
        return refused();
    }
    Json(serde_json::json!({ "ticket": "a-ticket" })).into_response()
}

async fn chair(State(stage): State<Stage>) -> Response {
    let read = stage.chair_reads.fetch_add(1, Ordering::SeqCst);
    if read >= stage.script.gone_from {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "no such game" })),
        )
            .into_response();
    }
    if within(stage.script.chair_refused, read) {
        return refused();
    }
    Json(serde_json::json!({ "state": "playing" })).into_response()
}

async fn seat_socket(ws: WebSocketUpgrade, State(stage): State<Stage>) -> Response {
    ws.on_upgrade(move |socket| play_socket(socket, stage))
}

/// The first socket closes at once, as one the gateway's check closed does;
/// the next ones stay open and silent until the seat hangs up.
async fn play_socket(mut socket: WebSocket, stage: Stage) {
    if stage.sockets.fetch_add(1, Ordering::SeqCst) == 0 {
        return;
    }
    while let Some(Ok(_)) = socket.recv().await {}
}

/// Plays the stand-in's table to the seat's end, as a chair taken on its
/// host's ticket (no session), leaving after `patience` of refusals.
async fn sit_at(stage: &Stage, patience: Duration) -> anyhow::Result<Played> {
    let lobby = Lobby::new(&stage.open().await);
    let chair = Chair {
        game_id: GAME.into(),
        seat: 1,
        seat_token: "a-seat-token".into(),
    };
    let mut link = SeatLink::new(lobby, chair, None).with_refusal_patience(patience);
    let deck = Deck::acceptance("Victory").unwrap();
    let mind = Arc::new(ScriptedMind::idle());
    let core = SeatCore::new(BridgeConfig::default(), deck.list, mind.disclosure());
    let options = PlayOptions {
        min_think: Duration::ZERO,
        room_check: Duration::from_millis(50),
        ..PlayOptions::default()
    };
    let mut transcript = Transcript::memory();
    tokio::time::timeout(
        Duration::from_secs(30),
        bridge::play(&mut link, core, mind, &mut transcript, &options),
    )
    .await
    .expect("the seat was still at the table long after the room closed")
}

/// Refused for a moment, ticket and chair alike, the seat asks again, dials
/// again, sits down on a second socket, and leaves only when the room is
/// gone. On the old code it left at the first refusal, on one socket.
#[tokio::test]
async fn a_chair_refused_for_a_moment_dials_again_and_stays() {
    let stage = Stage::new(Script {
        tickets_refused: (1, 3),
        chair_refused: (0, 3),
        gone_from: 8,
    });
    let played = sit_at(&stage, Duration::from_secs(10))
        .await
        .expect("the seat left without an error");
    assert!(played.result.is_none(), "{played:?}");
    assert_eq!(Stage::count(&stage.sockets), 2, "it sat down again");
    assert_eq!(played.dials, 2, "{played:?}");
    assert_eq!(Stage::count(&stage.tickets), 4, "two refused, then taken");
    assert!(
        Stage::count(&stage.chair_reads) > stage.script.gone_from,
        "and it left when the room was gone: {} reads",
        Stage::count(&stage.chair_reads)
    );
}

/// Refused for good, the seat still leaves: once the refusal has lasted its
/// patience, having asked and dialled again in the meantime.
#[tokio::test]
async fn a_chair_refused_for_good_leaves_after_its_patience() {
    let stage = Stage::new(Script {
        tickets_refused: (1, usize::MAX),
        chair_refused: (0, usize::MAX),
        gone_from: usize::MAX,
    });
    let played = sit_at(&stage, Duration::from_millis(300))
        .await
        .expect("the seat left without an error");
    assert!(played.result.is_none(), "{played:?}");
    assert_eq!(Stage::count(&stage.sockets), 1);
    assert!(
        Stage::count(&stage.tickets) >= 2,
        "it dialled again before leaving: {} tickets",
        Stage::count(&stage.tickets)
    );
}
