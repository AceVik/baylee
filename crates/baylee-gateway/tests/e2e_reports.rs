//! A game's record in the gateway's store (#315), and `POST /reports`
//! passing a report on to the feedback service (#307).
//!
//! The service is a stub on a port of the test's own that keeps what it was
//! sent; the gateway, its agent and the engine are real. So what these hold
//! is the whole path: a game played over the real sockets leaves its record
//! in the database, and a report about that game from a player who sat
//! there arrives at the service with the record, a pseudonym and no name.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use std::io::Read as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use base64::Engine as _;
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
use baylee_protocol::v1::{self, Envelope};
use common::{Gateway, Socket, attach_agent, http, json_field, login, spawn_gateway_with};
use futures_util::SinkExt;
use parking_lot::Mutex;
use prost::Message;

/// What the stub service was sent: the `Authorization` header and the body.
type Inbox = Arc<Mutex<Vec<(String, serde_json::Value)>>>;

/// How the stub answers: [`TAKES`], [`FAILS`] (500) or [`NO_ID`] (201
/// without a `report_id`).
type Mood = Arc<AtomicU8>;
const TAKES: u8 = 0;
const FAILS: u8 = 1;
const NO_ID: u8 = 2;

/// A feedback service that takes every report, numbering them `r0`, `r1`, …
async fn stub_service() -> (String, Inbox) {
    let (url, inbox, _) = moody_service().await;
    (url, inbox)
}

/// The same, answering as its [`Mood`] says; only a report it took is kept.
async fn moody_service() -> (String, Inbox, Mood) {
    let inbox: Inbox = Arc::default();
    let mood: Mood = Arc::default();
    let app = axum::Router::new()
        .route(
            "/intake/reports",
            axum::routing::post(
                |State((inbox, mood)): State<(Inbox, Mood)>,
                 headers: HeaderMap,
                 body: axum::body::Bytes| async move {
                    match mood.load(Ordering::SeqCst) {
                        FAILS => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                axum::Json(serde_json::json!({})),
                            );
                        }
                        NO_ID => return (StatusCode::CREATED, axum::Json(serde_json::json!({}))),
                        _ => {}
                    }
                    let auth = headers
                        .get("authorization")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or_default()
                        .to_owned();
                    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
                    let mut inbox = inbox.lock();
                    let id = format!("r{}", inbox.len());
                    inbox.push((auth, json));
                    (
                        StatusCode::CREATED,
                        axum::Json(serde_json::json!({ "report_id": id })),
                    )
                },
            ),
        )
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024 * 1024))
        .with_state((inbox.clone(), mood.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await });
    (url, inbox, mood)
}

fn with_service(label: &str, url: &str) -> Gateway {
    spawn_gateway_with(
        label,
        &[
            ("BAYLEE_FEEDBACK_URL", url.to_owned()),
            (
                "BAYLEE_FEEDBACK_TOKEN",
                "intake-secret-for-tests".to_owned(),
            ),
            ("BAYLEE_FEEDBACK_KEY", "pseudonym-key".to_owned()),
        ],
    )
}

fn deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"swamps","cards":["60 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// Plays a game against the house to the end, by conceding once the table
/// is open. Returns its id.
async fn a_finished_game(port: u16, token: &str) -> String {
    let deck_id = deck(port, token);
    let create = format!("{{\"deck_id\":\"{deck_id}\",\"mode\":\"ai\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(token), &create);
    assert_eq!(status, 200, "create ai game: {body}");
    let game = json_field(&body, "game_id").to_string();
    let seat_token = json_field(&body, "seat_token").to_string();
    let mut ws: Socket = common::dial_seat(port, &game, &seat_token).await;
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(&mut ws).await {
                Some(v1::envelope::Msg::GameStatic(_)) => {
                    common::send(
                        &mut ws,
                        &Envelope {
                            msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
                        },
                    )
                    .await;
                }
                Some(v1::envelope::Msg::Curtain(_)) => break,
                Some(_) => {}
                None => panic!("the socket closed before play was allowed"),
            }
        }
    })
    .await
    .expect("the table opened");
    let concede = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(&PlayerAction::Concede).unwrap(),
        })),
    };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        concede.encode_to_vec().into(),
    ))
    .await
    .expect("send the concession");
    // The record is omniscient (#315): through the end of the game, until
    // the socket closes or falls quiet, not one piece of it reaches a seat.
    while let Ok(Some(msg)) =
        tokio::time::timeout(std::time::Duration::from_secs(2), common::next_msg(&mut ws)).await
    {
        assert!(
            !matches!(msg, v1::envelope::Msg::GameRecordChunk(_)),
            "a seat was sent the game's record"
        );
    }
    game
}

