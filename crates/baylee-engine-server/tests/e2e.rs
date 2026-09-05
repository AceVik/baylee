//! End-to-end smoke test: the real server binary over a real socket.
//! Spawns `baylee-engine-server` on an ephemeral port, connects a
//! protobuf websocket client, creates the acceptance duel, answers a
//! mulligan, and verifies the game advances (a new choice arrives).

#![allow(clippy::missing_docs_in_private_items)]

use baylee_core::ids::PlayerId;
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

/// The AI chair is a chair: a socket can sit in it, is asked the questions
/// the house AI would have answered, and gives it back by hanging up.
///
/// This is the rules-side counterpart of the client's `dev-control` harness,
/// and it is written over two real sockets on purpose. A test that called
/// `Session::take_over` directly would pass while the harness still routed
/// every seat's envelopes to whichever socket happened to be holding the
/// lock — which is exactly what it did before this existed.
#[tokio::test]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn a_socket_can_take_an_ai_chair_and_hand_it_back() {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);

    let mut server = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-engine-server"))
        .env("PORT", port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn server");
    let url = format!("ws://127.0.0.1:{port}");
    let mut human = dial(&url).await;

    // Seat 0 creates the dev duel and is asked the first question.
    send(
        &mut human,
        &Envelope {
            msg: Some(v1::envelope::Msg::CreateGame(v1::CreateGame {
                preset: None,
            })),
        },
    )
    .await;
    let mut game_id = String::new();
    let mut asked_seat_0 = None;
    for _ in 0..10 {
        let Some(env) = next(&mut human).await else {
            break;
        };
        match env.msg {
            Some(v1::envelope::Msg::GameCreated(created)) => game_id = created.game_id,
            Some(v1::envelope::Msg::ChoiceRequest(req)) => {
                asked_seat_0 = serde_json::from_slice::<Pending>(&req.pending_json).ok();
                break;
            }
            _ => {}
        }
    }
    assert!(!game_id.is_empty(), "the game was created");
    let asked_seat_0 = asked_seat_0.expect("seat 0 was asked something");

    // A second socket takes seat 1 — the house AI's chair.
    let mut driver = dial(&url).await;
    send(
        &mut driver,
        &Envelope {
            msg: Some(v1::envelope::Msg::Join(v1::JoinGame {
                game_id: game_id.clone(),
                seat_token: "1".to_string(),
            })),
        },
    )
    .await;
    let mut driver_saw_view = false;
    let mut driver_asked = false;
    for _ in 0..10 {
        let Some(env) = next_within(&mut driver, 500).await else {
            break;
        };
        match env.msg {
            Some(v1::envelope::Msg::StateDelta(delta)) => {
                assert_eq!(
                    view_seat(&delta),
                    Some(PlayerId::new(1)),
                    "the driven socket is sent its own seat's view"
                );
                driver_saw_view = true;
            }
            Some(v1::envelope::Msg::ChoiceRequest(_)) => {
                driver_asked = true;
                break;
            }
            Some(v1::envelope::Msg::Error(err)) => panic!("seat 1 was refused: {}", err.message),
            _ => {}
        }
    }
    assert!(driver_saw_view, "seat 1 is sent its own view");

    // While this socket holds the chair, a second one may not have it. Two
    // programs on one chair would race each other for every question.
    let mut queue_jumper = dial(&url).await;
    send(
        &mut queue_jumper,
        &Envelope {
            msg: Some(v1::envelope::Msg::Join(v1::JoinGame {
                game_id: game_id.clone(),
                seat_token: "1".to_string(),
            })),
        },
    )
    .await;
    let mut second_refused = false;
    for _ in 0..10 {
        let Some(env) = next_within(&mut queue_jumper, 500).await else {
            break;
        };
        match env.msg {
            Some(v1::envelope::Msg::Error(_)) => {
                second_refused = true;
                break;
            }
            Some(v1::envelope::Msg::StateDelta(_) | v1::envelope::Msg::ChoiceRequest(_)) => {
                panic!("a second socket was served the chair someone is already driving")
            }
            _ => {}
        }
    }
    assert!(second_refused, "a chair already being driven is refused");
    drop(queue_jumper);

    // Seat 0 answers. The next question belongs to seat 1, and it must come
    // out of the socket rather than into `HeuristicAgent::act`.
    if !driver_asked {
        answer(&mut human, &asked_seat_0).await;
        for _ in 0..20 {
            let Some(env) = next(&mut driver).await else {
                break;
            };
            match env.msg {
                Some(v1::envelope::Msg::StateDelta(delta)) => assert_eq!(
                    view_seat(&delta),
                    Some(PlayerId::new(1)),
                    "the driven socket is sent its own seat's view"
                ),
                Some(v1::envelope::Msg::ChoiceRequest(_)) => {
                    driver_asked = true;
                    break;
                }
                _ => {}
            }
        }
    }
    assert!(
        driver_asked,
        "the taken-over seat is asked instead of answering itself"
    );

    // Nothing is left in flight for seat 0 before the chair goes back — and
    // nothing that arrived belonged to seat 1. This is the routing
    // assertion, and it is the reason the fan carries a seat number at all:
    // a fan that copied every frame to every socket would hand seat 0 the
    // driven seat's view of its own hand, and every take-over test above
    // would still pass.
    while let Some(env) = next_within(&mut human, 300).await {
        if let Some(v1::envelope::Msg::StateDelta(delta)) = env.msg {
            assert_eq!(
                view_seat(&delta),
                Some(PlayerId::new(0)),
                "seat 0 was sent another seat's view"
            );
        }
    }

    // Hanging up hands the chair back: the house AI answers the question this
    // socket was sitting on and plays seat 1 forward until seat 0 is needed.
    drop(driver);
    let mut human_asked_again = false;
    for _ in 0..20 {
        let Some(env) = next(&mut human).await else {
            break;
        };
        if matches!(env.msg, Some(v1::envelope::Msg::ChoiceRequest(_))) {
            human_asked_again = true;
            break;
        }
    }
    let _ = server.kill();
    let _ = server.wait();
    assert!(
        human_asked_again,
        "releasing the chair let the house AI play seat 1 on"
    );
}

