//! End-to-end regression test for human-vs-human over the gateway:
//! spawns the real `baylee-gateway` binary, registers two accounts over
//! HTTP, creates and joins a lobby game, then connects BOTH seat sockets
//! and asserts that player B sees the game advance when player A acts.
//!
//! Before the per-game broadcast this failed: the gateway filtered every
//! pumped envelope to the acting seat, so B's socket stayed silent until
//! B acted — which B couldn't do, not knowing the game state.

#![allow(clippy::missing_docs_in_private_items)]

use baylee_engine::choice::{Pending, PlayerAction};
use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use std::io::{Read, Write};

/// Minimal blocking HTTP/1.1 client (the gateway is a separate process;
/// nothing else needs this test's runtime thread).
fn http_post(port: u16, path: &str, token: Option<&str>, body: &str) -> (u16, String) {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect http");
    let auth = token.map_or(String::new(), |t| format!("Authorization: Bearer {t}\r\n"));
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).expect("write http");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read http");
    let status: u16 = raw
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("http status");
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

fn json_field<'a>(body: &'a str, field: &str) -> &'a str {
    let marker = format!("\"{field}\":\"");
    let start = body.find(&marker).expect("field present") + marker.len();
    let rest = &body[start..];
    let end = rest.find('"').expect("field ends");
    &rest[..end]
}

