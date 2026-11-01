//! A host's seat bridge sending its game's bug report on its chair's seat
//! token (`POST /lobby/games/{id}/chair/report`, `docs/feedback.md`): a
//! chair taken on its host's ticket has no session, so its language
//! model's report goes on the token, under the pseudonym of the host who
//! answers for the chair (owner, 31.10.2026), with that game's record.
//!
//! The feedback service is a stub on a port of the test's own that keeps
//! what it was sent; the gateway, its agent and the engine are real.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use std::io::Read as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::extract::State;
use axum::http::StatusCode;
use base64::Engine as _;
use baylee_protocol::v1::{self, Envelope};
use baylee_protocol::{TICKET_REFUSED, chair_redeem_path, chair_ticket_path};
use common::central::{central_file, parts_arrive, stub_central};
use common::{Gateway, Socket, attach_agent, http, json_field, login, spawn_gateway_with};
use futures_util::SinkExt;
use parking_lot::Mutex;
use prost::Message;

/// What the stub service was sent.
type Inbox = Arc<Mutex<Vec<serde_json::Value>>>;

/// Whether the stub service is down (answers `500`, keeps nothing).
type Down = Arc<AtomicBool>;

/// A feedback service that takes every report while it is up, numbering
/// them `r0`, `r1`, …
async fn stub_service() -> (String, Inbox, Down) {
    let inbox: Inbox = Arc::default();
    let down: Down = Arc::default();
    let app = axum::Router::new()
        .route(
            "/intake/reports",
            axum::routing::post(
                |State((inbox, down)): State<(Inbox, Down)>, body: axum::body::Bytes| async move {
                    if down.load(Ordering::SeqCst) {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            axum::Json(serde_json::json!({})),
                        );
                    }
                    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
                    let mut inbox = inbox.lock();
                    let id = format!("r{}", inbox.len());
                    inbox.push(json);
                    (
                        StatusCode::CREATED,
                        axum::Json(serde_json::json!({ "report_id": id })),
                    )
                },
            ),
        )
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024 * 1024))
        .with_state((inbox.clone(), down.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await });
    (url, inbox, down)
}

/// What a bridge sits down with.
const REDEEM: &str = r#"{"display_name":"LLM-test","deck":{"name":"swamps","cards":["60 Swamp"]}}"#;

/// A room of two chairs `host` opened and sits in (chair 0).
fn room(port: u16, host: &str) -> String {
    let body = r#"{"name":"forests","cards":["60 Forest"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(host), body);
    assert_eq!(status, 200, "create deck: {body}");
    let deck_id = json_field(&body, "deck_id");
    let create = format!("{{\"deck_id\":\"{deck_id}\",\"seats\":2,\"name\":\"bridges\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(host), &create);
    assert_eq!(status, 200, "create room: {body}");
    json_field(&body, "game_id").to_string()
}

/// Opens a seat and says ready once the table is shown.
async fn sit(port: u16, game: &str, seat_token: &str) -> Socket {
    let mut ws = common::dial_seat(port, game, seat_token).await;
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
                    break;
                }
                Some(_) => {}
                None => panic!("the socket closed before the table was shown"),
            }
        }
    })
    .await
    .expect("the table was shown");
    ws
}

/// Waits for the curtain on `ws`.
async fn curtain(ws: &mut Socket) {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(ws).await {
                Some(v1::envelope::Msg::Curtain(_)) => break,
                Some(_) => {}
                None => panic!("the socket closed before play was allowed"),
            }
        }
    })
    .await
    .expect("the table opened");
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

