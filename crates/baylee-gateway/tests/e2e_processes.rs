//! The whole circle with nothing faked: a real gateway, a real agent, and a
//! real engine process, with a player's socket at the end of it.
//!
//! `e2e_hvh` runs the same protocol with the engine in-process, which is what
//! keeps it fast and independent of what happens to be built. This one costs
//! the other half of that trade — it needs `baylee-agent` and
//! `baylee-engine-server` to exist beside the gateway binary — so it is
//! `#[ignore]`d and run on purpose:
//!
//! ```bash
//! cargo build --workspace --bins
//! cargo test -p baylee-gateway --test e2e_processes -- --ignored
//! ```

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::v1::{self, Envelope};
use common::{http, json_field, login, spawn_gateway};
use futures_util::StreamExt;
use prost::Message;

/// An agent process, torn down with the test. Killing it kills the engines it
/// started: they are spawned with `kill_on_drop`.
struct Agent(std::process::Child);

impl Drop for Agent {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A binary beside the gateway's, if it was built.
fn sibling(name: &str) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        .parent()?
        .join(name);
    path.exists().then_some(path)
}

#[tokio::test]
#[ignore = "needs `cargo build --workspace --bins`; see the module header"]
async fn a_real_agent_starts_a_real_engine_for_a_real_seat() {
    let (Some(agent_bin), Some(engine_bin)) =
        (sibling("baylee-agent"), sibling("baylee-engine-server"))
    else {
        panic!("build the workspace binaries first: cargo build --workspace --bins");
    };

    let gw = spawn_gateway("processes");
    let port = gw.port;
    let _agent = Agent(
        std::process::Command::new(&agent_bin)
            .env("BAYLEE_GATEWAY", format!("http://127.0.0.1:{port}"))
            .env("BAYLEE_AGENT_TOKEN", &gw.agent_token)
            .env("BAYLEE_AGENT_NAME", "e2e")
            .env("BAYLEE_ENGINE_BIN", &engine_bin)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn agent"),
    );

    let token = login(port, "processes@example.com", "process_player");
    let (status, body) = http(
        port,
        "POST",
        "/decks",
        Some(&token),
        "{\"name\":\"d\",\"cards\":[\"60 Forest\"]}",
    );
    assert_eq!(status, 200, "create deck: {body}");
    let deck_id = json_field(&body, "deck_id").to_string();

    // The agent needs a moment to register; until it has, there is nothing to
    // run a game and the gateway rightly says so.
    let create = format!("{{\"deck_id\":\"{deck_id}\",\"mode\":\"ai\"}}");
    let mut answer = None;
    for _ in 0..50 {
        let (status, body) = http(port, "POST", "/lobby/games", Some(&token), &create);
        if status == 200 {
            answer = Some(body);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    let answer = answer.expect("no agent ever registered");
    let game_id = json_field(&answer, "game_id").to_string();
    let seat_token = json_field(&answer, "seat_token").to_string();

    let url = format!("ws://127.0.0.1:{port}/games/{game_id}/ws?token={seat_token}");
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("seat socket");

    // The seat's first frame is the roster and the print table — and it came
    // out of a process the gateway started through an agent and cannot read.
    let first = next(&mut ws).await;
    assert!(
        matches!(first, Some(v1::envelope::Msg::GameStatic(_))),
        "expected the opening payload, got {first:?}"
    );
    // And then the game actually asks the player something.
    for _ in 0..20 {
        if let Some(v1::envelope::Msg::ChoiceRequest(_)) = next(&mut ws).await {
            return;
        }
    }
    panic!("the engine never asked the seat anything");
}

async fn next(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Option<v1::envelope::Msg> {
    loop {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(20), ws.next())
            .await
            .expect("no frame within 20s")?
            .expect("frame ok");
        if !frame.is_binary() {
            continue;
        }
        if let Ok(envelope) = Envelope::decode(frame.into_data())
            && let Some(msg) = envelope.msg
        {
            return Some(msg);
        }
    }
}
