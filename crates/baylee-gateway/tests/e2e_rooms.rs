//! End-to-end tests for rooms: a table whose seats are arranged in the open
//! before anyone plays.
//!
//! A room is a lobby game the host configures — how many chairs, which of
//! them are people and which are the AI, how hard each AI plays, and what
//! everyone brings. Every other player at the table sees all of it and sets
//! exactly one thing, the deck they themselves will play.
//!
//! Starting takes two different statements by two different people: every
//! player says they are ready, and the host says go. A room used to start
//! itself the moment the last chair had a deck, which meant picking a deck to
//! look at it put you in a game.

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

/// Says this player is ready, and asserts the gateway took it.
fn say_ready(port: u16, token: &str, game_id: &str) {
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/ready"),
        Some(token),
        "{}",
    );
    assert_eq!(status, 200, "ready: {body}");
}

#[tokio::test]
async fn a_room_is_arranged_in_the_open_and_starts_when_the_host_says_so() {
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
    assert_eq!(
        ready, 0,
        "the host has a deck but has not said they are ready: {listing}"
    );

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

    // The guest takes the free chair. The table is full, and still waiting:
    // sitting down is not the same statement as being ready to play.
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
    let (_, listing) = http(port, "GET", "/lobby/games", Some(&guest), "");
    assert!(listing.contains("\"state\":\"waiting\""), "{listing}");

    // The host cannot start a room that is not ready, however much they own
    // it.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/start"),
        Some(&host),
        "",
    );
    assert_eq!(status, 409, "nobody has said ready: {body}");

    // Both players say so. The AI chair needed nothing — it is ready as soon
    // as it is configured.
    say_ready(port, &host, &game_id);
    say_ready(port, &guest, &game_id);
    let (_, listing) = http(port, "GET", "/lobby/games", Some(&guest), "");
    assert!(listing.contains("\"startable\":true"), "{listing}");
    assert!(
        listing.contains("\"state\":\"waiting\""),
        "ready is not the same as started: {listing}"
    );

    // A guest cannot start someone else's room.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/start"),
        Some(&guest),
        "",
    );
    assert_eq!(status, 403, "{body}");

    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/start"),
        Some(&host),
        "",
    );
    assert_eq!(status, 200, "start: {body}");
    let (_, listing) = http(port, "GET", "/lobby/games", Some(&guest), "");
    assert!(listing.contains("\"state\":\"playing\""), "{listing}");
}

#[tokio::test]
async fn a_locked_room_takes_a_password_and_the_listing_never_carries_it() {
    let gw = spawn_gateway("rooms-locked");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let host = login(port, "h5@example.com", "hostfive");
    let guest = login(port, "g5@example.com", "guestfive");
    let host_deck = make_deck(port, &host, "hd");
    let guest_deck = make_deck(port, &guest, "gd");

    let create =
        format!("{{\"deck_id\":\"{host_deck}\",\"seats\":2,\"password\":\"kitchen-only\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &create);
    assert_eq!(status, 200, "{body}");
    let game_id = json_field(&body, "game_id").to_string();

    // The room says it is locked and nothing more than that.
    let (_, listing) = http(port, "GET", "/lobby/games", Some(&guest), "");
    assert!(listing.contains("\"locked\":true"), "{listing}");
    assert!(
        !listing.contains("kitchen-only"),
        "the password is on the listing: {listing}"
    );

    for (password, expected) in [
        (None, 403),
        (Some("kitchen-onl"), 403),
        (Some("kitchen-only"), 200),
    ] {
        let join = match password {
            Some(p) => format!("{{\"deck_id\":\"{guest_deck}\",\"password\":\"{p}\"}}"),
            None => format!("{{\"deck_id\":\"{guest_deck}\"}}"),
        };
        let (status, body) = http(
            port,
            "POST",
            &format!("/lobby/games/{game_id}/join"),
            Some(&guest),
            &join,
        );
        assert_eq!(status, expected, "password {password:?}: {body}");
    }
}

#[tokio::test]
async fn the_room_passes_to_whoever_has_been_there_longest() {
    let gw = spawn_gateway("rooms-host");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let host = login(port, "h6@example.com", "hostsix");
    let early = login(port, "e6@example.com", "earlysix");
    let late = login(port, "l6@example.com", "latesix");
    let decks = [
        make_deck(port, &host, "hd"),
        make_deck(port, &early, "ed"),
        make_deck(port, &late, "ld"),
    ];

    let create = format!("{{\"deck_id\":\"{}\",\"seats\":4}}", decks[0]);
    let (_, body) = http(port, "POST", "/lobby/games", Some(&host), &create);
    let game_id = json_field(&body, "game_id").to_string();

    // The one who arrives first takes the *last* chair, so seat order and
    // arrival order disagree — which is the whole point of the rule.
    let join = format!("{{\"deck_id\":\"{}\",\"seat\":3}}", decks[1]);
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/join"),
        Some(&early),
        &join,
    );
    assert_eq!(status, 200, "{body}");
    let join = format!("{{\"deck_id\":\"{}\",\"seat\":1}}", decks[2]);
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/join"),
        Some(&late),
        &join,
    );
    assert_eq!(status, 200, "{body}");

    // A player cannot take the room, and a chair nobody is in cannot be
    // handed it.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/host"),
        Some(&late),
        "{\"seat\":1}",
    );
    assert_eq!(status, 403, "{body}");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/host"),
        Some(&host),
        "{\"seat\":2}",
    );
    assert_eq!(status, 409, "an empty chair: {body}");

    // The host stands up. The room goes to the player who joined first, not
    // to the one in the next chair.
    let (status, _) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/leave"),
        Some(&host),
        "",
    );
    assert_eq!(status, 204);
    let (_, listing) = http(port, "GET", "/lobby/games", Some(&early), "");
    assert!(listing.contains(&game_id), "the room outlives its host");
    assert!(
        listing.contains("\"host\":\"earlysix\""),
        "arrival order, not seat order: {listing}"
    );

    // And the new host can hand it on deliberately.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/host"),
        Some(&early),
        "{\"seat\":1}",
    );
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"host\":\"latesix\""), "{body}");
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
async fn a_guest_leaving_frees_the_chair_and_the_last_player_out_closes_the_room() {
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
    assert_eq!(ready, 0, "and it is empty again: {listing}");
    assert!(
        listing.contains("\"player\":null"),
        "nothing of the guest is left in it: {listing}"
    );

    // The guest was the only other player, so with them gone the host's own
    // exit leaves nobody to arrange the table — and a room nobody is in is
    // closed rather than advertised.
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
async fn a_table_seats_between_two_and_eight() {
    let gw = spawn_gateway("rooms-size");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let host = login(port, "h4@example.com", "hostfour");
    let deck = make_deck(port, &host, "d");

    // The gateway's bound is the engine's: `GamePreset::validate` takes two
    // to eight seats, so a room that the gateway opened must never be one the
    // engine would then refuse to build.
    for (chairs, expected) in [(1, 400), (2, 200), (4, 200), (8, 200), (9, 400)] {
        let create = format!("{{\"deck_id\":\"{deck}\",\"seats\":{chairs}}}");
        let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &create);
        assert_eq!(status, expected, "{chairs} chairs: {body}");
    }
}
