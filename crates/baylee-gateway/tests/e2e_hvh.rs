//! End-to-end regression test for human-vs-human over the gateway: spawns
//! the real `baylee-gateway` binary, attaches an agent that runs a real
//! engine, registers two accounts over HTTP, creates and joins a lobby game,
//! then connects BOTH seat sockets and asserts that player B sees the game
//! advance when player A acts.
//!
//! Before the per-game broadcast this failed: the gateway filtered every
//! pumped envelope to the acting seat, so B's socket stayed silent until B
//! acted — which B couldn't do, not knowing the game state.
//!
//! It now also covers the whole circle. The gateway runs no rules: it asks an
//! agent for an engine, the engine dials back, and every frame in this test
//! crosses both sockets.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_engine::choice::{Pending, PlayerAction};
use baylee_protocol::v1::{self, Envelope};
use common::{attach_agent, http, json_field, login, spawn_gateway};
use futures_util::{SinkExt, StreamExt};
use prost::Message;

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
    let gw = spawn_gateway("hvh");
    let port = gw.port;
    let agent = attach_agent(&gw).await;

    // Two accounts.
    let tokens = [
        login(port, "alice@example.com", "alice_hvh"),
        login(port, "bob@example.com", "bob_hvh"),
    ];

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
        let (status, body) = http(port, "POST", "/decks", Some(token), &deck);
        assert_eq!(status, 200, "create deck {i}: {body}");
        deck_ids.push(json_field(&body, "deck_id").to_string());
    }

    // A opens a waiting game, B joins it.
    let create = format!("{{\"deck_id\":\"{}\",\"mode\":\"open\"}}", deck_ids[0]);
    let (status, body) = http(port, "POST", "/lobby/games", Some(&tokens[0]), &create);
    assert_eq!(status, 200, "create open game: {body}");
    let game_id = json_field(&body, "game_id").to_string();
    let seat_token_a = json_field(&body, "seat_token").to_string();

    let join = format!("{{\"deck_id\":\"{}\"}}", deck_ids[1]);
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/join"),
        Some(&tokens[1]),
        &join,
    );
    assert_eq!(status, 200, "join game: {body}");
    let seat_token_b = json_field(&body, "seat_token").to_string();

    // Both say they are ready, and the host starts the table. Sitting down is
    // not the same statement as being ready to play, so neither is enough on
    // its own.
    for token in &tokens {
        let (status, body) = http(
            port,
            "POST",
            &format!("/lobby/games/{game_id}/ready"),
            Some(token),
            "{}",
        );
        assert_eq!(status, 200, "ready: {body}");
    }
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/start"),
        Some(&tokens[0]),
        "",
    );
    assert_eq!(status, 200, "start: {body}");

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

    agent.abort();
}

/// The gateway runs no rules of its own, so with nobody to run an engine there
/// is no game to be had — and it has to say so rather than hand out a seat
/// token for a table that will never start.
#[tokio::test]
async fn a_game_without_an_agent_is_refused() {
    let gw = spawn_gateway("no-agent");
    let token = login(gw.port, "solo@example.com", "solo_player");
    let (status, body) = http(
        gw.port,
        "POST",
        "/decks",
        Some(&token),
        "{\"name\":\"d\",\"cards\":[\"60 Forest\"]}",
    );
    assert_eq!(status, 200, "create deck: {body}");
    let deck_id = json_field(&body, "deck_id").to_string();

    let create = format!("{{\"deck_id\":\"{deck_id}\",\"mode\":\"ai\"}}");
    let (status, body) = http(gw.port, "POST", "/lobby/games", Some(&token), &create);
    assert_eq!(
        status, 503,
        "a game started with no engine to run it: {body}"
    );

    // And the table did not survive the failure as a ghost in the lobby.
    let (status, body) = http(gw.port, "GET", "/lobby/games", Some(&token), "");
    assert_eq!(status, 200);
    assert!(
        body.contains("\"games\":[]"),
        "a failed game was left in the lobby: {body}"
    );
}

/// An agent is not a player. The control socket takes a shared secret from the
/// gateway's own configuration, and nothing a player could ever hold.
#[tokio::test]
async fn the_control_socket_refuses_the_wrong_secret() {
    let gw = spawn_gateway("agent-auth");
    let url = format!("ws://127.0.0.1:{}/agent/ws", gw.port);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("the upgrade itself is not the check");
    let hello = Envelope {
        msg: Some(v1::envelope::Msg::AgentHello(v1::AgentHello {
            token: "not-the-secret".to_string(),
            name: "impostor".to_string(),
            capacity: 0,
        })),
    };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        hello.encode_to_vec().into(),
    ))
    .await
    .expect("send hello");
    let next = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next()).await;
    let welcomed = matches!(
        next,
        Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Binary(_))))
    );
    assert!(!welcomed, "an agent with the wrong secret was welcomed");
}