/// Waits until the game's record is complete in the store.
async fn stored(gw: &Gateway, game: &str) {
    for _ in 0..common::WAIT_TRIES {
        let done = gw.scalar(&format!(
            "SELECT count(*) FROM game_record WHERE game_id = '{game}' AND complete"
        ));
        if done == 1 {
            return;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    panic!("the game's record was never completed");
}

fn report(port: u16, token: Option<&str>, body: &str) -> (u16, String) {
    http(port, "POST", "/reports", token, body)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_finished_game_leaves_its_record_and_a_report_about_it_carries_it() {
    let (url, inbox) = stub_service().await;
    let gw = with_service("reports-record", &url);
    let port = gw.port;
    let agent = attach_agent(&gw).await;
    let token = login(port, "rhea", "RheaReporter");
    let game = a_finished_game(port, &token).await;
    stored(&gw, &game).await;

    assert!(
        gw.scalar(&format!(
            "SELECT bytes FROM game_record WHERE game_id = '{game}'"
        )) > 0
    );
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM game_record_seat WHERE game_id = '{game}' AND account_id IS NOT NULL"
        )),
        1,
        "the player's seat is linked, the house's is not"
    );
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM game_record_seat WHERE game_id = '{game}'"
        )),
        2
    );

    let body = serde_json::json!({
        "kind": "bug",
        "text": "the swamp did a thing",
        "game_id": game,
        "client": { "version": "test", "os": "somewhere" },
    })
    .to_string();
    let (status, answer) = report(port, Some(&token), &body);
    assert_eq!(status, 201, "{answer}");
    assert_eq!(json_field(&answer, "report_id"), "r0");

    // A second report from the same player: the same pseudonym.
    let (status, _) = report(
        port,
        Some(&token),
        r#"{"kind":"feedback","text":"nice","game_id":null,"client":{}}"#,
    );
    assert_eq!(status, 201);

    // Someone who did not sit at the game names it: no record for them.
    let stranger = login(port, "sten", "StenStranger");
    let naming = serde_json::json!({
        "kind": "other", "text": "", "game_id": game, "client": {},
    })
    .to_string();
    assert_eq!(report(port, Some(&stranger), &naming).0, 201);

    let inbox = inbox.lock();
    assert_eq!(inbox.len(), 3);
    let (auth, sent) = &inbox[0];
    assert_eq!(auth, "Bearer intake-secret-for-tests");
    // Without the record, whose base64 may spell anything; it is read
    // decoded below.
    let mut rest = sent.clone();
    rest["record"] = serde_json::Value::Null;
    let raw = rest.to_string();
    for private in ["rhea", "RheaReporter", token.as_str()] {
        assert!(!raw.contains(private), "the report carried {private:?}");
    }
    assert_eq!(sent["kind"], "bug");
    assert_eq!(sent["game_id"], game.as_str());
    assert_eq!(sent["client"]["os"], "somewhere");
    assert!(
        sent["gateway"]["version"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    );
    let reporter = sent["reporter"].as_str().expect("a pseudonym");
    assert_eq!(reporter.len(), 64);
    assert_eq!(
        inbox[1].1["reporter"], reporter,
        "one pseudonym per account"
    );
    assert_ne!(inbox[2].1["reporter"], reporter);
    assert!(inbox[1].1["record"].is_null());
    assert!(
        inbox[2].1["record"].is_null(),
        "a stranger was sent the record"
    );

    assert_eq!(sent["record"]["complete"], true);
    let gz = base64::engine::general_purpose::STANDARD
        .decode(sent["record"]["gzip_base64"].as_str().expect("the record"))
        .expect("base64");
    let mut record = String::new();
    flate2::read::MultiGzDecoder::new(&gz[..])
        .read_to_string(&mut record)
        .expect("one gzip stream");
    let lines: Vec<serde_json::Value> = record
        .lines()
        .map(|l| serde_json::from_str(l).expect("a record line"))
        .collect();
    assert_eq!(lines.first().unwrap()["kind"], "header");
    assert_eq!(lines.last().unwrap()["kind"], "end");
    assert!(
        lines
            .iter()
            .any(|l| l["kind"] == "input" && l["action"] == "Concede")
    );
    assert!(!record.contains("RheaReporter"));
    drop(inbox);
    agent.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_report_is_refused_for_what_it_is() {
    let unconfigured = common::spawn_gateway("reports-off");
    let token = login(unconfigured.port, "ola", "Ola");
    let fine = r#"{"kind":"bug","text":"x","game_id":null,"client":{}}"#;
    let (status, body) = report(unconfigured.port, Some(&token), fine);
    assert_eq!(status, 503);
    assert!(body.contains("reports are not configured"), "{body}");

    let (url, inbox) = stub_service().await;
    let gw = with_service("reports-refused", &url);
    let port = gw.port;
    assert_eq!(report(port, None, fine).0, 401);
    let token = login(port, "pia", "Pia");
    for bad in [
        r#"{"kind":"rant","text":"x","game_id":null,"client":{}}"#,
        r#"{"kind":"bug","text":"x","game_id":null,"client":[]}"#,
        r#"{"kind":"bug","game_id":null,"client":{}}"#,
        &format!(
            r#"{{"kind":"bug","text":"x","game_id":"{}","client":{{}}}}"#,
            "g".repeat(129)
        ),
        "not json",
    ] {
        assert_eq!(report(port, Some(&token), bad).0, 400, "{bad}");
    }
    let long = serde_json::json!({
        "kind": "bug", "text": "é".repeat(20_001), "game_id": null, "client": {},
    })
    .to_string();
    assert_eq!(report(port, Some(&token), &long).0, 413);
    let exactly = serde_json::json!({
        "kind": "bug", "text": "é".repeat(20_000), "game_id": null, "client": {},
    })
    .to_string();
    assert_eq!(
        report(port, Some(&token), &exactly).0,
        201,
        "20000 characters fit"
    );
    let heavy = serde_json::json!({
        "kind": "crash", "text": "", "game_id": null,
        "client": { "log": "x".repeat(2 * 1024 * 1024) },
    })
    .to_string();
    assert_eq!(report(port, Some(&token), &heavy).0, 413);

    // A game id of exactly 128 characters is one; "" is none.
    let longest = format!(
        r#"{{"kind":"bug","text":"x","game_id":"{}","client":{{}}}}"#,
        "g".repeat(128)
    );
    assert_eq!(report(port, Some(&token), &longest).0, 201);
    assert_eq!(
        report(
            port,
            Some(&token),
            r#"{"kind":"bug","text":"x","game_id":"","client":{}}"#
        )
        .0,
        201
    );
    let inbox = inbox.lock();
    assert_eq!(inbox.len(), 3, "only what was taken arrived");
    assert_eq!(inbox[1].1["game_id"], "g".repeat(128));
    assert!(inbox[2].1["game_id"].is_null(), "an empty id is no id");
    drop(inbox);

    // A service that is not there takes nothing.
    let gone = with_service("reports-gone", "http://127.0.0.1:9");
    let token = login(gone.port, "ruth", "Ruth");
    assert_eq!(report(gone.port, Some(&token), fine).0, 502);
}

/// The budget (`REPORTS_PER_WINDOW`, 64 an hour): an account's 65th report
/// is refused and another account's first is not; a report the service did
/// not take, or that the gateway's database could not look up, costs
/// nothing of it, however many there are.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_budget_is_64_per_account_and_only_what_was_taken_counts() {
    let (url, inbox, mood) = moody_service().await;
    let gw = with_service("reports-budget", &url);
    let port = gw.port;
    let token = login(port, "pia", "Pia");
    let fine = r#"{"kind":"bug","text":"x","game_id":null,"client":{}}"#;

    // Seventy the service refused, seventy it answered without an id, and
    // seventy the gateway could not look the game up for.
    mood.store(FAILS, Ordering::SeqCst);
    for _ in 0..70 {
        assert_eq!(report(port, Some(&token), fine).0, 502);
    }
    mood.store(NO_ID, Ordering::SeqCst);
    for _ in 0..70 {
        assert_eq!(report(port, Some(&token), fine).0, 502);
    }
    mood.store(TAKES, Ordering::SeqCst);
    // A column, not the table: the harness's search_path ends in `public`,
    // so renaming the table away would let the lookup resolve a
    // `public.game_record` the dev database happens to have, and the
    // forced failure would depend on what public holds. The table still
    // resolves here; the column `for_seated` selects does not.
    gw.sql("ALTER TABLE game_record RENAME COLUMN complete TO complete_away");
    let about_a_game = r#"{"kind":"bug","text":"x","game_id":"g1","client":{}}"#;
    for _ in 0..70 {
        assert_eq!(report(port, Some(&token), about_a_game).0, 503);
    }
    gw.sql("ALTER TABLE game_record RENAME COLUMN complete_away TO complete");
    assert!(inbox.lock().is_empty());

    for i in 0..64 {
        assert_eq!(report(port, Some(&token), fine).0, 201, "report {i}");
    }
    assert_eq!(report(port, Some(&token), fine).0, 429, "the 65th");
    assert_eq!(
        report(port, Some(&token), about_a_game).0,
        429,
        "whatever it is about"
    );
    let other = login(port, "quin", "Quin");
    assert_eq!(
        report(port, Some(&other), fine).0,
        201,
        "another account is not held"
    );
    assert_eq!(inbox.lock().len(), 65);
}

