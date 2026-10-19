//! A fresh client finds its account's surviving chair without any local game ID.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{attach_agent, http, json_field, login, spawn_gateway};

#[tokio::test]
async fn own_seats_are_discovered_beyond_every_public_page_and_only_by_the_owner() {
    let gateway = spawn_gateway("resume-discovery");
    let agent = attach_agent(&gateway).await;
    let port = gateway.port;
    let owner = login(port, "discovery-owner", "owner");
    let stranger = login(port, "discovery-stranger", "stranger");
    let (status, body) = http(
        port,
        "POST",
        "/decks",
        Some(&owner),
        r#"{"name":"Private deck","cards":["60 Forest"]}"#,
    );
    assert_eq!(status, 200, "{body}");
    let deck = json_field(&body, "deck_id");
    let (status, body) = http(
        port,
        "POST",
        "/lobby/games",
        Some(&owner),
        &format!(r#"{{"deck_id":"{deck}","mode":"ai"}}"#),
    );
    assert_eq!(status, 200, "{body}");
    let surviving_game = json_field(&body, "game_id").to_string();

    // Waiting rooms sort before playing games in the public listing. Both
    // the client's eight-row page and the gateway's 25-row default miss it.
    for index in 0..30 {
        let (status, body) = http(
            port,
            "POST",
            "/lobby/games",
            Some(&stranger),
            &format!(r#"{{"name":"Other room {index}"}}"#),
        );
        assert_eq!(status, 200, "{body}");
    }
    for path in [
        "/lobby/games",
        "/lobby/games?limit=8",
        "/lobby/games?q=Other",
    ] {
        let (status, body) = http(port, "GET", path, Some(&owner), "");
        assert_eq!(status, 200, "{body}");
        assert!(
            !body.contains(&surviving_game),
            "public page unexpectedly reaches own game: {body}"
        );
    }

    assert_eq!(http(port, "GET", "/lobby/seats", None, "").0, 401);
    let (status, body) = http(
        port,
        "GET",
        "/lobby/seats?q=Other&offset=999",
        Some(&owner),
        "",
    );
    assert_eq!(status, 200, "{body}");
    let discovered: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        discovered,
        serde_json::json!({
            "seats": [{"game_id": surviving_game, "seat": 0, "playing": true}],
        })
    );
    let game = discovered["seats"][0]["game_id"].as_str().unwrap();
    let (status, body) = http(port, "GET", "/lobby/seats", Some(&stranger), "");
    assert_eq!(status, 200, "{body}");
    assert!(!body.contains(game));
    assert_eq!(
        http(
            port,
            "POST",
            &format!("/lobby/games/{game}/seat"),
            Some(&stranger),
            "{}"
        )
        .0,
        403
    );
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/seat"),
        Some(&owner),
        "{}",
    );
    assert_eq!(status, 200, "{body}");
    assert_eq!(json_field(&body, "game_id"), game);
    assert!(!json_field(&body, "seat_token").is_empty());
    agent.abort();
}
