//! One `LobbyRequest` in, one HTTP call out, one `LobbyEvent` back: the verb and the route, the escaped query string, the bearer header a signed-in lobby adds and nobody else, and the JSON field names the gateway deserialises — then `decode` against every answer it can give, including a 204 with no body and a proxy's HTML that must fail rather than panic. Nothing here builds an `App`; `build` and `decode` are called directly, which is what makes this the cheapest place a mistyped path is caught. The starter deck's rows are checked here too, because what they are held against is what `POST /decks` accepts. What a screen then does with an event belongs to that screen's part.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn every_request_hits_the_route_the_gateway_serves() {
    let cases = [
        (
            LobbyRequest::LogIn {
                email: "a@b.c".to_string(),
                password: "pw".to_string(),
            },
            "POST",
            "http://gw/auth/login",
        ),
        (
            LobbyRequest::Register {
                email: "a@b.c".to_string(),
                display_name: "V".to_string(),
                password: "pw".to_string(),
            },
            "POST",
            "http://gw/auth/register",
        ),
        (LobbyRequest::ListDecks, "GET", "http://gw/decks"),
        (
            LobbyRequest::SaveDeck {
                deck_id: None,
                name: "d".to_string(),
                cards: vec!["1 Forest".to_string()],
                sideboard: Vec::new(),
                commanders: Vec::new(),
            },
            "POST",
            "http://gw/decks",
        ),
        (
            LobbyRequest::SaveDeck {
                deck_id: Some("d1".to_string()),
                name: "d".to_string(),
                cards: vec!["1 Forest".to_string()],
                sideboard: Vec::new(),
                commanders: Vec::new(),
            },
            "PUT",
            "http://gw/decks/d1",
        ),
        (LobbyRequest::LoadPool, "GET", "http://gw/pool?lang=en"),
        (
            LobbyRequest::LoadDeck {
                deck_id: "d1".to_string(),
            },
            "GET",
            "http://gw/decks/d1",
        ),
        (
            LobbyRequest::DeleteDeck {
                deck_id: "d1".to_string(),
            },
            "DELETE",
            "http://gw/decks/d1",
        ),
        (
            LobbyRequest::ListGames(GameQuery {
                q: "a room".to_string(),
                offset: 8,
                limit: 8,
            }),
            "GET",
            "http://gw/lobby/games?q=a%20room&offset=8&limit=8",
        ),
        (
            LobbyRequest::CreateGame {
                deck_id: "d1".to_string(),
                mode: GameMode::Ai,
                chairs: 2,
                name: String::new(),
                password: String::new(),
            },
            "POST",
            "http://gw/lobby/games",
        ),
        (
            LobbyRequest::JoinGame {
                game_id: "g1".to_string(),
                deck_id: "d1".to_string(),
                seat: None,
                password: String::new(),
            },
            "POST",
            "http://gw/lobby/games/g1/join",
        ),
        (
            LobbyRequest::Rematch {
                game_id: "g1".to_string(),
            },
            "POST",
            "http://gw/lobby/games/g1/rematch",
        ),
    ];
    for (request, method, url) in cases {
        let (built, _) = build("http://gw", None, "en", request.clone());
        assert_eq!(built.method.as_str(), method, "{request:?}");
        assert_eq!(built.url, url, "{request:?}");
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one assertion per body the gateway reads
fn the_bodies_carry_the_field_names_the_gateway_deserialises() {
    let (login, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::LogIn {
            email: "a@b.c".to_string(),
            password: "pw".to_string(),
        },
    );
    assert_eq!(
        body(&login),
        serde_json::json!({ "email": "a@b.c", "password": "pw" })
    );
    let (register, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::Register {
            email: "a@b.c".to_string(),
            display_name: "V".to_string(),
            password: "pw".to_string(),
        },
    );
    assert_eq!(
        body(&register),
        serde_json::json!({
            "email": "a@b.c",
            "display_name": "V",
            "password": "pw",
            "lang": "en"
        })
    );
    let (deck, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::SaveDeck {
            deck_id: None,
            name: "Starter".to_string(),
            cards: vec!["1 Forest".to_string()],
            sideboard: vec!["2 Naturalize".to_string()],
            commanders: Vec::new(),
        },
    );
    assert_eq!(
        body(&deck),
        serde_json::json!({
            "name": "Starter",
            "cards": ["1 Forest"],
            "sideboard": ["2 Naturalize"],
            "commanders": []
        })
    );
    let (game, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::CreateGame {
            deck_id: "d1".to_string(),
            mode: GameMode::Open,
            chairs: 2,
            name: String::new(),
            password: String::new(),
        },
    );
    assert_eq!(
        body(&game),
        serde_json::json!({ "deck_id": "d1", "mode": "open", "seats": 2, "name": "", "password": "" })
    );
    let (join, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::JoinGame {
            game_id: "g1".to_string(),
            deck_id: "d1".to_string(),
            seat: None,
            password: String::new(),
        },
    );
    assert_eq!(
        body(&join),
        serde_json::json!({ "deck_id": "d1", "seat": null, "password": "" })
    );

    // Arranging a chair, which is the room's own verb.
    let (chair, expect) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::SetSeat {
            game_id: "g1".to_string(),
            seat: 2,
            kind: Some(SeatKind::Ai),
            ai: Some("sharp".to_string()),
            deck_id: None,
            team: Some(2),
        },
    );
    assert_eq!(chair.url, "http://gw/lobby/games/g1/seats/2");
    assert_eq!(
        body(&chair),
        serde_json::json!({ "kind": "ai", "ai": "sharp", "deck_id": null, "team": 2 })
    );
    assert!(
        matches!(expect, Expect::Moved),
        "the room moved, so the page being read is asked for again"
    );

    // Playing again answers a ticket, exactly as a join does. Reading it as
    // anything else would leave the player on the veil with a seat granted.
    let (again, expect) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::Rematch {
            game_id: "g1".to_string(),
        },
    );
    assert_eq!(body(&again), serde_json::json!({}));
    assert!(matches!(expect, Expect::Seat));
}

