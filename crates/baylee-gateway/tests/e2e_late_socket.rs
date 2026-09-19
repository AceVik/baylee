//! Regression test for the seam between "the engine is attached" and "a seat
//! is listening": the two happen in either order, and the gateway has to work
//! both ways round.
//!
//! A one-tap game against the house AI orders its engine *before* it answers
//! the request, so the player learns their seat token only after the order is
//! placed. With a warm engine binary that engine attaches in single-digit
//! milliseconds — comfortably before the client has finished parsing the
//! answer and dialled its own socket. The readiness flag is a
//! `tokio::sync::watch`, and a `watch::Sender` with no receivers refuses
//! `send` *and keeps the old value*, so the flag stayed `false`: the socket
//! then waited out the full 30-second timeout for an engine that had already
//! arrived, and the player saw an empty table.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::v1::{self, Envelope};
use common::{attach_agent_watching, http, json_field, login, spawn_gateway};
use futures_util::StreamExt;
use prost::Message;

#[tokio::test]
async fn a_seat_socket_that_arrives_after_its_engine_still_gets_the_game() {
    let gw = spawn_gateway("late-socket");
    let port = gw.port;
    let (_agent, mut engines) = attach_agent_watching(&gw).await;

    let token = login(port, "latecomer@example.com", "latecomer");
    let (status, body) = http(
        port,
        "POST",
        "/decks",
        Some(&token),
        "{\"name\":\"d\",\"cards\":[\"40 Forest\",\"20 Swamp\"]}",
    );
    assert_eq!(status, 200, "create deck: {body}");
    let deck_id = json_field(&body, "deck_id").to_string();

    let create = format!("{{\"deck_id\":\"{deck_id}\",\"mode\":\"ai\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&token), &create);
    assert_eq!(status, 200, "one-tap game against the house: {body}");
    let game_id = json_field(&body, "game_id").to_string();
    let seat_token = json_field(&body, "seat_token").to_string();

    // The whole point: the engine attaches *first*, so this socket is the late
    // one. A gateway that only notices readiness as it happens has nothing
    // left to tell this socket.
    //
    // Waiting for the engine rather than for three seconds is what makes that
    // true by construction instead of by hope. The agent sends a preset when
    // the gateway sends it `GameSetup`, and the gateway sends `GameSetup` only
    // once the engine has dialled in and said hello — so a preset in hand *is*
    // an attached engine. Three seconds was the same claim on an idle box, and
    // on a busy one it bought an engine that had not arrived yet: the socket
    // below was then the early one, the seam this file is named for went
    // untested, and the test passed.
    tokio::time::timeout(common::WAIT_BUDGET, engines.recv())
        .await
        .expect("the agent was never asked to run this game")
        .expect("the agent hung up before its engine attached");

    let url = format!("ws://127.0.0.1:{port}/games/{game_id}/ws?token={seat_token}");
    let mut ws = common::dial_seat(&url).await;

    // A working gateway answers at once. The broken one accepted the socket,
    // said nothing, and closed it half a minute later — so a generous
    // timeout still fails fast enough to be worth running.
    let frame = tokio::time::timeout(common::WAIT_BUDGET, ws.next())
        .await
        .expect("the table said nothing to a socket that arrived after its engine")
        .expect("the socket was closed instead of played on")
        .expect("frame ok");
    let env = Envelope::decode(frame.into_data()).expect("decode envelope");
    let Some(v1::envelope::Msg::GameStatic(msg)) = env.msg else {
        panic!("a seat's first frame is the opening payload");
    };
    assert_eq!(msg.view_version, baylee_view::VIEW_VERSION);
    let statics: baylee_view::GameStatic =
        serde_json::from_slice(&msg.static_json).expect("static json");
    assert_eq!(statics.your_seat, baylee_core::ids::PlayerId::new(0));
}
