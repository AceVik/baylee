//! The lobby listing says how long a running table has been going
//! (`running_secs`, beta.7), and says nothing of the kind for a room that
//! still waits.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{attach_agent, http, json_field, login, spawn_gateway};

fn make_deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

#[tokio::test]
async fn a_running_table_says_how_long_it_has_run_and_a_waiting_room_does_not() {
    let gw = spawn_gateway("lobby-running");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let host = login(port, "wait-host", "Host");
    let deck = make_deck(port, &host);
    let create = format!("{{\"deck_id\":\"{deck}\",\"seats\":2,\"name\":\"Waiting\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &create);
    assert_eq!(status, 200, "{body}");

    let player = login(port, "run-player", "Player");
    let deck = make_deck(port, &player);
    let create = format!("{{\"deck_id\":\"{deck}\",\"mode\":\"ai\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&player), &create);
    assert_eq!(status, 200, "{body}");

    let (status, body) = http(port, "GET", "/lobby/games", Some(&host), "");
    assert_eq!(status, 200, "{body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("json");
    let games = value["games"].as_array().expect("games");
    assert_eq!(games.len(), 2, "{body}");
    let mut seen = (false, false);
    for game in games {
        let running = &game["running_secs"];
        match game["state"].as_str() {
            Some("waiting") => {
                seen.0 = true;
                assert!(running.is_null(), "a waiting room ran: {game}");
            }
            Some("playing") => {
                seen.1 = true;
                let secs = running.as_u64().expect("a running table says how long");
                assert!(secs < 60, "a game started just now ran {secs} s");
            }
            other => panic!("unexpected state {other:?}"),
        }
    }
    assert_eq!(seen, (true, true), "{body}");
}
