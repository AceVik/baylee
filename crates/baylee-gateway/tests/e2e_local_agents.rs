//! Agents: whether any is there, and which of them are on this machine.
//!
//! Two answers the gateway gives about the processes behind it. The lobby
//! says whether a game can start at all (`agents_available`), so a client can
//! say so before a player presses anything, and pushes that again when it
//! changes. `/health` says how many running games belong to an agent on the
//! gateway's unix socket (`games.local_running`), which is what a deploy of
//! this machine waits for.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{attach_agent, http, json_field, login, spawn_gateway};
use futures_util::StreamExt;

/// A live websocket to the gateway.
type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// The next lobby push's `agents_available`, or a panic after ten seconds.
async fn next_availability(feed: &mut Socket) -> bool {
    let frame = tokio::time::timeout(std::time::Duration::from_secs(10), feed.next())
        .await
        .expect("a lobby push within ten seconds")
        .expect("the feed is open")
        .expect("a frame");
    let listing: serde_json::Value =
        serde_json::from_str(frame.to_text().expect("the feed speaks text")).expect("json");
    listing["agents_available"]
        .as_bool()
        .unwrap_or_else(|| panic!("every push says agents_available: {listing}"))
}

/// The lobby says whether a game can start, and says it again the moment the
/// first agent arrives and the moment the last one leaves: a client with the
/// lobby open learns it without asking.
#[tokio::test]
async fn the_lobby_says_whether_an_agent_is_there_and_says_it_again_when_that_changes() {
    let gw = spawn_gateway("agents-feed");
    let port = gw.port;
    let watcher = login(port, "agent-watcher", "Watcher");

    let (status, body) = http(port, "GET", "/lobby/games", Some(&watcher), "");
    assert_eq!(status, 200, "{body}");
    assert!(
        body.contains("\"agents_available\":false"),
        "no agent, no game: {body}"
    );

    let url = format!("ws://127.0.0.1:{port}/lobby/ws?token={watcher}");
    let (mut feed, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("lobby feed");
    assert!(!next_availability(&mut feed).await, "the opening page");

    let agent = attach_agent(&gw).await;
    assert!(
        next_availability(&mut feed).await,
        "the first agent in is pushed"
    );

    agent.abort();
    assert!(
        !next_availability(&mut feed).await,
        "and so is the last one out"
    );
}

#[cfg(unix)]
use baylee_protocol::v1::{self, Envelope};
#[cfg(unix)]
use futures_util::SinkExt;
#[cfg(unix)]
use prost::Message as _;

#[cfg(unix)]
async fn over_unix(
    path: &std::path::Path,
    route: &str,
) -> tokio_tungstenite::WebSocketStream<tokio::net::UnixStream> {
    let socket = tokio::net::UnixStream::connect(path).await.expect("unix");
    tokio_tungstenite::client_async(format!("ws://localhost{route}"), socket)
        .await
        .expect("handshake over the unix socket")
        .0
}
#[cfg(unix)]
async fn say<S>(ws: &mut tokio_tungstenite::WebSocketStream<S>, msg: v1::envelope::Msg)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let envelope = Envelope { msg: Some(msg) };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        envelope.encode_to_vec().into(),
    ))
    .await
    .expect("send");
}
#[cfg(unix)]
async fn hear<S>(ws: &mut tokio_tungstenite::WebSocketStream<S>) -> v1::envelope::Msg
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    loop {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(10), ws.next())
            .await
            .expect("an answer within ten seconds")
            .expect("open")
            .expect("a frame");
        if frame.is_binary()
            && let Some(msg) = Envelope::decode(frame.into_data()).expect("protobuf").msg
        {
            return msg;
        }
    }
}