/// Naming a seat that is not at the table is refused rather than served.
#[tokio::test]
async fn a_socket_cannot_sit_at_a_seat_that_is_not_there() {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);

    let mut server = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-engine-server"))
        .env("PORT", port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn server");
    let url = format!("ws://127.0.0.1:{port}");
    let mut human = dial(&url).await;
    send(
        &mut human,
        &Envelope {
            msg: Some(v1::envelope::Msg::CreateGame(v1::CreateGame {
                preset: None,
            })),
        },
    )
    .await;
    let mut game_id = String::new();
    for _ in 0..10 {
        let Some(env) = next(&mut human).await else {
            break;
        };
        if let Some(v1::envelope::Msg::GameCreated(created)) = env.msg {
            game_id = created.game_id;
            break;
        }
    }
    assert!(!game_id.is_empty(), "the game was created");

    let mut stranger = dial(&url).await;
    send(
        &mut stranger,
        &Envelope {
            msg: Some(v1::envelope::Msg::Join(v1::JoinGame {
                game_id,
                seat_token: "7".to_string(),
            })),
        },
    )
    .await;
    let mut refused = false;
    for _ in 0..10 {
        let Some(env) = next(&mut stranger).await else {
            break;
        };
        match env.msg {
            Some(v1::envelope::Msg::Error(_)) => {
                refused = true;
                break;
            }
            Some(v1::envelope::Msg::StateDelta(_) | v1::envelope::Msg::ChoiceRequest(_)) => {
                panic!("seat 7 was served a view at a two-seat table")
            }
            _ => {}
        }
    }
    let _ = server.kill();
    let _ = server.wait();
    assert!(refused, "a seat that is not at the table is refused");
}

type Client =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn dial(url: &str) -> Client {
    for _ in 0..50 {
        if let Ok((stream, _)) = tokio_tungstenite::connect_async(url).await {
            return stream;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("server accepts the websocket");
}

async fn send(ws: &mut Client, envelope: &Envelope) {
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        envelope.encode_to_vec().into(),
    ))
    .await
    .expect("send");
}

/// The next envelope, or `None` once the socket has gone quiet.
///
/// Quiet is an answer here, not a hang: several of the assertions above are
/// about a frame *arriving*, so the wait has to end on its own.
async fn next(ws: &mut Client) -> Option<Envelope> {
    next_within(ws, 5_000).await
}

/// The same, with the wait spelled out — short when the point is that the
/// socket has nothing left to say.
async fn next_within(ws: &mut Client, millis: u64) -> Option<Envelope> {
    loop {
        let frame = tokio::time::timeout(std::time::Duration::from_millis(millis), ws.next()).await;
        match frame {
            Ok(Some(Ok(frame))) if frame.is_binary() => {
                return Envelope::decode(frame.into_data()).ok();
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(_)) | None) | Err(_) => return None,
        }
    }
}

/// The seat a view was built for.
///
/// `PlayerView::seat` is what makes the routing checkable from outside the
/// process: a view says whose it is, so a socket receiving one that is not
/// its own is a leak and not merely noise.
fn view_seat(delta: &v1::StateDelta) -> Option<PlayerId> {
    let view: serde_json::Value = serde_json::from_slice(&delta.view_json).ok()?;
    serde_json::from_value(view.get("seat")?.clone()).ok()
}

/// Answers a pending with the most trivial legal reply it has.
async fn answer(ws: &mut Client, pending: &Pending) {
    let action = match pending {
        Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
        _ => PlayerAction::PassPriority,
    };
    send(
        ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                game_id: String::new(),
                seat_token: String::new(),
                action_json: serde_json::to_vec(&action).unwrap(),
            })),
        },
    )
    .await;
}
