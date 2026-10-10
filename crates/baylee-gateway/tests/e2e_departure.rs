//! A player whose client went away (`docs/protocol.md` §"Leaving, and
//! losing the connection"), in a waiting room.
//!
//! The owner's report on beta.6: signed in, opened a room, closed the
//! client — and the room stayed, with him still in it. A waiting room has
//! no engine to hold a chair, and nothing tied the room to the client's
//! sockets at all. Now the last socket closing arms a departure; if the
//! account opens none within the grace (`BAYLEE_ROOM_GRACE_SECS`, shortened
//! here) it leaves every room as its own Leave would. A client quitting on
//! purpose says so (`POST /lobby/depart`) and is not waited for.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{Socket, attach_agent, http, json_field, login, spawn_gateway_with};
use std::time::Duration;

/// The grace the tests run under, in seconds.
const GRACE: u64 = 2;

fn gateway(label: &str) -> common::Gateway {
    spawn_gateway_with(label, &[("BAYLEE_ROOM_GRACE_SECS", GRACE.to_string())])
}

fn make_deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// A three-chair room `host` opened and `guest` joined; its id.
fn room_of_two(port: u16, host: &str, guest: &str) -> String {
    let deck = make_deck(port, host);
    let create = format!("{{\"deck_id\":\"{deck}\",\"seats\":3,\"name\":\"Den\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(host), &create);
    assert_eq!(status, 200, "create room: {body}");
    let id = json_field(&body, "game_id").to_string();
    let deck = make_deck(port, guest);
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{id}/join"),
        Some(guest),
        &format!("{{\"deck_id\":\"{deck}\"}}"),
    );
    assert_eq!(status, 200, "join: {body}");
    id
}

