//! A player whose connection is gone in a running game (`docs/protocol.md`
//! §"Leaving, and losing the connection"), over the real gateway with an
//! engine that runs its clocks.
//!
//! - Another player at the table: the table is told, waits the reconnect
//!   window, then the house plays the chair until the player is back.
//! - Nobody else at the table: the game pauses — no clock, no house move —
//!   until the player is back.
//! - A player who quits on purpose (`POST /lobby/depart`) with nobody else
//!   at the table ends the game.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::v1;
use common::{Socket, attach_agent_clocked, http, json_field, login, spawn_gateway};
use futures_util::StreamExt as _;
use prost::Message as _;
use std::time::Duration;

/// The shortest reconnect window a room may name (`clock::MIN_RECONNECT_SECS`).
const WINDOW: u64 = 10;

fn make_deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// Every log line that arrives on `ws` within `secs`, as JSON text, until
/// one contains `want`. Panics with what did arrive if none does.
async fn log_until(ws: &mut Socket, secs: u64, want: &str) -> String {
    let mut seen = String::new();
    let end = tokio::time::Instant::now() + Duration::from_secs(secs);
    loop {
        let frame = tokio::time::timeout_at(end, ws.next()).await;
        let Ok(Some(Ok(frame))) = frame else {
            panic!("{want} never arrived; the log said: {seen}");
        };
        if !frame.is_binary() {
            continue;
        }
        let Ok(envelope) = v1::Envelope::decode(frame.into_data()) else {
            continue;
        };
        if let Some(v1::envelope::Msg::StateDelta(delta)) = envelope.msg
            && !delta.log_json.is_empty()
        {
            seen.push_str(&String::from_utf8_lossy(&delta.log_json));
            seen.push('\n');
            if seen.contains(want) {
                return seen;
            }
        }
    }
}

/// Reads `ws` until the table opens (`Curtain`): before it a socket that
/// goes is a loader that went, not a player who lost the connection.
async fn until_open(ws: &mut Socket) {
    let open = tokio::time::timeout(Duration::from_secs(20), async {
        while let Some(msg) = common::next_msg(ws).await {
            if matches!(msg, v1::envelope::Msg::Curtain(_)) {
                return true;
            }
        }
        false
    })
    .await;
    assert_eq!(open, Ok(true), "the table never opened");
}

/// Nothing on `ws` within `secs` mentions `unwanted`.
async fn log_quiet(ws: &mut Socket, secs: u64, unwanted: &str) {
    let end = tokio::time::Instant::now() + Duration::from_secs(secs);
    while let Ok(Some(Ok(frame))) = tokio::time::timeout_at(end, ws.next()).await {
        if let Ok(envelope) = v1::Envelope::decode(frame.into_data())
            && let Some(v1::envelope::Msg::StateDelta(delta)) = envelope.msg
        {
            let text = String::from_utf8_lossy(&delta.log_json);
            assert!(!text.contains(unwanted), "{unwanted} arrived: {text}");
        }
    }
}

/// The chair `session` sits in at `game_id`, asked for again as a
/// restarted client asks (`POST …/seat`): a fresh seat token.
fn chair_again(port: u16, session: &str, game_id: &str) -> String {
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/seat"),
        Some(session),
        "",
    );
    assert_eq!(status, 200, "the chair again: {body}");
    json_field(&body, "seat_token").to_string()
}

/// Whether `game_id` is still listed (a finished game is not).
fn listed(port: u16, reader: &str, game_id: &str) -> bool {
    let (status, body) = http(port, "GET", "/lobby/games", Some(reader), "");
    assert_eq!(status, 200, "{body}");
    body.contains(game_id)
}

