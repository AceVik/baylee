//! The deck-builder contract, against a real gateway process.
//!
//! The builder in `baylee-client-core` decides everything locally — what the
//! pool holds, whether a deck is legal, which route a save takes. All of that
//! is only worth anything if the gateway agrees, and none of it is exercised
//! by the duel tests, which start from a deck that is already stored.
//!
//! No agent is attached: nothing here starts a game.

mod common;

use common::{http, json_field, login, spawn_gateway};

/// The pool the deck builder searches.
#[test]
fn the_card_pool_is_public_and_says_what_the_engine_does_with_each_card() {
    let gateway = spawn_gateway("pool");

    // No token: the pool is reference data about this build, the same for
    // everybody, and the sign-in screen is allowed to have shown it already.
    let (status, body) = http(gateway.port, "GET", "/pool", None, "");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"cards\":["), "{body}");
    assert!(body.contains("\"pool_hash\""), "{body}");
    assert!(
        body.contains("\"english_name\""),
        "a deck row is written with the English name: {body}"
    );
    // Every row says how far the engine gets with it, or a builder offering a
    // stub would be lying about it.
    assert!(
        body.contains("\"coverage\":\"implemented\""),
        "at least one card is fully implemented: {body}"
    );
    // The pool is the registry, so a card the client can name is in it.
    assert!(body.contains("Baleful Strix"), "{body}");

    let (status, translated) = http(gateway.port, "GET", "/pool?lang=de", None, "");
    assert_eq!(status, 200, "{translated}");
    assert!(
        translated.contains("\"lang\":\"de\""),
        "the answer says which language it is in: {translated}"
    );
    // Without a catalog there is nothing to translate *to*, and the honest
    // answer is the English row rather than a blank one.
    assert!(translated.contains("Baleful Strix"), "{translated}");
}

/// A deck's whole life: saved, listed, read back, edited, deleted.
#[test]
fn a_deck_survives_a_round_trip_with_its_sideboard() {
    let gateway = spawn_gateway("decks");
    let token = login(gateway.port, "builder@example.test", "Builder");

    let body = r#"{"name":"Strixes","cards":["4 Baleful Strix","20 Forest"],
                   "sideboard":["2 Counterspell"],"commander":null}"#;
    let (status, saved) = http(gateway.port, "POST", "/decks", Some(&token), body);
    assert_eq!(status, 200, "{saved}");
    let id = json_field(&saved, "deck_id").to_string();

    // The list carries counts, which is what the lobby's deck rows show.
    let (status, list) = http(gateway.port, "GET", "/decks", Some(&token), "");
    assert_eq!(status, 200, "{list}");
    assert!(list.contains("\"cards\":2"), "two lines, not 24: {list}");
    assert!(list.contains("\"sideboard\":1"), "{list}");

    // The deck itself comes back row for row — this is what the builder
    // re-opens, and a lost row would be silently dropped on the next save.
    let (status, one) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{one}");
    assert!(one.contains("4 Baleful Strix"), "{one}");
    assert!(one.contains("20 Forest"), "{one}");
    assert!(one.contains("2 Counterspell"), "{one}");

    // Editing overwrites in place: same id, new contents.
    let edited = r#"{"name":"Strixes, again","cards":["4 Baleful Strix"],
                     "sideboard":[],"commander":null}"#;
    let (status, answer) = http(
        gateway.port,
        "PUT",
        &format!("/decks/{id}"),
        Some(&token),
        edited,
    );
    assert_eq!(status, 204, "{answer}");
    let (_, one) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert!(one.contains("Strixes, again"), "{one}");
    assert!(
        !one.contains("Counterspell"),
        "the sideboard went with it: {one}"
    );

    let (status, answer) = http(
        gateway.port,
        "DELETE",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 204, "{answer}");
    let (status, _) = http(
        gateway.port,
        "GET",
        &format!("/decks/{id}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 404, "and it is really gone");
}

/// What the builder greys the save button for, the gateway refuses. The two
/// lists must agree or a live button would still fail.
#[test]
fn the_gateway_refuses_exactly_what_the_builder_calls_blocking() {
    let gateway = spawn_gateway("legality");
    let token = login(gateway.port, "picky@example.test", "Picky");

    for (why, body) in [
        (
            "an unknown card",
            r#"{"name":"D","cards":["1 Not A Real Card"],"sideboard":[],"commander":null}"#,
        ),
        (
            "an empty deck",
            r#"{"name":"D","cards":[],"sideboard":[],"commander":null}"#,
        ),
        (
            "a fifth copy",
            r#"{"name":"D","cards":["5 Baleful Strix"],"sideboard":[],"commander":null}"#,
        ),
        (
            "a nameless deck",
            r#"{"name":"","cards":["1 Forest"],"sideboard":[],"commander":null}"#,
        ),
        (
            "an unknown card in the sideboard",
            r#"{"name":"D","cards":["1 Forest"],"sideboard":["1 Not A Real Card"],"commander":null}"#,
        ),
    ] {
        let (status, answer) = http(gateway.port, "POST", "/decks", Some(&token), body);
        assert_eq!(status, 400, "{why}: {answer}");
    }

    // And a basic land is the one card there may be any number of.
    let body = r#"{"name":"Mono-forest","cards":["40 Forest"],"sideboard":[],"commander":null}"#;
    let (status, answer) = http(gateway.port, "POST", "/decks", Some(&token), body);
    assert_eq!(status, 200, "{answer}");
}

/// Decks belong to accounts, and the builder addresses them by id alone.
#[test]
fn another_account_cannot_read_edit_or_delete_a_deck() {
    let gateway = spawn_gateway("deck-owners");
    let mine = login(gateway.port, "mine@example.test", "Mine");
    let theirs = login(gateway.port, "theirs@example.test", "Theirs");

    let body = r#"{"name":"Mine","cards":["1 Forest"],"sideboard":[],"commander":null}"#;
    let (status, saved) = http(gateway.port, "POST", "/decks", Some(&mine), body);
    assert_eq!(status, 200, "{saved}");
    let id = json_field(&saved, "deck_id").to_string();

    for (method, payload) in [("GET", ""), ("PUT", body), ("DELETE", "")] {
        let (status, answer) = http(
            gateway.port,
            method,
            &format!("/decks/{id}"),
            Some(&theirs),
            payload,
        );
        assert_eq!(status, 403, "{method}: {answer}");
    }
    // And an unsigned request never gets that far.
    let (status, _) = http(gateway.port, "GET", &format!("/decks/{id}"), None, "");
    assert_eq!(status, 401);
}
