//! End-to-end tests for rooms: a table whose seats are arranged in the open
//! before anyone plays.
//!
//! A room is a lobby game the host configures — how many chairs, which of
//! them are people and which are the AI, how hard each AI plays, and what
//! everyone brings. Every other player at the table sees all of it and sets
//! exactly one thing, the deck they themselves will play.
//!
//! There is no start button on the wire. A room starts the moment every chair
//! is ready, which is the same rule the two-seat open table has always
//! followed when its second player sat down; a host who has given up waiting
//! hands the empty chair to the AI, and that is the start.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{attach_agent, http, json_field, login, spawn_gateway};

/// A deck this account owns, and its id.
fn make_deck(port: u16, token: &str, name: &str) -> String {
    let body = format!("{{\"name\":\"{name}\",\"cards\":[\"40 Forest\",\"20 Swamp\"]}}");
    let (status, body) = http(port, "POST", "/decks", Some(token), &body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// How many seats the listed room has, and how many of them are ready.
fn seat_counts(body: &str) -> (usize, usize) {
    (
        body.matches("\"seat\":").count(),
        body.matches("\"ready\":true").count(),
    )
}

#[tokio::test]
async fn a_room_is_arranged_in_the_open_and_starts_when_every_chair_is_ready() {
    let gw = spawn_gateway("rooms");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let host = login(port, "host@example.com", "host");
    let guest = login(port, "guest@example.com", "guest");
    let host_deck = make_deck(port, &host, "host-deck");
    let guest_deck = make_deck(port, &guest, "guest-deck");

    // A three-seat table: the host, and two chairs to arrange.
    let create = format!("{{\"deck_id\":\"{host_deck}\",\"seats\":3,\"name\":\"Kitchen table\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &create);
    assert_eq!(status, 200, "create room: {body}");
    let game_id = json_field(&body, "game_id").to_string();

    // Everyone can see how it is arranged, before anyone has played anything.
    let (status, listing) = http(port, "GET", "/lobby/games", Some(&guest), "");
    assert_eq!(status, 200);
    assert!(listing.contains("Kitchen table"), "{listing}");
    let (seats, ready) = seat_counts(&listing);
    assert_eq!(seats, 3, "three chairs: {listing}");
    assert_eq!(ready, 1, "only the host has a deck yet: {listing}");

    // The host hands the third chair to the AI. Two chairs are ready now, and
    // the table still waits — seat 1 is a person who has not arrived.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/seats/2"),
        Some(&host),
        "{\"kind\":\"ai\",\"ai\":\"sharp\"}",
    );
    assert_eq!(status, 200, "arrange seat 2: {body}");
    assert!(body.contains("\"ai\":\"sharp\""), "{body}");
    let (status, listing) = http(port, "GET", "/lobby/games", Some(&guest), "");
    assert_eq!(status, 200);
    assert!(listing.contains("\"state\":\"waiting\""), "{listing}");

    // The guest takes the free chair, which is the last one open — and that
    // is the start.
    let join = format!("{{\"deck_id\":\"{guest_deck}\"}}");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/join"),
        Some(&guest),
        &join,
    );
    assert_eq!(status, 200, "join: {body}");
    assert!(body.contains("\"seat\":1"), "the open chair: {body}");
    assert!(!json_field(&body, "seat_token").is_empty());

    let (status, listing) = http(port, "GET", "/lobby/games", Some(&guest), "");
    assert_eq!(status, 200);
    assert!(
        listing.contains("\"state\":\"playing\""),
        "every chair ready starts the game: {listing}"
    );
}

#[tokio::test]
async fn only_the_host_arranges_the_table_and_only_a_player_brings_their_own_deck() {
    let gw = spawn_gateway("rooms-authority");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let host = login(port, "h2@example.com", "hosttwo");
    let guest = login(port, "g2@example.com", "guesttwo");
    let host_deck = make_deck(port, &host, "hd");
    let guest_deck = make_deck(port, &guest, "gd");

    let create = format!("{{\"deck_id\":\"{host_deck}\",\"seats\":4}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &create);
    assert_eq!(status, 200, "{body}");
    let game_id = json_field(&body, "game_id").to_string();

    // A guest who is not even at the table arranges nothing.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/seats/3"),
        Some(&guest),
        "{\"kind\":\"ai\"}",
    );
    assert_eq!(status, 403, "not your seat: {body}");

    // Having sat down, the guest still arranges nothing but their own deck.
    let join = format!("{{\"deck_id\":\"{guest_deck}\",\"seat\":1}}");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/join"),
        Some(&guest),
        &join,
    );
    assert_eq!(status, 200, "join seat 1: {body}");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/seats/2"),
        Some(&guest),
        "{\"kind\":\"ai\"}",
    );
    assert_eq!(status, 403, "only the host arranges seats: {body}");

    // And the host does not reach into an occupied chair.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/seats/1"),
        Some(&host),
        "{\"kind\":\"ai\"}",
    );
    assert_eq!(status, 409, "someone is sitting there: {body}");

    // A deck that is not yours is not a deck you can seat anywhere.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/seats/1"),
        Some(&guest),
        &format!("{{\"deck_id\":\"{host_deck}\"}}"),
    );
    assert_eq!(status, 403, "not your deck: {body}");

    // An AI difficulty that does not exist is refused rather than defaulted:
    // a table that quietly plays at another level than it advertises is worse
    // than one that says no.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/seats/2"),
        Some(&host),
        "{\"kind\":\"ai\",\"ai\":\"unbeatable\"}",
    );
    assert_eq!(status, 400, "no such AI: {body}");
}