/// A game `host` played against its own seat bridge in chair 1, on the
/// host's chair ticket, to the end (the host conceded): the game, the
/// bridge's seat token and the host's own.
async fn a_finished_bridge_game(gw: &Gateway, host: &str) -> (String, String, String) {
    let port = gw.port;
    let game = room(port, host);
    let (status, body) = http(port, "POST", &chair_ticket_path(&game, 1), Some(host), "{}");
    assert_eq!(status, 200, "a chair ticket: {body}");
    let ticket = json_field(&body, "ticket").to_string();
    let (status, body) = http(
        port,
        "POST",
        &chair_redeem_path(&game, 1),
        Some(&ticket),
        REDEEM,
    );
    assert_eq!(status, 200, "redeem: {body}");
    let bridge = json_field(&body, "seat_token").to_string();
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/chair/ready"),
        Some(&bridge),
        "{}",
    );
    assert_eq!(status, 204, "{body}");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/start"),
        Some(host),
        "{}",
    );
    assert_eq!(status, 200, "start: {body}");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/seat"),
        Some(host),
        "{}",
    );
    assert_eq!(status, 200, "{body}");
    let host_seat = json_field(&body, "seat_token").to_string();
    let mut host_ws = sit(port, &game, &host_seat).await;
    let _bridge_ws = sit(port, &game, &bridge).await;
    curtain(&mut host_ws).await;
    let concede = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(&baylee_engine::choice::PlayerAction::Concede).unwrap(),
        })),
    };
    host_ws
        .send(tokio_tungstenite::tungstenite::Message::Binary(
            concede.encode_to_vec().into(),
        ))
        .await
        .expect("concede");
    stored(gw, &game).await;
    (game, bridge, host_seat)
}

/// A report about `game_id` (JSON), as a bridge sends it.
fn body(game_id: &serde_json::Value) -> String {
    serde_json::json!({
        "kind": "bug",
        "text": "Untrusted: reports written by a language model.",
        "game_id": game_id,
        "client": { "source": "llm-seat", "seat": 1 },
    })
    .to_string()
}