/// The same path into the real service rather than the stub: what the
/// gateway sends is what the service takes (`docs/feedback.md`), and the
/// record it hands on is the one it stored.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_report_reaches_the_real_service_with_its_record() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let gw = with_service("reports-service", &url);
    // The service keeps tables of its own and may share a schema with a
    // gateway without either's migrator seeing the other's.
    let db = baylee_feedback::connect(&gw.database_url(), 2)
        .await
        .expect("the service migrates");
    let config = baylee_feedback::Config::new(
        "test-gw=intake-secret-for-tests",
        Some("a-read-token-for-tests"),
        None,
    )
    .expect("config");
    let app = baylee_feedback::app(Arc::new(baylee_feedback::AppState { db, config }));
    tokio::spawn(async move { axum::serve(listener, app).await });

    let port = gw.port;
    let agent = attach_agent(&gw).await;
    let token = login(port, "tove", "Tove");
    let game = a_finished_game(port, &token).await;
    stored(&gw, &game).await;
    let body = serde_json::json!({
        "kind": "crash", "text": "it fell over", "game_id": game,
        "client": { "panic": "index out of bounds" },
    })
    .to_string();
    let (status, answer) = report(port, Some(&token), &body);
    assert_eq!(status, 201, "{answer}");
    let id = json_field(&answer, "report_id").to_string();

    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM feedback_report WHERE id = '{id}' AND gateway = 'test-gw' \
             AND kind = 'crash' AND record_complete AND client->>'panic' = 'index out of bounds'"
        )),
        1
    );
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM feedback_report f, game_record g \
             WHERE f.id = '{id}' AND g.game_id = '{game}' AND octet_length(f.record) = g.bytes"
        )),
        1,
        "the service holds the record the gateway stored"
    );
    agent.abort();
}