#[test]
fn a_trailing_slash_on_the_gateway_does_not_double_up() {
    // `gateway_url()` trims one, but a hand-set `.env` is not the only way
    // in and a `//decks` is a 404 with no explanation.
    let (built, _) = build("http://gw/", None, "en", LobbyRequest::ListDecks);
    assert!(!built.url.contains("//decks"), "{}", built.url);
}

/// A search is a person's typing, and a person types `&`, `#` and spaces.
/// Any of them straight into a URL is a query the gateway reads as something
/// else — or, with a token, as somebody else's parameters.
#[test]
fn a_typed_search_survives_the_query_string() {
    let query = GameQuery {
        q: "tom & jerry #2".to_string(),
        offset: 16,
        limit: 8,
    };
    assert_eq!(
        super::http::params(&query),
        "q=tom%20%26%20jerry%20%232&offset=16&limit=8"
    );
    // The socket and the button ask the same question, in the same words:
    // the feed builds its URL out of this too.
    let (built, _) = build("http://gw", None, "en", LobbyRequest::ListGames(query));
    assert!(
        built
            .url
            .ends_with("q=tom%20%26%20jerry%20%232&offset=16&limit=8"),
        "{}",
        built.url
    );
}

#[test]
fn only_a_signed_in_lobby_sends_a_token() {
    let (anonymous, _) = build("http://gw", None, "en", LobbyRequest::ListDecks);
    assert_eq!(anonymous.headers.get("Authorization"), None);
    let (signed, _) = build("http://gw", Some("tok"), "en", LobbyRequest::ListDecks);
    assert_eq!(signed.headers.get("Authorization"), Some("Bearer tok"));
}

#[test]
fn a_json_body_says_so() {
    let (built, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::ListGames(GameQuery::default()),
    );
    assert!(built.body.is_empty(), "a GET carries none");
    let (built, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::SaveDeck {
            deck_id: None,
            name: "d".to_string(),
            cards: vec!["1 Forest".to_string()],
            sideboard: Vec::new(),
            commanders: Vec::new(),
        },
    );
    assert_eq!(built.headers.get("Content-Type"), Some("application/json"));
}