/// A game whose agent came in on the unix socket counts as this machine's,
/// and the socket carries the engine plane as well as the control plane.
#[cfg(unix)]
#[tokio::test]
async fn a_game_ordered_from_an_agent_on_the_unix_socket_is_a_local_one() {
    use std::os::unix::fs::{FileTypeExt as _, PermissionsExt as _};

    let path = std::env::temp_dir().join(format!("baylee-gw-{}.sock", std::process::id()));
    // A file where the socket goes is a leftover of an earlier run, and the
    // gateway clears it rather than failing to bind.
    std::fs::write(&path, b"stale").expect("a stale file");
    let gw = common::spawn_gateway_with(
        "agents-unix",
        &[("BAYLEE_UNIX_SOCKET", path.display().to_string())],
    );
    let port = gw.port;

    let meta = std::fs::metadata(&path).expect("the socket exists");
    assert!(meta.file_type().is_socket(), "the stale file was replaced");
    assert_eq!(meta.permissions().mode() & 0o777, 0o660, "owner and group");

    let mut agent = over_unix(&path, "/agent/ws").await;
    say(
        &mut agent,
        v1::envelope::Msg::AgentHello(v1::AgentHello {
            token: gw.agent_token.clone(),
            name: "local".to_string(),
            capacity: 0,
            protocol_version: baylee_protocol::PROTOCOL_VERSION,
        }),
    )
    .await;
    assert!(matches!(
        hear(&mut agent).await,
        v1::envelope::Msg::AgentWelcome(_)
    ));

    let host = login(port, "local-host", "Local");
    let (status, body) = http(
        port,
        "POST",
        "/decks",
        Some(&host),
        "{\"name\":\"d\",\"cards\":[\"40 Forest\",\"20 Swamp\"]}",
    );
    assert_eq!(status, 200, "{body}");
    let deck = json_field(&body, "deck_id").to_string();
    let (status, body) = http(
        port,
        "POST",
        "/lobby/games",
        Some(&host),
        &format!("{{\"deck_id\":\"{deck}\",\"mode\":\"ai\"}}"),
    );
    assert_eq!(status, 200, "{body}");

    let v1::envelope::Msg::StartEngine(start) = hear(&mut agent).await else {
        panic!("the local agent is ordered to start the game");
    };
    let mut engine = over_unix(&path, "/engine/ws").await;
    say(
        &mut engine,
        v1::envelope::Msg::EngineHello(v1::EngineHello {
            game_id: start.game_id.clone(),
            token: start.engine_token.clone(),
            protocol_version: baylee_protocol::PROTOCOL_VERSION,
        }),
    )
    .await;
    assert!(
        matches!(hear(&mut engine).await, v1::envelope::Msg::GameSetup(_)),
        "the engine plane answers on the unix socket too"
    );

    let (status, body) = http(port, "GET", "/health", None, "");
    assert_eq!(status, 200, "{body}");
    let health: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(health["games"]["running"], 1, "{health}");
    assert_eq!(health["games"]["local_running"], 1, "{health}");

    // The same machine's agent going away does not make its game anyone
    // else's: a deploy stops the agent first and still waits for this one.
    drop(agent);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let (_, body) = http(port, "GET", "/health", None, "");
    let health: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(health["agents"]["connected"], 0, "{health}");
    assert_eq!(health["games"]["local_running"], 1, "{health}");
    let _ = std::fs::remove_file(&path);
}

/// An agent over TCP is somewhere else as far as the gateway knows, and its
/// games are not this machine's to wait for.
#[tokio::test]
async fn a_game_on_an_agent_over_tcp_is_not_a_local_one() {
    let gw = spawn_gateway("agents-tcp");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let host = login(port, "tcp-host", "Remote");
    let (_, body) = http(
        port,
        "POST",
        "/decks",
        Some(&host),
        "{\"name\":\"d\",\"cards\":[\"40 Forest\",\"20 Swamp\"]}",
    );
    let deck = json_field(&body, "deck_id").to_string();
    let (status, body) = http(
        port,
        "POST",
        "/lobby/games",
        Some(&host),
        &format!("{{\"deck_id\":\"{deck}\",\"mode\":\"ai\"}}"),
    );
    assert_eq!(status, 200, "{body}");
    let (_, body) = http(port, "GET", "/health", None, "");
    let health: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(health["games"]["running"], 1, "{health}");
    assert_eq!(health["games"]["local_running"], 0, "{health}");
}