/// The bridge's report goes on its seat token, after the game, under the
/// host's pseudonym and with that game's record; once, about that game
/// only, and never on a person's chair, another game's door or a token
/// that opens nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn a_hosts_bridge_reports_on_its_seat_token_under_the_hosts_pseudonym() {
    let (url, inbox, down) = stub_service().await;
    let gw = spawn_gateway_with(
        "chair_report",
        &[
            ("BAYLEE_FEEDBACK_URL", url),
            (
                "BAYLEE_FEEDBACK_TOKEN",
                "intake-secret-for-tests".to_owned(),
            ),
            ("BAYLEE_FEEDBACK_KEY", "pseudonym-key".to_owned()),
        ],
    );
    let port = gw.port;
    let agent = attach_agent(&gw).await;
    let host = login(port, "hana", "HanaHost");
    let (game, bridge, host_seat) = a_finished_bridge_game(&gw, &host).await;
    let other = login(port, "otto", "OttoOther");
    let elsewhere = room(port, &other);
    let path = format!("/lobby/games/{game}/chair/report");
    let null = serde_json::Value::Null;

    // The game is over, and the chair still asks after it.
    let (status, answer) = http(
        port,
        "GET",
        &format!("/lobby/games/{game}/chair"),
        Some(&bridge),
        "",
    );
    assert_eq!(status, 200, "{answer}");
    assert_eq!(json_field(&answer, "state"), "over");

    // No token, a token that opens nothing, and the bridge's token at
    // another game's door.
    assert_eq!(http(port, "POST", &path, None, &body(&null)).0, 401);
    let (status, answer) = http(port, "POST", &path, Some(&"0".repeat(64)), &body(&null));
    assert_eq!(status, 401, "{answer}");
    assert!(answer.contains(TICKET_REFUSED), "{answer}");
    let (status, answer) = http(
        port,
        "POST",
        &format!("/lobby/games/{elsewhere}/chair/report"),
        Some(&bridge),
        &body(&null),
    );
    assert_eq!(status, 401, "{answer}");
    // A person's chair reports with its session.
    let (status, answer) = http(port, "POST", &path, Some(&host_seat), &body(&null));
    assert_eq!(status, 403, "{answer}");
    // About this game only, and with the gateway's record.
    let (status, answer) = http(
        port,
        "POST",
        &path,
        Some(&bridge),
        &body(&serde_json::json!(elsewhere)),
    );
    assert_eq!(status, 400, "{answer}");
    let mut local: serde_json::Value = serde_json::from_str(&body(&null)).unwrap();
    local["local_record"] = serde_json::json!({ "complete": true, "gzip_base64": "H4sIAAAAAAAA" });
    let (status, answer) = http(port, "POST", &path, Some(&bridge), &local.to_string());
    assert_eq!(status, 400, "{answer}");
    assert!(inbox.lock().is_empty(), "nothing refused was sent");

    // A report the service did not take was not sent: the chair may send
    // it again.
    down.store(true, Ordering::SeqCst);
    let (status, answer) = http(port, "POST", &path, Some(&bridge), &body(&null));
    assert_eq!(status, 502, "{answer}");
    down.store(false, Ordering::SeqCst);

    // Taken: the game named by the token, whatever the body left out.
    let (status, answer) = http(port, "POST", &path, Some(&bridge), &body(&null));
    assert_eq!(status, 201, "{answer}");
    assert_eq!(json_field(&answer, "report_id"), "r0");
    // Once a chair a game, its own game's id or not.
    let (status, answer) = http(
        port,
        "POST",
        &path,
        Some(&bridge),
        &body(&serde_json::json!(game)),
    );
    assert_eq!(status, 409, "{answer}");

    // The host's own report, for its pseudonym.
    let (status, answer) = http(port, "POST", "/reports", Some(&host), &body(&null));
    assert_eq!(status, 201, "{answer}");

    let inbox = inbox.lock();
    assert_eq!(inbox.len(), 2);
    let sent = &inbox[0];
    assert_eq!(sent["game_id"], game.as_str());
    assert_eq!(sent["client"]["source"], "llm-seat");
    assert_eq!(
        sent["reporter"], inbox[1]["reporter"],
        "the host's pseudonym: the host answers for the chair"
    );
    assert_eq!(sent["record_origin"], "gateway");
    assert_eq!(sent["record"]["complete"], true);
    let mut rest = sent.clone();
    rest["record"] = serde_json::Value::Null;
    let raw = rest.to_string();
    for private in ["hana", "HanaHost", bridge.as_str(), host.as_str()] {
        assert!(!raw.contains(private), "the report carried {private:?}");
    }
    // The record is exactly this game's, byte for byte as the store holds
    // it (Postgres wraps its base64 in lines).
    let attached = sent["record"]["gzip_base64"].as_str().expect("the record");
    let held = gw.text(&format!(
        "SELECT replace(encode(string_agg(data, ''::bytea ORDER BY seq), 'base64'), E'\\n', '') \
         FROM game_record_chunk WHERE game_id = '{game}'"
    ));
    assert_eq!(attached, held);
    let gz = base64::engine::general_purpose::STANDARD
        .decode(attached)
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
    drop(inbox);
    agent.abort();
}

