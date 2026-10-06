//! A bridge held by the program that started it (`--tethered`): when that
//! program lets go of its stdin before the game begins, the bridge gives
//! its chair up, so a host's client can seat another in it
//! (`docs/llm-seat.md` §"A language model at your table").
//!
//! The real binary against a stand-in gateway that seats it in the chair it
//! asked for and never starts the game. The house plays: no model is asked.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const ROOM: &str = "01a0ef91-74c7-7712-8cf4-b3a82c245612";

/// What the stand-in was asked.
#[derive(Clone, Default)]
struct Stage {
    joined: Arc<Mutex<Vec<Value>>>,
    left: Arc<Mutex<usize>>,
}

async fn guest() -> Json<Value> {
    Json(json!({ "token": "a-session" }))
}

async fn deck() -> Json<Value> {
    Json(json!({ "deck_id": "a-deck" }))
}

async fn listing(State(stage): State<Stage>) -> Json<Value> {
    let taken = !stage.joined.lock().unwrap().is_empty() && *stage.left.lock().unwrap() == 0;
    Json(json!({ "games": [{
        "id": ROOM,
        "state": "waiting",
        "seats": [
            { "seat": 0, "kind": "human", "taken": true },
            { "seat": 1, "kind": "human", "taken": taken },
            { "seat": 2, "kind": "human", "taken": false },
        ],
    }]}))
}

async fn join(State(stage): State<Stage>, Json(body): Json<Value>) -> Json<Value> {
    let seat = body["seat"].as_u64().unwrap_or(2);
    stage.joined.lock().unwrap().push(body);
    Json(json!({ "seat": seat, "seat_token": "a-seat-token" }))
}

async fn ready() -> Json<Value> {
    Json(json!({}))
}

async fn leave(State(stage): State<Stage>, Path(_room): Path<String>) -> Json<Value> {
    *stage.left.lock().unwrap() += 1;
    Json(json!({}))
}