/// (c) Another player at the table: told at once with the window, then the
/// house takes the chair, and gives it back when the player returns.
#[tokio::test]
async fn with_another_player_the_table_waits_then_the_house_holds_the_chair() {
    let gw = spawn_gateway("lost-two");
    let port = gw.port;
    let _agent = attach_agent_clocked(&gw).await;
    let alice = login(port, "lost-alice", "Alice");
    let bob = login(port, "lost-bob", "Bob");
    let create = format!(
        "{{\"deck_id\":\"{}\",\"mode\":\"open\",\"reconnect_window_secs\":{WINDOW}}}",
        make_deck(port, &alice)
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(&alice), &create);
    assert_eq!(status, 200, "{body}");
    let game_id = json_field(&body, "game_id").to_string();
    let alice_token = json_field(&body, "seat_token").to_string();
    let join = format!("{{\"deck_id\":\"{}\"}}", make_deck(port, &bob));
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/join"),
        Some(&bob),
        &join,
    );
    assert_eq!(status, 200, "{body}");
    let bob_token = json_field(&body, "seat_token").to_string();
    for token in [&alice, &bob] {
        let (status, body) = http(
            port,
            "POST",
            &format!("/lobby/games/{game_id}/ready"),
            Some(token),
            "{}",
        );
        assert_eq!(status, 200, "{body}");
    }
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/start"),
        Some(&alice),
        "",
    );
    assert_eq!(status, 200, "{body}");

    let mut alice_ws = common::dial_seat(port, &game_id, &alice_token).await;
    let mut bob_ws = common::dial_seat(port, &game_id, &bob_token).await;
    until_open(&mut alice_ws).await;
    until_open(&mut bob_ws).await;

    drop(alice_ws);
    let told = log_until(&mut bob_ws, 10, "ConnectionLost").await;
    assert!(
        told.contains(&format!(r#""wait_secs":{WINDOW}"#)),
        "the line names the window: {told}"
    );
    // The window runs out: the house sits down for Alice.
    log_until(&mut bob_ws, WINDOW + 10, r#"{"StandIn":{"player":0}}"#).await;

    // Alice comes back and the chair is hers again.
    let token = chair_again(port, &alice, &game_id);
    let _alice_ws = common::dial_seat(port, &game_id, &token).await;
    log_until(&mut bob_ws, 10, r#"{"Returned":{"player":0}}"#).await;
}

/// (d) Nobody else at the table: the game pauses — past the window, no
/// house move — and goes on when the player is back.
#[tokio::test]
async fn alone_with_the_house_the_game_pauses_until_the_player_is_back() {
    let gw = spawn_gateway("lost-alone");
    let port = gw.port;
    let _agent = attach_agent_clocked(&gw).await;
    let player = login(port, "lost-solo", "Solo");
    let create = format!(
        "{{\"deck_id\":\"{}\",\"mode\":\"ai\",\"reconnect_window_secs\":{WINDOW}}}",
        make_deck(port, &player)
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(&player), &create);
    assert_eq!(status, 200, "{body}");
    let game_id = json_field(&body, "game_id").to_string();
    let token = json_field(&body, "seat_token").to_string();
    let mut ws = common::dial_seat(port, &game_id, &token).await;
    until_open(&mut ws).await;
    drop(ws);

    // Past the window, with time to spare: no stand-in, the game still on.
    tokio::time::sleep(Duration::from_secs(WINDOW + 3)).await;
    assert!(listed(port, &player, &game_id), "the paused game ended");

    let token = chair_again(port, &player, &game_id);
    let mut ws = common::dial_seat(port, &game_id, &token).await;
    let said = log_until(&mut ws, 10, r#"{"Returned":{"player":0}}"#).await;
    assert!(
        said.contains(r#"{"ConnectionLost":{"player":0,"wait_secs":null}}"#),
        "the log says the game was paused: {said}"
    );
    assert!(
        !said.contains(r#"{"StandIn""#),
        "the house sat down at a paused table: {said}"
    );
    log_quiet(&mut ws, 1, r#"{"StandIn""#).await;
}

/// (e) Quitting on purpose with nobody else at the table ends the game at
/// once.
#[tokio::test]
async fn quitting_on_purpose_alone_ends_the_game() {
    let gw = spawn_gateway("lost-quit");
    let port = gw.port;
    let _agent = attach_agent_clocked(&gw).await;
    let player = login(port, "quit-solo", "Quitter");
    let create = format!(
        "{{\"deck_id\":\"{}\",\"mode\":\"ai\"}}",
        make_deck(port, &player)
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(&player), &create);
    assert_eq!(status, 200, "{body}");
    let game_id = json_field(&body, "game_id").to_string();
    let token = json_field(&body, "seat_token").to_string();
    let mut ws = common::dial_seat(port, &game_id, &token).await;
    until_open(&mut ws).await;

    let (status, body) = http(port, "POST", "/lobby/depart", Some(&player), "");
    assert_eq!(status, 204, "{body}");
    for _ in 0..100 {
        if !listed(port, &player, &game_id) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the game is still listed");
}
