//! A seat socket has a frame allowance (#284): one that floods is closed
//! with 1008, which says why, rather than having its frames dropped.
//!
//! The allowance is set well above what a real client sends, and every
//! other test in this suite plays through it. So this one only floods: ten
//! thousand frames at once, far past any allowance the gateway could run.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::v1::{self, Envelope};
use common::{http, json_field, login, spawn_gateway};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

#[tokio::test]
async fn a_seat_that_floods_its_socket_is_closed_with_policy() {
    let gw = spawn_gateway("seat-rate");
    let port = gw.port;
    let _agent = common::attach_agent(&gw).await;
    let token = login(port, "flooder", "flooder");
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
    assert_eq!(status, 200, "a game against the house: {body}");
    let game_id = json_field(&body, "game_id");
    let seat_token = json_field(&body, "seat_token");

    let mut ws = common::dial_seat(port, game_id, seat_token).await;
    let beat = Envelope {
        msg: Some(v1::envelope::Msg::Heartbeat(v1::Heartbeat::default())),
    }
    .encode_to_vec();
    for _ in 0..10_000 {
        if ws.feed(Message::Binary(beat.clone().into())).await.is_err() {
            break;
        }
    }
    let _ = ws.flush().await;

    let closed = tokio::time::timeout(common::WAIT_BUDGET, async {
        while let Some(frame) = ws.next().await {
            match frame {
                Ok(Message::Close(frame)) => return frame,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        None
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "a flooding seat was still open after {:?}",
            common::WAIT_BUDGET
        )
    });
    let closed = closed.expect("the socket closed without saying why");
    assert_eq!(closed.code, CloseCode::Policy, "{closed:?}");
}
