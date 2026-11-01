//! The table's reconnect window and the resume that preserves a decision.
//! A stand-in supplies the transport, never a production gateway or account.

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use baylee_client_core::reconnect::Window;
use baylee_client_core::test_support::{ViewBuilder, statics};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction, YesNoPrompt};
use baylee_protocol::v1;
use baylee_seat::bridge::{self, PlayOptions};
use baylee_seat::deck::Deck;
use baylee_seat::link::SeatLink;
use baylee_seat::lobby::{Chair, Lobby};
use baylee_seat::{BridgeConfig, Mind, ScriptedMind, SeatCore, Transcript};
use prost::Message as _;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::Instant;

const GAME: &str = "01a0ef91-74c7-7712-8cf4-b3a82c245612";
const ME: PlayerId = PlayerId::new(0);

#[derive(Clone)]
struct Gate {
    accept_after: Option<Duration>,
    started: Instant,
    calls: Arc<AtomicUsize>,
}

impl Gate {
    fn new(accept_after: Option<Duration>) -> Self {
        Self {
            accept_after,
            started: Instant::now(),
            calls: Arc::default(),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    async fn open(&self) -> String {
        let app = Router::new()
            .route("/ws-ticket", post(ticket))
            .route("/lobby/games/{id}/chair", get(chair))
            .route("/games/{id}/ws", get(gate_socket))
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

async fn chair() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "state": "playing" }))
}

async fn gate_socket(ws: WebSocketUpgrade, State(gate): State<Gate>) -> Response {
    gate.calls.fetch_add(1, Ordering::SeqCst);
    if gate
        .accept_after
        .is_some_and(|after| gate.started.elapsed() < after)
    {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    ws.on_upgrade(|mut socket| async move { while let Some(Ok(_)) = socket.recv().await {} })
}

async fn connect(
    window: Window,
    accept_after: Option<Duration>,
) -> (anyhow::Result<Option<baylee_seat::link::Socket>>, Gate) {
    let gate = Gate::new(accept_after);
    let lobby = Lobby::new(&gate.open().await);
    let chair = Chair {
        game_id: GAME.into(),
        seat: 0,
        seat_token: "a-seat-token".into(),
    };
    let mut link = SeatLink::new(lobby, chair, None);
    link.hold_for(window);
    let result = link.connect().await;
    (result, gate)
}

/// Report 01a162c8: a known 180-second window does not end at twelve dials.
#[tokio::test(start_paused = true)]
async fn a_known_window_keeps_dialling_past_the_old_limit() {
    let (result, gate) = connect(Window::secs(180), Some(Duration::from_secs(150))).await;
    let _socket = result
        .expect("the seat did not fail before its window ended")
        .expect("the delayed table accepted the seat");
    assert!(gate.calls() > 12, "only {} dials", gate.calls());
    assert!(gate.started.elapsed() >= Duration::from_secs(150));
}

/// The same known window still ends in a controlled error once it has passed.
#[tokio::test(start_paused = true)]
async fn a_known_window_stops_after_its_deadline() {
    let (result, gate) = connect(Window::secs(180), Some(Duration::from_secs(3600))).await;
    assert!(result.is_err(), "an unreachable table is still an error");
    assert!(gate.calls() > 12, "only {} dials", gate.calls());
    assert!(gate.started.elapsed() >= Duration::from_secs(180));
}

/// Without `GameStatic` the old bounded schedule is unchanged.
#[tokio::test(start_paused = true)]
async fn an_unknown_window_keeps_the_old_bounded_schedule() {
    let (result, gate) = connect(Window::Unknown, Some(Duration::from_secs(3600))).await;
    assert!(result.is_err());
    assert!(gate.started.elapsed() < Duration::from_secs(180));
}

#[derive(Clone, Default)]
struct ResumeStage {
    sockets: Arc<AtomicUsize>,
    done: Arc<AtomicBool>,
    resumes: Arc<Mutex<Vec<u64>>>,
    actions: Arc<Mutex<Vec<PlayerAction>>>,
}

impl ResumeStage {
    async fn open(&self) -> String {
        let app = Router::new()
            .route("/ws-ticket", post(ticket))
            .route("/lobby/games/{id}/chair", get(resume_chair))
            .route("/games/{id}/ws", get(resume_socket))
            .with_state(self.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await });
        address
    }
}

async fn resume_chair(State(stage): State<ResumeStage>) -> Json<serde_json::Value> {
    let state = if stage.done.load(Ordering::SeqCst) {
        "finished"
    } else {
        "playing"
    };
    Json(serde_json::json!({ "state": state }))
}

async fn resume_socket(ws: WebSocketUpgrade, State(stage): State<ResumeStage>) -> Response {
    let number = stage.sockets.fetch_add(1, Ordering::SeqCst);
    ws.on_upgrade(move |socket| resume_exchange(socket, stage, number))
}

