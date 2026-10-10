//! The admission hold (`BAYLEE_ADMISSION_HOLD`, `docs/deploy-hooks.md`):
//! while the file exists no new game starts, by any route and on any
//! agent, games already running go on, and `/health` says which it is.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{attach_agent, http, json_field, login, spawn_gateway, spawn_gateway_with};

fn make_deck(port: u16, token: &str) -> String {
    let body = "{\"name\":\"d\",\"cards\":[\"40 Forest\",\"20 Swamp\"]}";
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

fn health(port: u16) -> serde_json::Value {
    let (status, body) = http(port, "GET", "/health", None, "");
    assert_eq!(status, 200, "{body}");
    serde_json::from_str(&body).expect("json")
}

/// A room of the host and one AI chair, ready to start.
fn room(port: u16, host: &str, deck: &str) -> String {
    let (status, body) = http(
        port,
        "POST",
        "/lobby/games",
        Some(host),
        &format!("{{\"deck_id\":\"{deck}\",\"seats\":2}}"),
    );
    assert_eq!(status, 200, "create room: {body}");
    let game = json_field(&body, "game_id").to_string();
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/seats/1"),
        Some(host),
        "{\"kind\":\"ai\",\"ai\":\"steady\"}",
    );
    assert_eq!(status, 200, "AI chair: {body}");
    game
}

#[tokio::test(flavor = "multi_thread")]
async fn while_the_hold_file_exists_no_game_starts_and_running_ones_go_on() {
    let hold = std::env::temp_dir().join(format!("baylee-admission-{}", std::process::id()));
    let _ = std::fs::remove_file(&hold);
    let gw = spawn_gateway_with(
        "admission",
        &[("BAYLEE_ADMISSION_HOLD", hold.display().to_string())],
    );
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let host = login(port, "hold-host", "Host");
    let deck = make_deck(port, &host);
    let one_tap = format!("{{\"deck_id\":\"{deck}\",\"mode\":\"ai\"}}");

    assert_eq!(health(port)["admission"], "open");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &one_tap);
    assert_eq!(status, 200, "open: {body}");
    let waiting_room = room(port, &host, &deck);
    assert_eq!(health(port)["games"]["running"], 1);

    // The deploy places the hold.
    std::fs::write(&hold, b"").unwrap();
    let held = health(port);
    assert_eq!(held["admission"], "held", "{held}");
    assert_eq!(held["games"]["running"], 1, "the running game goes on");

    let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &one_tap);
    assert_eq!(status, 503, "a one-tap game: {body}");
    assert!(body.contains("being updated"), "{body}");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{waiting_room}/start"),
        Some(&host),
        "",
    );
    assert_eq!(status, 503, "a room's start: {body}");
    assert!(body.contains("being updated"), "{body}");
    // Nothing was left half-started: still one running, the room waits.
    let after = health(port);
    assert_eq!(after["games"]["running"], 1, "{after}");
    assert_eq!(after["games"]["waiting"], 1, "{after}");

    // Lifted, the same room starts.
    std::fs::remove_file(&hold).unwrap();
    assert_eq!(health(port)["admission"], "open");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{waiting_room}/start"),
        Some(&host),
        "",
    );
    assert_eq!(status, 200, "{body}");
    assert_eq!(health(port)["games"]["running"], 2);
}

/// Without the setting the gateway holds nothing and says so, which is how
/// a deploy tells a gateway that would ignore the file.
#[tokio::test(flavor = "multi_thread")]
async fn a_gateway_without_the_setting_says_it_is_unmanaged() {
    let gw = spawn_gateway("admission-unmanaged");
    assert_eq!(health(gw.port)["admission"], "unmanaged");
}