/// How long the gateway waits for an engine's record before a report goes
/// without it (`report::FLUSH_WAIT`, which a test cannot import from the
/// gateway's binary).
const FLUSH_WAIT: std::time::Duration = std::time::Duration::from_secs(3);

/// A game against the house that goes on: the table is open and the seat
/// has fallen quiet on its first question, which is returned unanswered.
async fn a_game_that_goes_on(port: u16, token: &str) -> (String, Socket, Pending) {
    let deck_id = deck(port, token);
    let create = format!("{{\"deck_id\":\"{deck_id}\",\"mode\":\"ai\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(token), &create);
    assert_eq!(status, 200, "create ai game: {body}");
    let game = json_field(&body, "game_id").to_string();
    let seat_token = json_field(&body, "seat_token").to_string();
    let mut ws: Socket = common::dial_seat(port, &game, &seat_token).await;
    let mut asked = None;
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(&mut ws).await {
                Some(v1::envelope::Msg::GameStatic(_)) => {
                    common::send(
                        &mut ws,
                        &Envelope {
                            msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
                        },
                    )
                    .await;
                }
                Some(v1::envelope::Msg::Curtain(_)) => break,
                Some(v1::envelope::Msg::ChoiceRequest(c)) => {
                    asked = serde_json::from_slice(&c.pending_json).ok();
                }
                Some(_) => {}
                None => panic!("the socket closed before play was allowed"),
            }
        }
    })
    .await
    .expect("the table opened");
    let asked = quiet(&mut ws).await.or(asked).expect("the seat is asked");
    (game, ws, asked)
}

