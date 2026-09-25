//! The gateway holds every peer to the protocol it speaks itself (#271): an
//! agent, an engine or a seat socket of another protocol is refused, and told
//! both numbers.
//!
//! Nothing compares a client to an engine directly. Each is compared to the
//! gateway, and a peer that got past it speaks the gateway's protocol, so two
//! that did speak each other's.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::v1::{self, Envelope};
use baylee_protocol::{PROTOCOL_VERSION, version_refusal};
use common::{Gateway, Socket, http, json_field, login, next_msg, send, spawn_gateway};

/// Another protocol than this build's.
const OTHER: u32 = PROTOCOL_VERSION + 1;

fn agent_hello(gw: &Gateway, protocol_version: u32) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::AgentHello(v1::AgentHello {
            token: gw.agent_token.clone(),
            name: "test-agent".to_string(),
            capacity: 0,
            protocol_version,
        })),
    }
}

/// The next message, which must come within the budget.
async fn next(ws: &mut Socket, what: &str) -> Option<v1::envelope::Msg> {
    tokio::time::timeout(common::WAIT_BUDGET, next_msg(ws))
        .await
        .unwrap_or_else(|_| panic!("waited {:?} for {what}", common::WAIT_BUDGET))
}

/// An account with a deck, and a game against the house ordered with it:
/// the account's token and the order's answer.
fn order_a_game(port: u16, name: &str) -> (String, u16, String) {
    let token = login(port, name, name);
    let (status, body) = http(
        port,
        "POST",
        "/decks",
        Some(&token),
        "{\"name\":\"d\",\"cards\":[\"40 Forest\",\"20 Swamp\"]}",
    );
    assert_eq!(status, 200, "create deck: {body}");
    let create = format!(
        "{{\"deck_id\":\"{}\",\"mode\":\"ai\"}}",
        json_field(&body, "deck_id")
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(&token), &create);
    (token, status, body)
}

#[tokio::test]
async fn an_agent_of_another_protocol_is_told_both_numbers_and_not_registered() {
    let gw = spawn_gateway("protocol-agent");
    let url = format!("ws://127.0.0.1:{}/agent/ws", gw.port);
    let mut ws = common::dial(&url).await.expect("agent socket");
    send(&mut ws, &agent_hello(&gw, OTHER)).await;
    match next(&mut ws, "the refusal").await {
        Some(v1::envelope::Msg::Error(error)) => assert_eq!(
            Some(error.message),
            version_refusal("This agent", OTHER),
            "the agent is told both numbers"
        ),
        other => panic!("an agent of another protocol was not refused: {other:?}"),
    }
    assert_eq!(next(&mut ws, "the close").await, None, "and let go");
    let (_, status, body) = order_a_game(gw.port, "orderer");
    assert_eq!(status, 503, "no agent is registered to run it: {body}");
}

#[tokio::test]
async fn an_engine_of_another_protocol_ends_its_game() {
    let gw = spawn_gateway("protocol-engine");
    let url = format!("ws://127.0.0.1:{}/agent/ws", gw.port);
    let mut agent = common::dial(&url).await.expect("agent socket");
    send(&mut agent, &agent_hello(&gw, PROTOCOL_VERSION)).await;
    assert!(
        matches!(
            next(&mut agent, "the welcome").await,
            Some(v1::envelope::Msg::AgentWelcome(_))
        ),
        "an agent of this protocol is welcome"
    );
    let (token, status, body) = order_a_game(gw.port, "player");
    assert_eq!(status, 200, "a game against the house: {body}");
    let game_id = json_field(&body, "game_id").to_string();
    let Some(v1::envelope::Msg::StartEngine(start)) = next(&mut agent, "the order").await else {
        panic!("the agent was not asked to run the game");
    };
    assert_eq!(start.game_id, game_id);

    let mut engine = common::dial(&start.gateway_url)
        .await
        .expect("engine socket");
    let hello = Envelope {
        msg: Some(v1::envelope::Msg::EngineHello(v1::EngineHello {
            game_id: game_id.clone(),
            token: start.engine_token.clone(),
            protocol_version: OTHER,
        })),
    };
    send(&mut engine, &hello).await;
    match next(&mut engine, "the refusal").await {
        Some(v1::envelope::Msg::Error(error)) => assert_eq!(
            Some(error.message),
            version_refusal("This engine", OTHER),
            "the engine is told both numbers"
        ),
        other => panic!("an engine of another protocol was not refused: {other:?}"),
    }
    assert_eq!(next(&mut engine, "the close").await, None, "and let go");
    // The agent would start the same binary again, so the game does not
    // wait for another: its engine is released and it leaves the listing.
    match next(&mut agent, "the stop").await {
        Some(v1::envelope::Msg::StopEngine(stop)) => assert_eq!(stop.game_id, game_id),
        other => panic!("the agent was not told to stop the engine: {other:?}"),
    }
    let (status, body) = http(gw.port, "GET", "/lobby/games", Some(&token), "");
    assert_eq!(status, 200, "{body}");
    assert!(!body.contains(&game_id), "the game is still listed: {body}");
}

#[tokio::test]
async fn a_seat_socket_of_another_protocol_is_told_both_numbers_and_nothing_else() {
    let gw = spawn_gateway("protocol-seat");
    let port = gw.port;
    let _agent = common::attach_agent(&gw).await;
    let (_, status, body) = order_a_game(port, "seated");
    assert_eq!(status, 200, "a game against the house: {body}");
    let game_id = json_field(&body, "game_id").to_string();
    let seat_token = json_field(&body, "seat_token").to_string();

    // By hand, because the shared path can only say this build's protocol.
    let other =
        format!("ws://127.0.0.1:{port}/games/{game_id}/ws?token={seat_token}&protocol={OTHER}");
    let silent = format!("ws://127.0.0.1:{port}/games/{game_id}/ws?token={seat_token}");
    for (url, theirs) in [(other, OTHER), (silent, 0)] {
        let mut ws = common::dial(&url)
            .await
            .expect("the upgrade is not the check");
        match next(&mut ws, "the refusal").await {
            Some(v1::envelope::Msg::HelloAck(ack)) => {
                assert!(!ack.compatible, "{ack:?}");
                assert_eq!(ack.protocol_version, PROTOCOL_VERSION, "the gateway's own");
                assert_eq!(
                    Some(ack.message),
                    version_refusal("This client", theirs),
                    "the client is told both numbers"
                );
            }
            other => panic!(
                "a seat socket speaking {theirs} was not refused: {:.160}",
                format!("{other:?}")
            ),
        }
        assert_eq!(next(&mut ws, "the close").await, None, "and nothing else");
    }
    // The seat is still there for a client that speaks the protocol.
    let mut ws = common::dial_seat(port, &game_id, &seat_token).await;
    assert!(
        matches!(
            next(&mut ws, "the table").await,
            Some(v1::envelope::Msg::GameStatic(_))
        ),
        "the seat opens on its table"
    );
}
