//! `POST /lobby/games` with `mode: "ai"` plays the house at the difficulty
//! Play's caret names (`ai`), `steady` when it names none, and refuses a
//! name that is no difficulty rather than playing some other one.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{attach_agent, http, json_field, login, spawn_gateway};

fn make_deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// The house chair of the player's one running game, as the list says it.
fn house_chair(port: u16, token: &str) -> serde_json::Value {
    let (status, body) = http(port, "GET", "/lobby/games", Some(token), "");
    assert_eq!(status, 200, "{body}");
    let list: serde_json::Value = serde_json::from_str(&body).expect("json");
    list["games"]
        .as_array()
        .expect("games")
        .iter()
        .find(|g| {
            g["seats"]
                .as_array()
                .is_some_and(|s| s.iter().any(|c| c["you"] == true))
        })
        .map_or_else(
            || panic!("my game is listed: {body}"),
            |g| g["seats"][1].clone(),
        )
}

#[tokio::test]
async fn the_house_plays_at_the_difficulty_asked_for() {
    let gw = spawn_gateway("house-difficulty");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let bad = login(port, "picky-player", "Picky");
    let deck = make_deck(port, &bad);
    let body = format!(r#"{{"deck_id":"{deck}","mode":"ai","ai":"invented"}}"#);
    let (status, answer) = http(port, "POST", "/lobby/games", Some(&bad), &body);
    assert_eq!(status, 400, "an unknown difficulty: {answer}");

    let sharp = login(port, "sharp-player", "Sharp");
    let deck = make_deck(port, &sharp);
    let body = format!(r#"{{"deck_id":"{deck}","mode":"ai","ai":"sharp"}}"#);
    let (status, answer) = http(port, "POST", "/lobby/games", Some(&sharp), &body);
    assert_eq!(status, 200, "{answer}");
    let chair = house_chair(port, &sharp);
    assert_eq!(chair["kind"], "ai", "{chair}");
    assert_eq!(chair["ai"], "sharp", "{chair}");

    let plain = login(port, "plain-player", "Plain");
    let deck = make_deck(port, &plain);
    let body = format!(r#"{{"deck_id":"{deck}","mode":"ai"}}"#);
    let (status, answer) = http(port, "POST", "/lobby/games", Some(&plain), &body);
    assert_eq!(status, 200, "{answer}");
    assert_eq!(house_chair(port, &plain)["ai"], "steady");
}