/// Reads a seat until it falls quiet: the last question it was asked. No
/// piece of the record, and nothing about asking for one, reaches a seat.
async fn quiet(ws: &mut Socket) -> Option<Pending> {
    let mut asked = None;
    while let Ok(Some(msg)) =
        tokio::time::timeout(std::time::Duration::from_millis(700), common::next_msg(ws)).await
    {
        match msg {
            v1::envelope::Msg::GameRecordChunk(_)
            | v1::envelope::Msg::FlushRecord(_)
            | v1::envelope::Msg::RecordFlushed(_) => {
                panic!("a seat was sent something of the game's record: {msg:?}")
            }
            v1::envelope::Msg::ChoiceRequest(c) => {
                asked = serde_json::from_slice(&c.pending_json).ok();
            }
            _ => {}
        }
    }
    asked
}

/// The record a report arrived at the service with, unpacked.
fn record_sent(sent: &serde_json::Value) -> Vec<u8> {
    let gz = base64::engine::general_purpose::STANDARD
        .decode(sent["record"]["gzip_base64"].as_str().expect("a record"))
        .expect("base64");
    let mut record = Vec::new();
    flate2::read::MultiGzDecoder::new(&gz[..])
        .read_to_end(&mut record)
        .expect("one gzip stream");
    record
}

/// The hash a record replays to.
fn replays_to(record: &[u8]) -> u64 {
    let replayed = baylee_gamehost::record::replay(record).expect("the record replays");
    assert!(!replayed.ended, "the game goes on");
    replayed.engine.snapshot_hash()
}

/// A seat's answer, as its socket sends it.
async fn say(ws: &mut Socket, action: &PlayerAction) {
    common::send(
        ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                game_id: String::new(),
                seat_token: String::new(),
                action_json: serde_json::to_vec(action).unwrap(),
            })),
        },
    )
    .await;
}

