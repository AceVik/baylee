//! `GET /lobby/stats` (WG-0): three numbers that move with the lobby.
//!
//! "Online" is the hard one, because a session row is not presence: a
//! guest's lasts a month. So it is checked against the two things that are —
//! a lobby socket held open, and a chair at a running game — and against
//! the one that is not: a signed-in account that is doing neither.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{attach_agent, http, json_field, json_number, login, spawn_gateway};

/// The three numbers, as `(players_online, tables_waiting, games_running)`.
fn stats(port: u16, token: &str) -> (i64, i64, i64) {
    let (status, body) = http(port, "GET", "/lobby/stats", Some(token), "");
    assert_eq!(status, 200, "stats: {body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    let keys: Vec<&String> = value.as_object().expect("an object").keys().collect();
    assert_eq!(
        keys,
        ["games_running", "players_online", "tables_waiting"],
        "the answer carries more than its three numbers: {body}"
    );
    (
        json_number(&body, "players_online"),
        json_number(&body, "tables_waiting"),
        json_number(&body, "games_running"),
    )
}

/// Waits for `players_online` to read `want`: a socket closing is noticed
/// by the gateway's task for it, a moment after the client let go.
async fn online(port: u16, token: &str, want: i64) {
    for _ in 0..500 {
        if stats(port, token).0 == want {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!(
        "players_online never reached {want}: {:?}",
        stats(port, token)
    );
}

fn make_deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

#[tokio::test]
async fn the_numbers_move_with_a_socket_a_table_and_a_game() {
    let gw = spawn_gateway("lobby-stats");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let (status, _) = http(port, "GET", "/lobby/stats", None, "");
    assert_eq!(status, 401, "a stranger read how busy the gateway is");

    // Signed in and doing nothing is not online.
    let idle = login(port, "idle-player", "Idle");
    assert_eq!(stats(port, &idle), (0, 0, 0));

    // A lobby socket is.
    let watcher = login(port, "watch-player", "Watcher");
    let url = common::lobby_url(port, &watcher, "");
    let (mut socket, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("open the lobby socket");
    online(port, &idle, 1).await;

    // A waiting table counts as a table, and its host (no socket open, no
    // game running) is not online for having opened it.
    let host = login(port, "host-player", "Host");
    let deck = make_deck(port, &host);
    let create = format!("{{\"deck_id\":\"{deck}\",\"seats\":2,\"name\":\"Waiting\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &create);
    assert_eq!(status, 200, "{body}");
    assert_eq!(stats(port, &idle), (1, 1, 0));

    // Closing the socket takes the watcher out again.
    socket.close(None).await.expect("close");
    drop(socket);
    online(port, &idle, 0).await;

    // A game against the house runs at once; its player is online while
    // their chair's socket is open, and not for holding the chair: a chair
    // whose connection is gone is held, its player is not here.
    let player = login(port, "game-player", "Player");
    let deck = make_deck(port, &player);
    let create = format!("{{\"deck_id\":\"{deck}\",\"mode\":\"ai\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&player), &create);
    assert_eq!(status, 200, "{body}");
    assert_eq!(stats(port, &idle), (0, 1, 1));
    let game_id = json_field(&body, "game_id").to_string();
    let seat_token = json_field(&body, "seat_token").to_string();
    let seat = common::dial_seat(port, &game_id, &seat_token).await;
    online(port, &idle, 1).await;
    drop(seat);
    online(port, &idle, 0).await;
}