/// The room as `reader` sees it in the listing, if it is listed.
fn room(port: u16, reader: &str, id: &str) -> Option<serde_json::Value> {
    let (status, body) = http(port, "GET", "/lobby/games", Some(reader), "");
    assert_eq!(status, 200, "{body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    value["games"]
        .as_array()
        .expect("games")
        .iter()
        .find(|g| g["id"] == id)
        .cloned()
}

/// Whose chair is the host's, and who sits, as handles.
fn sitting(room: &serde_json::Value) -> (Option<String>, Vec<String>) {
    let seats = room["seats"].as_array().expect("seats");
    let host = seats
        .iter()
        .find(|s| s["host"] == true)
        .and_then(|s| s["player"].as_str())
        .map(str::to_string);
    let players = seats
        .iter()
        .filter_map(|s| s["player"].as_str())
        .map(str::to_string)
        .collect();
    (host, players)
}

fn online(port: u16, token: &str) -> i64 {
    let (status, body) = http(port, "GET", "/lobby/stats", Some(token), "");
    assert_eq!(status, 200, "{body}");
    common::json_number(&body, "players_online")
}

async fn lobby_socket(port: u16, token: &str) -> Socket {
    let url = common::lobby_url(port, token, "");
    tokio_tungstenite::connect_async(&url)
        .await
        .expect("open the lobby socket")
        .0
}

/// Waits until `check` holds, for up to `secs`.
async fn until(secs: u64, what: &str, mut check: impl FnMut() -> bool) {
    let end = std::time::Instant::now() + Duration::from_secs(secs);
    while std::time::Instant::now() < end {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("{what}: not within {secs} s");
}

/// (a) The host's client goes away: after the grace the host has left the
/// room as its Leave would (the room passes to the guest, who is told on
/// the lobby feed), and is offline.
#[tokio::test]
async fn a_host_whose_client_went_away_leaves_the_room_after_the_grace() {
    let gw = gateway("depart-host");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let host = login(port, "gone-host", "Gone");
    let guest = login(port, "stay-guest", "Stay");
    let id = room_of_two(port, &host, &guest);

    let mut host_socket = lobby_socket(port, &host).await;
    let mut guest_socket = lobby_socket(port, &guest).await;
    until(5, "both online", || online(port, &guest) == 2).await;
    let (owner, _) = sitting(&room(port, &guest, &id).expect("listed"));
    assert!(
        owner.as_deref().is_some_and(|h| h.starts_with("Gone")),
        "{owner:?}"
    );

    // The client closes; nothing is said.
    drop(host_socket.close(None).await);
    drop(host_socket);
    until(5, "the host offline at once", || online(port, &guest) == 1).await;
    // Still seated inside the grace: a restart into an update comes back.
    let (_, players) = sitting(&room(port, &guest, &id).expect("listed"));
    assert_eq!(
        players.len(),
        2,
        "left before the grace ran out: {players:?}"
    );

    until(GRACE + 5, "the host left the room", || {
        room(port, &guest, &id).is_some_and(|r| sitting(&r).1.len() == 1)
    })
    .await;
    let (owner, players) = sitting(&room(port, &guest, &id).expect("still listed"));
    assert!(
        owner.as_deref().is_some_and(|h| h.starts_with("Stay")),
        "the room passes to whoever is still in it: {owner:?}"
    );
    assert!(players.iter().all(|p| p.starts_with("Stay")), "{players:?}");

    // The guest was told: the feed pushed a listing without the host.
    let told = tokio::time::timeout(Duration::from_secs(5), async {
        use futures_util::StreamExt as _;
        while let Some(Ok(frame)) = guest_socket.next().await {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = frame
                && !text.contains("Gone")
                && text.contains(&id)
            {
                return true;
            }
        }
        false
    })
    .await;
    assert_eq!(
        told,
        Ok(true),
        "the guest's feed never showed the host gone"
    );
}

/// A room whose only player went away is closed.
#[tokio::test]
async fn a_room_nobody_is_left_in_closes() {
    let gw = gateway("depart-alone");
    let port = gw.port;
    let host = login(port, "lone-host", "Lone");
    let reader = login(port, "lone-reader", "Reader");
    let deck = make_deck(port, &host);
    let create = format!("{{\"deck_id\":\"{deck}\",\"seats\":2,\"name\":\"Solo\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &create);
    assert_eq!(status, 200, "{body}");
    let id = json_field(&body, "game_id").to_string();
    let socket = lobby_socket(port, &host).await;
    until(5, "online", || online(port, &reader) == 1).await;
    drop(socket);
    until(GRACE + 5, "the room closed", || {
        room(port, &reader, &id).is_none()
    })
    .await;
    assert_eq!(online(port, &reader), 0);
}

/// (b) A client that comes back inside the grace keeps its room — also
/// when it goes away again later, which is then waited for anew.
#[tokio::test]
async fn coming_back_inside_the_grace_keeps_the_room() {
    let gw = gateway("depart-back");
    let port = gw.port;
    let host = login(port, "back-host", "Back");
    let guest = login(port, "back-guest", "Guest");
    let id = room_of_two(port, &host, &guest);

    let first = lobby_socket(port, &host).await;
    until(5, "online", || online(port, &guest) == 1).await;
    drop(first);
    until(5, "offline", || online(port, &guest) == 0).await;
    tokio::time::sleep(Duration::from_millis(GRACE * 1000 / 2)).await;
    let second = lobby_socket(port, &host).await;
    // Past the first departure's due time, with the client back.
    tokio::time::sleep(Duration::from_millis(GRACE * 1000)).await;
    let (owner, players) = sitting(&room(port, &guest, &id).expect("listed"));
    assert_eq!(players.len(), 2, "a client that came back lost its chair");
    assert!(owner.is_some_and(|h| h.starts_with("Back")));
    drop(second);
    until(GRACE + 5, "the later departure acted", || {
        room(port, &guest, &id).is_some_and(|r| sitting(&r).1.len() == 1)
    })
    .await;
}

/// (e) A client that quits on purpose says so and is not waited for.
#[tokio::test]
async fn an_explicit_departure_leaves_at_once() {
    // A grace far longer than the test: only the explicit word can act.
    let gw = spawn_gateway_with(
        "depart-now",
        &[("BAYLEE_ROOM_GRACE_SECS", "600".to_string())],
    );
    let port = gw.port;
    let host = login(port, "now-host", "Now");
    let guest = login(port, "now-guest", "Guest");
    let id = room_of_two(port, &host, &guest);
    let socket = lobby_socket(port, &host).await;
    until(5, "online", || online(port, &guest) == 1).await;

    let (status, body) = http(port, "POST", "/lobby/depart", None, "");
    assert_eq!(status, 401, "a stranger departed somebody: {body}");
    let (status, body) = http(port, "POST", "/lobby/depart", Some(&host), "");
    assert_eq!(status, 204, "{body}");
    let (owner, players) = sitting(&room(port, &guest, &id).expect("listed"));
    assert_eq!(players.len(), 1, "{players:?}");
    assert!(owner.is_some_and(|h| h.starts_with("Guest")));
    drop(socket);
    until(5, "offline", || online(port, &guest) == 0).await;
    // Nothing left to leave is no error.
    let (status, _) = http(port, "POST", "/lobby/depart", Some(&host), "");
    assert_eq!(status, 204);
}