async fn recv_until<F>(
    ws: &mut (
             impl StreamExt<
        Item = Result<
            tokio_tungstenite::tungstenite::Message,
            tokio_tungstenite::tungstenite::Error,
        >,
    > + Unpin
         ),
    mut accept: F,
) -> Envelope
where
    F: FnMut(&Envelope) -> bool,
{
    for _ in 0..50 {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(10), ws.next())
            .await
            .expect("no frame within 10s")
            .expect("stream open")
            .expect("frame ok");
        if !frame.is_binary() {
            continue;
        }
        let env = Envelope::decode(frame.into_data()).expect("decode envelope");
        if accept(&env) {
            return env;
        }
    }
    panic!("expected envelope never arrived");
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn human_vs_human_both_seats_receive_updates() {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let store_path =
        std::env::temp_dir().join(format!("baylee-gateway-test-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&store_path);

    let mut server = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        .env("PORT", port.to_string())
        .env("STORE_PATH", &store_path)
        .stdout(std::process::Stdio::null())
        .stderr(if std::env::var("HVH_DEBUG").is_ok() {
            std::process::Stdio::inherit()
        } else {
            std::process::Stdio::null()
        })
        .spawn()
        .expect("spawn gateway");

    // Wait for the port to accept.
    for _ in 0..50 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // Two accounts. Registration answers `{"ok":true}` without a token,
    // so log in afterwards.
    let mut tokens = Vec::new();
    for (email, name) in [
        ("alice@example.com", "alice_hvh"),
        ("bob@example.com", "bob_hvh"),
    ] {
        let register = format!(
            "{{\"email\":\"{email}\",\"display_name\":\"{name}\",\"password\":\"a-very-fine-password\"}}"
        );
        let (status, _) = http_post(port, "/auth/register", None, &register);
        assert_eq!(status, 200, "register {email}");
        let login = format!("{{\"email\":\"{email}\",\"password\":\"a-very-fine-password\"}}");
        let (status, body) = http_post(port, "/auth/login", None, &login);
        assert_eq!(status, 200, "login {email}");
        tokens.push(json_field(&body, "token").to_string());
    }

    // One deck each (basic lands pass the registry and the count rules).
    let mut deck_ids = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        // Deliberately not the same card pool: the opening payload must have
        // a hole where the other deck's exclusive printing is.
        let cards = if i == 0 {
            "\"40 Island\",\"20 Forest\""
        } else {
            "\"40 Forest\",\"20 Swamp\""
        };
        let deck = format!("{{\"name\":\"d{i}\",\"cards\":[{cards}]}}");
        let (status, body) = http_post(port, "/decks", Some(token), &deck);
        assert_eq!(status, 200, "create deck {i}: {body}");
        deck_ids.push(json_field(&body, "deck_id").to_string());
    }

    // A opens a waiting game, B joins it.
    let create = format!("{{\"deck_id\":\"{}\",\"mode\":\"open\"}}", deck_ids[0]);
    let (status, body) = http_post(port, "/lobby/games", Some(&tokens[0]), &create);
    assert_eq!(status, 200, "create open game: {body}");
    let game_id = json_field(&body, "game_id").to_string();
    let seat_token_a = json_field(&body, "seat_token").to_string();

    let join = format!("{{\"deck_id\":\"{}\"}}", deck_ids[1]);
    let (status, body) = http_post(
        port,
        &format!("/lobby/games/{game_id}/join"),
        Some(&tokens[1]),
        &join,
    );
    assert_eq!(status, 200, "join game: {body}");
    let seat_token_b = json_field(&body, "seat_token").to_string();

    // Connect both seat sockets.
    let url = |token: &str| format!("ws://127.0.0.1:{port}/games/{game_id}/ws?token={token}");
    let mut ws_a = None;
    let mut ws_b = None;
    for _ in 0..50 {
        if ws_a.is_none()
            && let Ok((stream, _)) = tokio_tungstenite::connect_async(url(&seat_token_a)).await
        {
            ws_a = Some(stream);
        }
        if ws_b.is_none()
            && let Ok((stream, _)) = tokio_tungstenite::connect_async(url(&seat_token_b)).await
        {
            ws_b = Some(stream);
        }
        if ws_a.is_some() && ws_b.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let mut ws_a = ws_a.expect("seat A socket");
    let mut ws_b = ws_b.expect("seat B socket");

    // The very first thing a seat is sent is the roster and the print table.
    // A client has no preset to build them from, and without the print table
    // a PrintRef names no card at all — so it has to arrive before anything
    // that refers to one.
    let opening = recv_until(&mut ws_b, |_| true).await;
    let Some(v1::envelope::Msg::GameStatic(msg)) = opening.msg else {
        panic!("a seat's first frame is the opening payload");
    };
    assert_eq!(msg.view_version, baylee_view::VIEW_VERSION);
    let statics: baylee_view::GameStatic =
        serde_json::from_slice(&msg.static_json).expect("static json");
    assert_eq!(statics.your_seat, baylee_core::ids::PlayerId::new(1));
    assert_eq!(
        statics.seat_name(baylee_core::ids::PlayerId::new(0)),
        "alice_hvh"
    );
    assert_eq!(
        statics.seat_name(baylee_core::ids::PlayerId::new(1)),
        "bob_hvh"
    );
    assert!(
        statics.prints.iter().any(Option::is_some),
        "the print table came along"
    );
    assert!(
        statics.prints.iter().any(Option::is_none),
        "and stopped short of the other deck: {:?}",
        statics.prints
    );
    assert!(
        statics.seats.iter().all(|s| !s.is_ai),
        "both chairs are people in a human-vs-human game"
    );

    // Seat A gets the first choice request (its mulligan).
    let first = recv_until(&mut ws_a, |env| {
        matches!(env.msg, Some(v1::envelope::Msg::ChoiceRequest(_)))
    })
    .await;
    let Some(v1::envelope::Msg::ChoiceRequest(req)) = first.msg else {
        panic!("checked above");
    };
    let pending: Pending = serde_json::from_slice(&req.pending_json).expect("pending json");
    let action = match pending {
        Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
        _ => PlayerAction::PassPriority,
    };

    // Drain whatever B got from the initial pumps (non-blocking).
    while let Ok(Some(_)) =
        tokio::time::timeout(std::time::Duration::from_millis(200), ws_b.next()).await
    {}

    // A acts. WITHOUT B sending anything, B's socket must now deliver a
    // fresh envelope — this is the regression assertion.
    let answer = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(&action).unwrap(),
        })),
    };
    ws_a.send(tokio_tungstenite::tungstenite::Message::Binary(
        answer.encode_to_vec().into(),
    ))
    .await
    .expect("A sends its action");

    recv_until(&mut ws_b, |env| {
        matches!(
            env.msg,
            Some(v1::envelope::Msg::StateDelta(_) | v1::envelope::Msg::ChoiceRequest(_))
        )
    })
    .await;

    let _ = server.kill();
    let _ = server.wait();
    let _ = std::fs::remove_file(&store_path);
}