/// A report naming a game that goes on, from a player who sits at it, asks
/// that game's engine for its record once and carries it to the moment of
/// the report (#323). Nobody else's report asks: not a stranger's naming
/// the game, not one naming no game or another, and not one about a game
/// that ended, whose record is whole already.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_report_at_a_game_that_goes_on_asks_its_engine_for_the_moment() {
    let (url, inbox) = stub_service().await;
    let gw = with_service("reports-flush", &url);
    let port = gw.port;
    let (agent, probe) = common::attach_agent_probed(&gw).await;
    let token = login(port, "rhea", "RheaReporter");
    let (game, mut ws, _) = a_game_that_goes_on(port, &token).await;
    let now = probe.hash.borrow().expect("the game is built");
    let naming = serde_json::json!({
        "kind": "bug", "text": "now", "game_id": game, "client": {},
    })
    .to_string();

    let stranger = login(port, "sten", "StenStranger");
    assert_eq!(report(port, Some(&stranger), &naming).0, 201);
    let none = r#"{"kind":"bug","text":"x","game_id":null,"client":{}}"#;
    assert_eq!(report(port, Some(&token), none).0, 201);
    let other = r#"{"kind":"bug","text":"x","game_id":"no-such-game","client":{}}"#;
    assert_eq!(report(port, Some(&token), other).0, 201);
    assert_eq!(probe.flushes(), 0, "none of those asks the engine");

    let started = std::time::Instant::now();
    assert_eq!(report(port, Some(&token), &naming).0, 201);
    assert!(
        started.elapsed() < FLUSH_WAIT,
        "an answered ask ends the wait: {:?}",
        started.elapsed()
    );
    assert_eq!(probe.flushes(), 1, "the reporter's own game is asked once");
    {
        let inbox = inbox.lock();
        assert!(
            inbox[0].1["record"].is_null(),
            "a stranger was sent the record"
        );
        let sent = &inbox[3].1;
        assert_eq!(sent["record"]["complete"], false, "the game goes on");
        assert_eq!(
            replays_to(&record_sent(sent)),
            now,
            "the record reaches the moment of the report"
        );
    }
    assert!(
        quiet(&mut ws).await.is_none(),
        "the seat was asked nothing new"
    );

    // The game ends; a report about it asks nothing and carries it whole.
    say(&mut ws, &PlayerAction::Concede).await;
    quiet(&mut ws).await;
    stored(&gw, &game).await;
    assert_eq!(report(port, Some(&token), &naming).0, 201);
    assert_eq!(probe.flushes(), 1, "a game that ended is not asked");
    assert_eq!(inbox.lock()[4].1["record"]["complete"], true);
    agent.abort();
}

/// An engine that does not answer (busy, stuck, or from before the ask)
/// costs a report [`FLUSH_WAIT`] and never the report: it goes with the
/// record as stored, which is where the last answered ask left it. The
/// next ask the engine answers carries the game on to its own moment.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_report_whose_engine_does_not_answer_carries_what_was_stored() {
    let (url, inbox) = stub_service().await;
    let gw = with_service("reports-mute", &url);
    let port = gw.port;
    let (agent, probe) = common::attach_agent_probed(&gw).await;
    let token = login(port, "rhea", "RheaReporter");
    let (game, mut ws, asked) = a_game_that_goes_on(port, &token).await;
    let naming = serde_json::json!({
        "kind": "bug", "text": "now", "game_id": game, "client": {},
    })
    .to_string();

    let first = probe.hash.borrow().expect("the game is built");
    assert_eq!(report(port, Some(&token), &naming).0, 201);

    // The game moves on.
    say(&mut ws, &timeout_answer(&asked).expect("a plain answer")).await;
    quiet(&mut ws).await;
    let later = probe.hash.borrow().expect("the game is built");
    assert_ne!(later, first, "the game moved");

    probe.set_mute(true);
    let started = std::time::Instant::now();
    assert_eq!(
        report(port, Some(&token), &naming).0,
        201,
        "a report is never refused for its engine"
    );
    let waited = started.elapsed();
    assert_eq!(probe.flushes(), 2, "the engine was asked");
    assert!(
        waited >= FLUSH_WAIT && waited < FLUSH_WAIT * 3,
        "waited {waited:?} for an engine that never answers"
    );

    probe.set_mute(false);
    assert_eq!(report(port, Some(&token), &naming).0, 201);
    assert_eq!(probe.flushes(), 3);
    let hashes: Vec<u64> = inbox
        .lock()
        .iter()
        .map(|(_, sent)| {
            assert_eq!(sent["record"]["complete"], false);
            replays_to(&record_sent(sent))
        })
        .collect();
    assert_eq!(
        hashes,
        [first, first, later],
        "the unanswered report carries what was stored; the next goes on"
    );
    assert!(
        quiet(&mut ws).await.is_none(),
        "the seat was asked nothing new"
    );
    agent.abort();
}