/// Where the gateway files its reports with Central
/// (`BAYLEE_REPORTS_TO=central`), a bridge's report goes there too: under
/// the host's pseudonym, about its own game, with that game's record and
/// the bridge's `client` (its `llm-seat` source with it) as the parts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hosts_bridge_report_goes_to_central_where_the_operator_says_so() {
    let link = baylee_account::Token::generate().expect("a token");
    let (url, filed) = stub_central(link.bearer().to_string()).await;
    let file = central_file(&url, &link);
    let gw = spawn_gateway_with(
        "chair_report_central",
        &[
            ("BAYLEE_REPORTS_TO", "central".to_owned()),
            ("BAYLEE_FEEDBACK_KEY", "pseudonym-key".to_owned()),
            (
                "BAYLEE_CENTRAL_CONFIG_FILE",
                file.0.to_string_lossy().into_owned(),
            ),
        ],
    );
    let port = gw.port;
    let agent = attach_agent(&gw).await;
    let host = login(port, "hana", "HanaHost");
    let (game, bridge, _) = a_finished_bridge_game(&gw, &host).await;
    let null = serde_json::Value::Null;
    let path = format!("/lobby/games/{game}/chair/report");
    let (status, answer) = http(port, "POST", &path, Some(&bridge), &body(&null));
    assert_eq!(status, 201, "{answer}");
    // The host's own report, for its pseudonym.
    let (status, answer) = http(port, "POST", "/reports", Some(&host), &body(&null));
    assert_eq!(status, 201, "{answer}");
    parts_arrive(&filed, 3, 20).await;

    let desk = filed.lock();
    assert_eq!(desk.reports.len(), 2);
    let sent = &desk.reports[0];
    assert_eq!(sent["game_id"], game.as_str());
    assert_eq!(
        sent["reporter"], desk.reports[1]["reporter"],
        "the host's pseudonym: the host answers for the chair"
    );
    let raw = sent.to_string();
    for private in ["hana", "HanaHost", bridge.as_str(), host.as_str()] {
        assert!(!raw.contains(private), "the report carried {private:?}");
    }
    let first = format!("{:032x}", 1);
    let part = |name: &str| {
        desk.parts
            .iter()
            .find(|(id, p, _)| *id == first && p == name)
            .map_or_else(|| panic!("no {name} part"), |(_, _, gzip)| gzip.clone())
    };
    let mut client = String::new();
    flate2::read::GzDecoder::new(&part("client")[..])
        .read_to_string(&mut client)
        .expect("gzip");
    let client: serde_json::Value = serde_json::from_str(&client).unwrap();
    assert_eq!(client["source"], "llm-seat");
    let record = part("record");
    assert_eq!(
        i64::try_from(record.len()).unwrap(),
        gw.scalar(&format!(
            "SELECT bytes FROM game_record WHERE game_id = '{game}'"
        )),
        "the chair's game's record, as the store holds it"
    );
    drop(desk);
    agent.abort();
}

/// The host's reports an hour (`report::REPORTS_PER_WINDOW`).
const HOST_REPORTS_AN_HOUR: usize = 64;

/// A chair's report refused for its host's budget (`429`: the host's
/// account sent its reports for the hour) was not sent, so the chair may
/// send it again: it is refused for the budget again, never as one the
/// chair already sent (`409`), and nothing of it reached the service.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_chair_report_refused_for_its_host_s_budget_may_be_sent_again() {
    let (url, inbox, _down) = stub_service().await;
    let gw = spawn_gateway_with(
        "chair_report_budget",
        &[
            ("BAYLEE_FEEDBACK_URL", url),
            (
                "BAYLEE_FEEDBACK_TOKEN",
                "intake-secret-for-tests".to_owned(),
            ),
            ("BAYLEE_FEEDBACK_KEY", "pseudonym-key".to_owned()),
        ],
    );
    let port = gw.port;
    let agent = attach_agent(&gw).await;
    let host = login(port, "hana", "HanaHost");
    let (game, bridge, _) = a_finished_bridge_game(&gw, &host).await;
    let null = serde_json::Value::Null;
    for n in 0..HOST_REPORTS_AN_HOUR {
        let (status, answer) = http(port, "POST", "/reports", Some(&host), &body(&null));
        assert_eq!(status, 201, "the host's report {n}: {answer}");
    }
    let path = format!("/lobby/games/{game}/chair/report");
    for attempt in 0..2 {
        let (status, answer) = http(port, "POST", &path, Some(&bridge), &body(&null));
        assert_eq!(status, 429, "attempt {attempt}: {answer}");
    }
    assert_eq!(
        inbox.lock().len(),
        HOST_REPORTS_AN_HOUR,
        "the chair's report never reached the service"
    );
    agent.abort();
}
