//! The lobby listing: searched, ordered, paged, and pushed.
//!
//! Two things are being checked here that a single-page listing never had to
//! answer. **Order**, because games live in a `HashMap` and paging an
//! unordered collection hands out rows twice and drops others. And **push**,
//! because the lobby used to re-read the list every two seconds from every
//! client that had it open, which is the only way it could learn that a chair
//! had moved.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{attach_agent, http, json_field, login, spawn_gateway};
use futures_util::StreamExt;

/// A live websocket to the gateway.
type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// A deck this account owns, and its id.
fn make_deck(port: u16, token: &str, name: &str) -> String {
    let body = format!("{{\"name\":\"{name}\",\"cards\":[\"40 Forest\",\"20 Swamp\"]}}");
    let (status, body) = http(port, "POST", "/decks", Some(token), &body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// The table names in a listing, in the order it gave them.
fn names(body: &str) -> Vec<String> {
    body.split("\"name\":\"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap_or_default().to_string())
        .filter(|n| !n.is_empty())
        .collect()
}

#[tokio::test]
async fn the_listing_is_searched_ordered_and_paged() {
    let gw = spawn_gateway("lobby-page");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let host = login(port, "p1@example.com", "Pager");
    let deck = make_deck(port, &host, "d");

    // Six tables, opened in a known order. `created_at` is whole seconds, so
    // several of these share one — which is exactly the case the id tiebreak
    // exists for, and why this asserts on the *set* of a page rather than on
    // which of two same-second rooms comes first.
    for name in [
        "Kitchen",
        "Cellar",
        "Kitchen table",
        "Garden",
        "Attic",
        "Shed",
    ] {
        let create = format!("{{\"deck_id\":\"{deck}\",\"seats\":2,\"name\":\"{name}\"}}");
        let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &create);
        assert_eq!(status, 200, "open {name}: {body}");
    }

    let (status, body) = http(port, "GET", "/lobby/games", Some(&host), "");
    assert_eq!(status, 200);
    assert!(body.contains("\"total\":6"), "{body}");
    assert_eq!(names(&body).len(), 6, "one page holds all six: {body}");

    // Search matches the table's name, and matches it loosely: "kitchen"
    // finds "Kitchen" and "Kitchen table" and nothing else.
    let (_, body) = http(port, "GET", "/lobby/games?q=kitchen", Some(&host), "");
    let found = names(&body);
    assert!(body.contains("\"total\":2"), "{body}");
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.iter().all(|n| n.starts_with("Kitchen")), "{found:?}");

    // …and the host's name, because a player looking for a table knows one or
    // the other and rarely both.
    let (_, body) = http(port, "GET", "/lobby/games?q=pager", Some(&host), "");
    assert!(
        body.contains("\"total\":6"),
        "every table is Pager's: {body}"
    );
    let (_, body) = http(port, "GET", "/lobby/games?q=nobody", Some(&host), "");
    assert!(body.contains("\"total\":0"), "{body}");

    // Paging covers the listing exactly once. This is the assertion the
    // ordering exists for: over an unordered map these two pages would
    // overlap and miss rows, and nothing about one page alone would show it.
    let (_, first) = http(port, "GET", "/lobby/games?limit=4", Some(&host), "");
    let (_, second) = http(
        port,
        "GET",
        "/lobby/games?limit=4&offset=4",
        Some(&host),
        "",
    );
    let mut seen = names(&first);
    assert_eq!(seen.len(), 4, "{first}");
    assert_eq!(names(&second).len(), 2, "{second}");
    seen.extend(names(&second));
    seen.sort();
    let mut every = vec![
        "Attic".to_string(),
        "Cellar".to_string(),
        "Garden".to_string(),
        "Kitchen table".to_string(),
        "Kitchen".to_string(),
        "Shed".to_string(),
    ];
    every.sort();
    assert_eq!(seen, every, "two pages are the whole lobby, once each");

    // The page is stable: asking twice gives the same rows in the same order,
    // which a `HashMap` iteration does not.
    let (_, again) = http(port, "GET", "/lobby/games?limit=4", Some(&host), "");
    assert_eq!(
        names(&first),
        names(&again),
        "the order moved between reads"
    );
}

#[tokio::test]
async fn the_lobby_socket_pushes_a_change_nobody_asked_about() {
    let gw = spawn_gateway("lobby-feed");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let watcher = login(port, "w@example.com", "Watcher");
    let opener = login(port, "o@example.com", "Opener");
    let deck = make_deck(port, &opener, "d");

    let url = format!("ws://127.0.0.1:{port}/lobby/ws?token={watcher}");
    let (mut socket, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("open the lobby socket");

    // The listing arrives unasked, before anything has happened.
    let first = next_text(&mut socket).await;
    assert!(first.contains("\"total\":0"), "{first}");

    // Somebody else opens a table. Nothing polls; the socket says so.
    let create = format!("{{\"deck_id\":\"{deck}\",\"seats\":2,\"name\":\"Pushed\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&opener), &create);
    assert_eq!(status, 200, "{body}");

    let second = next_text(&mut socket).await;
    assert!(second.contains("Pushed"), "{second}");
    assert!(second.contains("\"total\":1"), "{second}");
    // Rendered for *this* reader: the room is the opener's, not the
    // watcher's, and the socket has to say so per socket rather than
    // broadcasting one answer to everybody.
    assert!(second.contains("\"yours\":false"), "{second}");
}

/// The socket takes its token and its query out of **one** query string, which
/// serde flattens — and a flattened struct is deserialized from a map of
/// strings, so `offset=8` reached a `usize` field as `"8"` and the upgrade was
/// refused with a `400`. The HTTP route parses the same struct without a
/// flatten and never saw it, so nothing but a real client noticed.
#[tokio::test]
async fn the_lobby_socket_takes_a_page_and_not_only_a_token() {
    let gw = spawn_gateway("lobby-feed-query");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let watcher = login(port, "q@example.com", "Querier");
    let deck = make_deck(port, &watcher, "d");
    for name in ["One", "Two", "Three"] {
        let create = format!("{{\"deck_id\":\"{deck}\",\"seats\":2,\"name\":\"{name}\"}}");
        let (status, body) = http(port, "POST", "/lobby/games", Some(&watcher), &create);
        assert_eq!(status, 200, "{body}");
    }

    let url = format!(
        "ws://127.0.0.1:{port}/lobby/ws?token={watcher}&q=&offset=2&limit=2&waiting_only=true"
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("the socket refused a page");
    let first = next_text(&mut socket).await;
    assert!(first.contains("\"offset\":2"), "{first}");
    assert!(first.contains("\"limit\":2"), "{first}");
    assert!(first.contains("\"total\":3"), "{first}");
    assert_eq!(names(&first).len(), 1, "the third table, alone: {first}");
}

#[tokio::test]
async fn the_lobby_socket_refuses_a_token_it_does_not_know() {
    let gw = spawn_gateway("lobby-feed-auth");
    let url = format!("ws://127.0.0.1:{}/lobby/ws?token=not-a-token", gw.port);
    assert!(
        tokio_tungstenite::connect_async(&url).await.is_err(),
        "the lobby socket let a stranger in"
    );
}

/// The next text frame, or a panic if the socket says nothing in time.
async fn next_text(socket: &mut Socket) -> String {
    let deadline = std::time::Duration::from_secs(5);
    loop {
        let frame = tokio::time::timeout(deadline, socket.next())
            .await
            .expect("the lobby socket said nothing")
            .expect("the lobby socket closed")
            .expect("the lobby socket errored");
        if let tokio_tungstenite::tungstenite::Message::Text(text) = frame {
            return text.to_string();
        }
    }
}
