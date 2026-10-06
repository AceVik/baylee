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
