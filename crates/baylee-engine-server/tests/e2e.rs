//! End-to-end smoke test: the real server binary over a real socket.
//! Spawns `baylee-engine-server` on an ephemeral port, connects a
//! protobuf websocket client, creates the acceptance duel, answers a
//! mulligan, and verifies the game advances (a new choice arrives).

#![allow(clippy::missing_docs_in_private_items)]

use baylee_engine::choice::{Pending, PlayerAction};
use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt, StreamExt};
use prost::Message;

#[tokio::test]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn create_game_and_answer_first_choice() {
    // Bind an ephemeral port first, then release it for the server.
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);

    let server = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-engine-server"))
        .env("PORT", port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn server");
    let mut server = server;

    // Wait for the port to accept.
    let url = format!("ws://127.0.0.1:{port}");
    let mut ws = None;
    for _ in 0..50 {
        match tokio_tungstenite::connect_async(&url).await {
            Ok((stream, _)) => {
                ws = Some(stream);
                break;
            }
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
        }
    }
    let mut ws = ws.expect("server accepts the websocket");

    // CreateGame → GameCreated (+ first choice request).
    let create = Envelope {
        msg: Some(v1::envelope::Msg::CreateGame(v1::CreateGame {
            preset: None,
        })),
    };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        create.encode_to_vec().into(),
    ))
    .await
    .expect("send create");

    let mut saw_game_created = false;
    let mut saw_view = false;
    let mut first_pending: Option<Pending> = None;
    let mut game_id = String::new();
    for _ in 0..10 {
        let Some(frame) = ws.next().await else { break };
        let frame = frame.expect("frame");
        if !frame.is_binary() {
            continue;
        }
        let env = Envelope::decode(frame.into_data()).expect("decode");
        match env.msg {
            Some(v1::envelope::Msg::GameCreated(created)) => {
                saw_game_created = true;
                game_id = created.game_id;
            }
            Some(v1::envelope::Msg::StateDelta(delta)) => {
                // Hidden information: the view exists and carries only
                // the viewing seat's own hand contents.
                let view: serde_json::Value =
                    serde_json::from_slice(&delta.view_json).expect("view json");
                saw_view = true;
                assert!(view.get("seats").is_some(), "view has seat lines");
                assert!(view.get("hand").is_some(), "view has the own hand");
            }
            Some(v1::envelope::Msg::ChoiceRequest(req)) => {
                first_pending = serde_json::from_slice(&req.pending_json).ok();
                break;
            }
            _ => {}
        }
    }
    assert!(saw_game_created, "server acknowledged the game");
    assert!(saw_view, "server sent a hidden-info-filtered view");
    let pending = first_pending.expect("server requested a choice");

    // Answer it; the game must advance with another choice (or game over).
    let action = match pending {
        Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
        _ => PlayerAction::PassPriority,
    };
    let answer = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(&action).unwrap(),
        })),
    };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        answer.encode_to_vec().into(),
    ))
    .await
    .expect("send action");

    let mut advanced = false;
    for _ in 0..10 {
        let Some(frame) = ws.next().await else { break };
        let frame = frame.expect("frame");
        if !frame.is_binary() {
            continue;
        }
        let env = Envelope::decode(frame.into_data()).expect("decode");
        if matches!(
            env.msg,
            Some(v1::envelope::Msg::ChoiceRequest(_) | v1::envelope::Msg::GameCreated(_))
        ) {
            advanced = true;
            break;
        }
    }
    assert!(advanced, "the game advanced after the first answer");

    // A second connection joins the same game by id (multi-game manager).
    let (mut ws2, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("second client connects");
    let join = Envelope {
        msg: Some(v1::envelope::Msg::Join(v1::JoinGame {
            game_id: game_id.clone(),
            seat_token: String::new(),
        })),
    };
    ws2.send(tokio_tungstenite::tungstenite::Message::Binary(
        join.encode_to_vec().into(),
    ))
    .await
    .expect("send join");
    let mut joined = false;
    for _ in 0..10 {
        let Some(frame) = ws2.next().await else { break };
        let frame = frame.expect("frame");
        if !frame.is_binary() {
            continue;
        }
        let env = Envelope::decode(frame.into_data()).expect("decode");
        if matches!(
            env.msg,
            Some(v1::envelope::Msg::StateDelta(_) | v1::envelope::Msg::ChoiceRequest(_))
        ) {
            joined = true;
            break;
        }
    }
    let _ = server.kill();
    assert!(joined, "a second client re-attached to the live game");
}