#[test]
fn a_tethered_bridge_let_go_before_its_game_gives_its_chair_up() {
    let stage = Stage::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let address = runtime.block_on(async {
        let app = Router::new()
            .route("/auth/guest", post(guest))
            .route("/decks", post(deck))
            .route("/lobby/games", get(listing))
            .route("/lobby/games/{id}/join", post(join))
            .route("/lobby/games/{id}/ready", post(ready))
            .route("/lobby/games/{id}/leave", post(leave))
            .with_state(stage.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await });
        address
    });
    let dir = std::env::temp_dir().join(format!("baylee-seat-tethered-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut bridge = Command::new(env!("CARGO_BIN_EXE_baylee-seat"))
        .args([
            "join",
            ROOM,
            "--mind",
            "house",
            "--chair",
            "1",
            "--tethered",
        ])
        .args(["--gateway", &address])
        // Nothing of the machine's own: no config directory, no key.
        .env_clear()
        .env("HOME", &dir)
        // Never the player's credential store.
        .env("BAYLEE_KEY_STORE", "off")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the bridge starts");
    let mut lines = BufReader::new(bridge.stdout.take().unwrap()).lines();
    let sat = lines
        .by_ref()
        .map_while(Result::ok)
        .find(|line| line.contains("sits in chair"))
        .expect("the bridge sits down");
    assert!(sat.contains("sits in chair 1 of room"), "{sat}");
    assert_eq!(
        stage.joined.lock().unwrap()[0]["seat"],
        json!(1),
        "the chair it was told"
    );
    assert_eq!(*stage.left.lock().unwrap(), 0);
    // The program lets go.
    drop(bridge.stdin.take());
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = bridge.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "the bridge did not stop");
        std::thread::sleep(Duration::from_millis(50));
    };
    let rest: Vec<String> = lines.map_while(Result::ok).collect();
    assert!(!status.success(), "{status}");
    assert_eq!(*stage.left.lock().unwrap(), 1, "{rest:?}");
    assert!(
        rest.iter().any(|line| line.contains("left the chair")),
        "{rest:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// What the stand-in saw of a bridge seated on its host's chair ticket.
#[derive(Clone, Default)]
struct TicketStage {
    /// The `Authorization` of each redemption, and its body.
    redeemed: Arc<Mutex<Vec<(String, Value)>>>,
    /// The `Authorization` of each chair leave.
    left: Arc<Mutex<Vec<String>>>,
    /// Guest sign-ins: none is wanted.
    guests: Arc<Mutex<usize>>,
}

fn authorization(headers: &axum::http::HeaderMap) -> String {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

async fn redeem(
    State(stage): State<TicketStage>,
    headers: axum::http::HeaderMap,
    Path((_room, seat)): Path<(String, u32)>,
    Json(body): Json<Value>,
) -> Json<Value> {
    stage
        .redeemed
        .lock()
        .unwrap()
        .push((authorization(&headers), body));
    Json(json!({ "game_id": ROOM, "seat": seat, "seat_token": "a-seat-token", "decide_secs": 120 }))
}

async fn chair(Path(_room): Path<String>) -> Json<Value> {
    Json(json!({ "game_id": ROOM, "seat": 1, "state": "waiting", "decide_secs": 120 }))
}

async fn chair_leave(
    State(stage): State<TicketStage>,
    headers: axum::http::HeaderMap,
    Path(_room): Path<String>,
) -> Json<Value> {
    stage.left.lock().unwrap().push(authorization(&headers));
    Json(json!({}))
}

async fn no_guest(State(stage): State<TicketStage>) -> (axum::http::StatusCode, Json<Value>) {
    *stage.guests.lock().unwrap() += 1;
    (
        axum::http::StatusCode::FORBIDDEN,
        Json(json!({ "error": "this gateway takes no guests" })),
    )
}

/// A bridge a host's client starts with `--chair-ticket` takes the ticket
/// from the first line of its stdin and shows it to the gateway once, as
/// `Authorization`, and nowhere else: not in its arguments, not on its
/// output with every log line on, not at a guest door it never knocks at.
/// Let go before the game, it gives the chair back with its seat token.
#[test]
#[allow(clippy::too_many_lines)] // a scenario against a stand-in, read top to bottom
fn a_chair_ticket_goes_in_on_stdin_and_nowhere_else() {
    use std::io::{Read as _, Write as _};
    let ticket = "5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed";
    let stage = TicketStage::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let address = runtime.block_on(async {
        let app = Router::new()
            .route("/auth/guest", post(no_guest))
            .route("/lobby/games/{id}/chairs/{seat}/redeem", post(redeem))
            .route("/lobby/games/{id}/chair", get(chair))
            .route("/lobby/games/{id}/chair/leave", post(chair_leave))
            .with_state(stage.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await });
        address
    });
    let dir = std::env::temp_dir().join(format!("baylee-seat-chair-ticket-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let args = [
        "join",
        ROOM,
        "--mind",
        "house",
        "--chair",
        "1",
        "--tethered",
        "--chair-ticket",
        "--gateway",
        &address,
    ];
    assert!(!args.iter().any(|arg| arg.contains(ticket)));
    let mut bridge = Command::new(env!("CARGO_BIN_EXE_baylee-seat"))
        .args(args)
        .env_clear()
        .env("HOME", &dir)
        .env("BAYLEE_KEY_STORE", "off")
        // Every line the bridge would log.
        .env("RUST_LOG", "trace")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the bridge starts");
    let mut stdin = bridge.stdin.take().unwrap();
    writeln!(stdin, "{ticket}").unwrap();
    stdin.flush().unwrap();
    let mut said = Vec::new();
    let mut lines = BufReader::new(bridge.stdout.take().unwrap()).lines();
    for line in lines.by_ref().map_while(Result::ok) {
        let sat = line.contains("sits in chair");
        said.push(line);
        if sat {
            break;
        }
    }
    assert!(
        said.iter()
            .any(|line| line.contains("sits in chair 1 of room")),
        "{said:?}"
    );
    {
        let redeemed = stage.redeemed.lock().unwrap();
        assert_eq!(redeemed.len(), 1, "one redemption");
        assert_eq!(redeemed[0].0, format!("Bearer {ticket}"));
        assert_eq!(redeemed[0].1["display_name"], json!("HOUSE-house"));
        assert!(
            redeemed[0].1["deck"]["cards"].is_array(),
            "the deck rides along"
        );
        assert!(
            !redeemed[0].1.to_string().contains(ticket),
            "the ticket is not in the body"
        );
    }
    assert_eq!(*stage.guests.lock().unwrap(), 0, "no guest sign-in");
    // The program lets go.
    drop(stdin);
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = bridge.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "the bridge did not stop");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(!status.success(), "{status}");
    said.extend(lines.map_while(Result::ok));
    let mut errors = String::new();
    bridge
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut errors)
        .unwrap();
    assert_eq!(
        *stage.left.lock().unwrap(),
        vec!["Bearer a-seat-token".to_string()],
        "the chair given back with its seat token: {said:?}"
    );
    assert!(
        said.iter().any(|line| line.contains("left the chair")),
        "{said:?}"
    );
    assert!(!errors.is_empty(), "the log was on");
    for line in said.iter().map(String::as_str).chain(errors.lines()) {
        assert!(!line.contains(ticket), "the ticket was shown: {line}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