#[tokio::test]
async fn a_guest_leaving_frees_the_chair_and_a_host_leaving_closes_the_room() {
    let gw = spawn_gateway("rooms-leaving");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let host = login(port, "h3@example.com", "hostthree");
    let guest = login(port, "g3@example.com", "guestthree");
    let host_deck = make_deck(port, &host, "hd");
    let guest_deck = make_deck(port, &guest, "gd");

    let create = format!("{{\"deck_id\":\"{host_deck}\",\"seats\":3}}");
    let (_, body) = http(port, "POST", "/lobby/games", Some(&host), &create);
    let game_id = json_field(&body, "game_id").to_string();

    let join = format!("{{\"deck_id\":\"{guest_deck}\"}}");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/join"),
        Some(&guest),
        &join,
    );
    assert_eq!(status, 200, "{body}");

    let (status, _) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/leave"),
        Some(&guest),
        "",
    );
    assert_eq!(status, 204);
    let (_, listing) = http(port, "GET", "/lobby/games", Some(&host), "");
    let (seats, ready) = seat_counts(&listing);
    assert_eq!(seats, 3, "the chair is still there: {listing}");
    assert_eq!(ready, 1, "and it is empty again: {listing}");

    // The room is the host's; without them nobody can arrange it, and a table
    // that can never start should not be advertised as one that might.
    let (status, _) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/leave"),
        Some(&host),
        "",
    );
    assert_eq!(status, 204);
    let (_, listing) = http(port, "GET", "/lobby/games", Some(&guest), "");
    assert!(!listing.contains(&game_id), "room is gone: {listing}");
}

#[tokio::test]
async fn a_table_seats_between_two_and_four() {
    let gw = spawn_gateway("rooms-size");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let host = login(port, "h4@example.com", "hostfour");
    let deck = make_deck(port, &host, "d");

    for (chairs, expected) in [(1, 400), (2, 200), (4, 200), (5, 400)] {
        let create = format!("{{\"deck_id\":\"{deck}\",\"seats\":{chairs}}}");
        let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &create);
        assert_eq!(status, expected, "{chairs} chairs: {body}");
    }
}