fn frame(msg: v1::envelope::Msg) -> Message {
    Message::Binary(v1::Envelope { msg: Some(msg) }.encode_to_vec().into())
}

fn static_frame() -> Message {
    let mut game = statics(0);
    game.reconnect_secs = Some(180);
    game.seats = [(ME, "TEST-seat"), (PlayerId::new(1), "Alice")]
        .into_iter()
        .map(|(player, name)| baylee_view::SeatIdentity {
            player,
            display_name: name.into(),
            is_ai: false,
            away: false,
            team: None,
        })
        .collect();
    frame(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
        game_id: game.game_id.clone(),
        view_version: game.view_version,
        static_json: serde_json::to_vec(&game).unwrap(),
    }))
}

fn view_frame(seq: u64) -> Message {
    let mut view = ViewBuilder::new(2).build();
    view.seq = seq;
    frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
        game_id: GAME.into(),
        seq,
        view_json: serde_json::to_vec(&view).unwrap(),
        log_json: Vec::new(),
    }))
}

fn question_frame(seq: u64, pending: &Pending) -> Message {
    frame(v1::envelope::Msg::ChoiceRequest(v1::ChoiceRequest {
        game_id: GAME.into(),
        seq,
        pending_json: serde_json::to_vec(&pending).unwrap(),
    }))
}

fn yes_no() -> Pending {
    Pending::YesNo {
        player: ME,
        prompt: YesNoPrompt::DrawOffer {
            proposer: PlayerId::new(1),
        },
        source: None,
    }
}

fn priority() -> Pending {
    Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            ..LegalActions::default()
        }),
    }
}

async fn resume_exchange(mut socket: WebSocket, stage: ResumeStage, number: usize) {
    if number == 0 {
        socket.send(static_frame()).await.unwrap();
        socket.send(view_frame(7)).await.unwrap();
        socket.send(question_frame(7, &yes_no())).await.unwrap();
        while let Some(Ok(Message::Binary(bytes))) = socket.recv().await {
            let envelope = v1::Envelope::decode(bytes).unwrap();
            if let Some(v1::envelope::Msg::PlayerAction(action)) = envelope.msg {
                let action = serde_json::from_slice::<PlayerAction>(&action.action_json).unwrap();
                stage.actions.lock().unwrap().push(action);
                return;
            }
        }
        return;
    }

    while let Some(Ok(Message::Binary(bytes))) = socket.recv().await {
        let envelope = v1::Envelope::decode(bytes).unwrap();
        if let Some(v1::envelope::Msg::Resume(resume)) = envelope.msg {
            stage.resumes.lock().unwrap().push(resume.last_seq);
            break;
        }
    }
    socket.send(view_frame(8)).await.unwrap();
    socket.send(question_frame(8, &priority())).await.unwrap();
    while let Some(Ok(Message::Binary(bytes))) = socket.recv().await {
        let envelope = v1::Envelope::decode(bytes).unwrap();
        if let Some(v1::envelope::Msg::PlayerAction(action)) = envelope.msg {
            let action = serde_json::from_slice::<PlayerAction>(&action.action_json).unwrap();
            stage.actions.lock().unwrap().push(action);
            stage.done.store(true, Ordering::SeqCst);
            return;
        }
    }
}

/// A Yes/No answer sent before a disconnect is not asked again on resume;
/// the next priority is answered once, with no duplicate decision.
#[tokio::test]
async fn resume_does_not_ask_the_old_question_again() {
    let stage = ResumeStage::default();
    let lobby = Lobby::new(&stage.open().await);
    let chair = Chair {
        game_id: GAME.into(),
        seat: 0,
        seat_token: "a-seat-token".into(),
    };
    let mut link = SeatLink::new(lobby, chair, None);
    let deck = Deck::acceptance("Victory").unwrap();
    let mind = Arc::new(ScriptedMind::idle().rule(|request| {
        matches!(request.pending, Pending::YesNo { .. }).then_some(PlayerAction::YesNo(true))
    }));
    let seen = mind.seen();
    let core = SeatCore::new(BridgeConfig::default(), deck.list, mind.disclosure());
    let options = PlayOptions {
        min_think: Duration::ZERO,
        room_check: Duration::from_millis(50),
        ..PlayOptions::default()
    };
    let mut transcript = Transcript::memory();
    let played = tokio::time::timeout(
        Duration::from_secs(15),
        bridge::play(&mut link, core, mind, &mut transcript, &options),
    )
    .await
    .expect("the seat resumed and finished the stand-in exchange")
    .expect("the seat did not leave on an error");

    assert!(played.result.is_none(), "{played:?}");
    assert_eq!(*stage.resumes.lock().unwrap(), [7]);
    assert_eq!(
        *stage.actions.lock().unwrap(),
        [PlayerAction::YesNo(true), PlayerAction::PassPriority]
    );
    assert_eq!(
        seen.lock()
            .unwrap()
            .iter()
            .filter(|seen| seen.seq == 7)
            .count(),
        1
    );
}