#[test]
fn the_gateways_own_answers_decode() {
    assert_eq!(
        decode(
            Lang::En,
            Expect::LoggedIn,
            &answer(200, r#"{"token":"tok","expires_at":123}"#)
        ),
        LobbyEvent::LoggedIn {
            token: "tok".to_string()
        }
    );
    assert_eq!(
        decode(
            Lang::En,
            Expect::Decks,
            &answer(
                200,
                r#"[{"id":"d1","name":"Allytifact","cards":96,"commanders":[]}]"#
            )
        ),
        LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 96,
            sideboard: 0,
            commanders: Vec::new(),
        }])
    );
    assert_eq!(
        decode(
            Lang::En,
            Expect::Seat,
            &answer(200, r#"{"game_id":"g1","seat":1,"seat_token":"st"}"#)
        ),
        LobbyEvent::Seated(SeatHandover {
            game_id: "g1".to_string(),
            seat: 1,
            seat_token: "st".to_string(),
            local: false,
        })
    );
    assert_eq!(
        decode(Lang::En, Expect::Registered, &answer(200, r#"{"ok":true}"#)),
        LobbyEvent::Registered {
            confirmation_required: false,
        }
    );
    assert_eq!(
        decode(
            Lang::En,
            Expect::DeckSaved,
            &answer(200, r#"{"deck_id":"d1"}"#)
        ),
        LobbyEvent::DeckSaved {
            deck_id: Some("d1".to_string())
        }
    );
}

#[test]
fn a_body_that_makes_no_sense_is_a_failure_not_a_panic() {
    assert!(matches!(
        decode(
            Lang::En,
            Expect::LoggedIn,
            &answer(200, "<html>proxy</html>")
        ),
        LobbyEvent::Failed(_)
    ));
}

#[test]
fn a_refusal_is_shown_in_the_gateways_own_words() {
    assert_eq!(
        gateway_error(Lang::En, &answer(401, r#"{"error":"invalid credentials"}"#)),
        "invalid credentials"
    );
    assert_eq!(
        gateway_error(Lang::En, &answer(502, "<html>bad gateway</html>")),
        "the gateway answered 502"
    );
}

#[test]
fn an_edit_answers_with_no_body_and_that_is_not_a_failure() {
    // `PUT /decks/{id}` is a 204. Reading an id out of nothing is not an
    // error here — the builder already holds the one it is editing.
    assert_eq!(
        decode(Lang::En, Expect::DeckSaved, &answer(204, "")),
        LobbyEvent::DeckSaved { deck_id: None }
    );
}

#[test]
fn the_pool_and_a_saved_deck_decode() {
    let cards = serde_json::to_string(&serde_json::json!({
        "total": 2,
        "pool_hash": "abc",
        "lang": "en",
        "has_text": true,
        "cards": []
    }))
    .expect("a body");
    assert_eq!(
        decode(Lang::En, Expect::Pool, &answer(200, &cards)),
        LobbyEvent::Pool {
            cards: Vec::new(),
            has_text: true
        }
    );
    assert_eq!(
        decode(
            Lang::En,
            Expect::DeckLoaded,
            &answer(
                200,
                r#"{"id":"d1","name":"Elves","cards":["4 Llanowar Elves"],
                   "sideboard":["1 Forest"],"commanders":[]}"#
            )
        ),
        LobbyEvent::DeckLoaded {
            id: "d1".to_string(),
            name: "Elves".to_string(),
            cards: vec!["4 Llanowar Elves".to_string()],
            sideboard: vec!["1 Forest".to_string()],
            commanders: Vec::new(),
        }
    );
    assert_eq!(
        decode(Lang::En, Expect::DeckDeleted, &answer(204, "")),
        LobbyEvent::DeckDeleted
    );
}

#[test]
fn the_starter_deck_is_one_the_gateway_will_accept() {
    let rows = starter_rows();
    assert!(
        !rows.is_empty(),
        "the acceptance file has an {STARTER} deck"
    );
    assert!(rows.len() <= 250, "the gateway caps the list at 250 rows");
    for row in &rows {
        let (count, name) = row.split_once(' ').expect("\"N Card Name\"");
        let count: u32 = count.parse().expect("a leading count");
        assert!((1..=4).contains(&count), "{row}");
        // The gateway resolves every name against the same registry, and
        // answers a miss with a 400 — "unknown card" for a name that is no
        // card, and "that card exists but this server cannot play it" for a
        // real one this build compiles nothing for. A starter deck must miss
        // neither way.
        assert!(
            baylee_cards::decks::by_name(name).is_some(),
            "{name} is not in the registry"
        );
    }
}

#[test]
fn issue_186_library_routes_and_snapshot_payloads_match_the_gateway() {
    use client_core::lobby::library::{Reply, Request};
    for (request, method, path) in [
        (Request::House, "GET", "http://gw/decks/shared"),
        (
            Request::History("d1".into()),
            "GET",
            "http://gw/decks/d1/history",
        ),
        (
            Request::Version("d1".into(), 2),
            "GET",
            "http://gw/decks/d1/versions/2",
        ),
        (
            Request::Copy("house".into()),
            "POST",
            "http://gw/decks/house/copy",
        ),
        (
            Request::Restore("d1".into(), 2),
            "POST",
            "http://gw/decks/d1/versions/2/revert",
        ),
    ] {
        let (http, _) = super::super::http::build(
            "http://gw",
            Some("tok"),
            "en",
            LobbyRequest::Library(request),
        );
        assert_eq!(http.method.as_str(), method);
        assert_eq!(http.url, path);
    }
    let reply = super::super::http::decode_library(
        Request::Version("d1".into(), 2),
        r#"{"version":2,"cards":["4 Forest"],"sideboard":["2 Island"],"commanders":[],"current":false}"#,
        Lang::En,
    );
    let LobbyEvent::Library(Reply::Version(id, snapshot)) = reply else {
        panic!("snapshot was not decoded")
    };
    assert_eq!(id, "d1");
    assert_eq!(snapshot.sideboard, ["2 Island"]);
    assert!(matches!(
        super::super::http::decode_library(Request::House, "<html>proxy failure</html>", Lang::En),
        LobbyEvent::Library(Reply::Failed(_))
    ));
}
