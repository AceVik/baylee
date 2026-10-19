//! End-to-end tests for blocked cards (`docs/protocol.md` §"Blocked
//! cards"): an admin takes a card of the pool out of play on this gateway,
//! for good or until a moment, and a deck holding it is refused where it is
//! chosen, where its player says ready and where a game is built, the house
//! deck included.
//!
//! What they hold: the console's search, block, list and lift; the answers
//! to a bad request; an audit line for each change that never holds the
//! reason; the player routes (`GET /cards/blocked`, the `blocked` list on
//! `GET /decks` and `GET /decks/shared`); a block with an end stops counting
//! by itself; and every place a deck is chosen or a game built says `409`
//! naming the deck, the card and the reason, and lets the same deck through
//! again once the block is lifted.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{Gateway, attach_agent, http, json_field, login, spawn_gateway_with};
use serde_json::{Value, json};

/// The token every console here is started with.
const TOKEN: &str = "e2e-admin-token-0123456789abcdef0123456789";

/// The admin the feedback service would name.
const ADMIN: &str = "viktor";

/// A reason nobody would write by accident: the audit must never carry it.
const REASON: &str = "plumbus-4711 misprints the stack";

/// A gateway and its console's port.
struct Console {
    gw: Gateway,
    port: u16,
}

/// A gateway with a console.
fn console(label: &str) -> Console {
    let port_file = std::env::temp_dir().join(format!(
        "baylee-blocks-{label}-{}-{}.port",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&port_file);
    let env = vec![
        ("BAYLEE_ADMIN_TOKEN", TOKEN.to_owned()),
        ("BAYLEE_ADMIN_BIND", "127.0.0.1:0".to_owned()),
        (
            "BAYLEE_ADMIN_PORT_FILE",
            port_file.to_string_lossy().into_owned(),
        ),
        ("RUST_LOG", "baylee_gateway::audit=info,warn".to_owned()),
    ];
    let gw = spawn_gateway_with(label, &env);
    let port = std::fs::read_to_string(&port_file)
        .expect("the console's port file")
        .trim()
        .parse()
        .expect("a port");
    let _ = std::fs::remove_file(&port_file);
    Console { gw, port }
}

/// One request to the console: status, lower-cased head, body.
fn ask(
    port: u16,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (u16, String, String) {
    use std::fmt::Write as _;
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut head = format!("{method} {path} HTTP/1.1\r\n");
    let _ = write!(head, "Host: 127.0.0.1:{port}\r\n");
    for (name, value) in headers {
        let _ = write!(head, "{name}: {value}\r\n");
    }
    let request = format!(
        "{head}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).expect("write");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read");
    let status = raw
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("a status");
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
    (status, head.to_lowercase(), body.to_owned())
}

const BEARER: &str = "Bearer e2e-admin-token-0123456789abcdef0123456789";

/// A console request as the feedback service sends it.
fn admin(c: &Console, method: &str, path: &str, body: &str) -> (u16, String) {
    let (status, _, body) = ask(
        c.port,
        method,
        path,
        &[("Authorization", BEARER), ("X-Baylee-Admin", ADMIN)],
        body,
    );
    (status, body)
}

/// A console request, answered as JSON.
fn admin_json(c: &Console, method: &str, path: &str, body: &Value) -> (u16, Value) {
    let body = if body.is_null() {
        String::new()
    } else {
        body.to_string()
    };
    let (status, text) = admin(c, method, path, &body);
    let value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (status, value)
}

/// A log without its colours (`ESC [ … m`).
fn plain(logs: &str) -> String {
    let mut out = String::with_capacity(logs.len());
    let mut chars = logs.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The index of a card of the pool, by its English name.
fn card(name: &str) -> u32 {
    let found = baylee_cards::pool::rows()
        .iter()
        .find(|c| c.english_name == name)
        .unwrap_or_else(|| panic!("the pool has no {name}"));
    assert_eq!(
        baylee_cards::decks::by_name(name).map(baylee_core::ids::CardIndex::get),
        Some(found.index),
        "a deck row names {name} by the index the console blocks"
    );
    found.index
}

/// An RFC 3339 moment `secs` from now, in whole seconds, in UTC.
fn in_secs(secs: i64) -> String {
    let at = time::OffsetDateTime::now_utc() + time::Duration::seconds(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    )
}

/// Blocks a card; asserts the console took it and returns the listed row.
fn block(c: &Console, index: u32, reason: &str, until: Option<&str>) -> Value {
    let mut body = json!({ "index": index, "reason": reason });
    if let Some(until) = until {
        body["until"] = json!(until);
    }
    let (status, row) = admin_json(c, "POST", "/admin/cards/blocks", &body);
    assert_eq!(status, 201, "block {index}: {row}");
    row
}

/// Lifts a block; asserts the console took it.
fn lift(c: &Console, id: &str) {
    let (status, body) = admin(c, "DELETE", &format!("/admin/cards/blocks/{id}"), "");
    assert_eq!(status, 204, "lift {id}: {body}");
}

/// Saves a deck and returns its id.
fn save_deck(port: u16, token: &str, deck: &Value) -> String {
    let (status, body) = http(port, "POST", "/decks", Some(token), &deck.to_string());
    assert_eq!(status, 200, "save deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// A sixty-card deck holding `card` four times.
fn deck_with(name: &str, card: &str) -> Value {
    json!({ "name": name, "cards": [format!("4 {card}"), "36 Forest", "20 Swamp"] })
}

/// A sixty-card deck of basic lands.
fn lands(name: &str) -> Value {
    json!({ "name": name, "cards": ["40 Forest", "20 Swamp"] })
}

/// `POST` as a player, answered as JSON.
fn post(port: u16, token: &str, path: &str, body: &Value) -> (u16, Value) {
    let (status, text) = http(port, "POST", path, Some(token), &body.to_string());
    let value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (status, value)
}

/// `GET` as a player, answered as JSON.
fn get(port: u16, token: &str, path: &str) -> (u16, Value) {
    let (status, text) = http(port, "GET", path, Some(token), "");
    let value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (status, value)
}

/// The refusal a player reads: its `error` sentence.
fn error_of(answer: &Value) -> &str {
    answer["error"]
        .as_str()
        .unwrap_or_else(|| panic!("an error sentence in {answer}"))
}

/// Asserts a `409` that names the deck, the card and the reason.
fn assert_refused(answer: &(u16, Value), deck: &str, card: &str, reason: &str) {
    let (status, body) = answer;
    assert_eq!(*status, 409, "{body}");
    let said = error_of(body);
    assert!(
        said.starts_with(&format!("the deck \u{201c}{deck}\u{201d} holds {card}, ")),
        "names the deck and the card: {said}"
    );
    assert!(said.contains("blocked on this gateway"), "{said}");
    assert!(said.ends_with(&format!(": {reason}")), "{said}");
}

/// Opens a room with `deck`, for two chairs, and returns its id.
fn open_room(port: u16, host: &str, deck: &str) -> String {
    let (status, body) = post(
        port,
        host,
        "/lobby/games",
        &json!({ "deck_id": deck, "seats": 2 }),
    );
    assert_eq!(status, 200, "open a room: {body}");
    body["game_id"].as_str().expect("a game id").to_owned()
}

/// The blocked cards a deck row of `GET /decks` lists, by deck name.
fn blocked_on(port: u16, token: &str, path: &str, deck: &str) -> Value {
    let (status, list) = get(port, token, path);
    assert_eq!(status, 200, "{list}");
    let row = list
        .as_array()
        .expect("a list")
        .iter()
        .find(|d| d["name"] == deck)
        .unwrap_or_else(|| panic!("no deck {deck} in {list}"));
    row["blocked"].clone()
}

#[test]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
fn the_console_searches_blocks_lists_and_lifts() {
    let c = console("cb_console");
    let strix = card("Baleful Strix");

    // Search: the shape of a row, any case, by index, prefix matches first.
    let (status, found) = admin_json(
        &c,
        "GET",
        "/admin/cards/search?q=baleful%20strix",
        &Value::Null,
    );
    assert_eq!(status, 200, "{found}");
    let row = &found["cards"][0];
    assert_eq!(row["name"], "Baleful Strix", "{found}");
    assert_eq!(row["index"], strix);
    for field in ["type_line", "mana_cost", "coverage", "scryfall_id"] {
        assert!(row.get(field).is_some(), "{field} in {row}");
    }
    assert!(row["blocked"].is_null(), "not blocked yet: {row}");
    let (_, upper) = admin_json(
        &c,
        "GET",
        // The full corpus holds other "Baleful …" cards; name it whole.
        "/admin/cards/search?q=BALEFUL%20STRIX",
        &Value::Null,
    );
    assert_eq!(upper["cards"][0]["index"], strix, "case does not matter");
    let (_, by_index) = admin_json(
        &c,
        "GET",
        &format!("/admin/cards/search?q={strix}"),
        &Value::Null,
    );
    assert!(
        by_index["cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["index"] == strix),
        "an index finds its card: {by_index}"
    );
    let (_, forest) = admin_json(&c, "GET", "/admin/cards/search?q=forest", &Value::Null);
    let names: Vec<String> = forest["cards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_lowercase())
        .collect();
    assert!(names.len() > 1, "{names:?}");
    let prefixed: Vec<bool> = names.iter().map(|n| n.starts_with("forest")).collect();
    assert!(prefixed[0], "{names:?}");
    assert!(
        prefixed.windows(2).all(|w| w[0] || !w[1]),
        "names starting with the query come first: {names:?}"
    );
    let (_, many) = admin_json(&c, "GET", "/admin/cards/search?q=a", &Value::Null);
    assert_eq!(many["cards"].as_array().unwrap().len(), 25, "at most 25");
    for q in ["", "%20%20"] {
        let (status, none) = admin_json(
            &c,
            "GET",
            &format!("/admin/cards/search?q={q}"),
            &Value::Null,
        );
        assert_eq!((status, none), (200, json!({ "cards": [] })), "q={q:?}");
    }
    let (status, none) = admin_json(&c, "GET", "/admin/cards/search", &Value::Null);
    assert_eq!((status, none), (200, json!({ "cards": [] })), "no q at all");
    let (status, too_long) = admin(
        &c,
        "GET",
        &format!("/admin/cards/search?q={}", "x".repeat(101)),
        "",
    );
    assert_eq!(status, 400, "{too_long}");

    // Nothing is blocked yet.
    let (status, blocks) = admin_json(&c, "GET", "/admin/cards/blocks", &Value::Null);
    assert_eq!((status, blocks), (200, json!([])));

    // Block it; the reason is trimmed.
    let made = block(&c, strix, &format!("  {REASON}  "), None);
    let id = made["id"].as_str().expect("an id").to_owned();
    assert_eq!(made["index"], strix);
    assert_eq!(made["name"], "Baleful Strix");
    assert_eq!(made["reason"], REASON, "trimmed");
    assert_eq!(made["created_by"], ADMIN);
    assert_eq!(made["state"], "active");
    assert!(made["until"].is_null() && made["lifted_at"].is_null());
    assert!(made["lifted_by"].is_null());
    assert!(
        made["created_at"].as_str().unwrap().ends_with('Z'),
        "{made}"
    );

    // Listed, newest first; the card is flagged in a search.
    let (_, blocks) = admin_json(&c, "GET", "/admin/cards/blocks", &Value::Null);
    assert_eq!(blocks, json!([made.clone()]));
    let (_, found) = admin_json(
        &c,
        "GET",
        "/admin/cards/search?q=baleful%20strix",
        &Value::Null,
    );
    assert_eq!(
        found["cards"][0]["blocked"],
        json!({ "index": strix, "name": "Baleful Strix", "reason": REASON, "until": null }),
    );

    // A card with a block in force is not blocked twice, not even with an end.
    for body in [
        json!({ "index": strix, "reason": "again" }),
        json!({ "index": strix, "reason": "again", "until": in_secs(3600) }),
    ] {
        let (status, refused) = admin_json(&c, "POST", "/admin/cards/blocks", &body);
        assert_eq!(status, 409, "{refused}");
    }

    // Lift it; once only.
    lift(&c, &id);
    let (status, again) = admin(&c, "DELETE", &format!("/admin/cards/blocks/{id}"), "");
    assert_eq!(status, 404, "already lifted: {again}");
    let (_, found) = admin_json(
        &c,
        "GET",
        "/admin/cards/search?q=baleful%20strix",
        &Value::Null,
    );
    assert!(found["cards"][0]["blocked"].is_null(), "{found}");
    let (_, blocks) = admin_json(&c, "GET", "/admin/cards/blocks", &Value::Null);
    let row = &blocks[0];
    assert_eq!(row["id"], id.as_str());
    assert_eq!(row["state"], "lifted", "the row stays: {row}");
    assert_eq!(row["lifted_by"], ADMIN);
    assert!(row["lifted_at"].as_str().is_some(), "{row}");

    // After a lift the card can be blocked anew, and the list is newest first.
    let second = block(&c, strix, "second thoughts", Some(&in_secs(3600)));
    assert_ne!(second["id"], made["id"]);
    assert_eq!(second["state"], "active");
    assert!(second["until"].as_str().is_some(), "{second}");
    let (_, blocks) = admin_json(&c, "GET", "/admin/cards/blocks", &Value::Null);
    let ids: Vec<&Value> = blocks
        .as_array()
        .unwrap()
        .iter()
        .map(|b| &b["id"])
        .collect();
    assert_eq!(ids, vec![&second["id"], &made["id"]], "newest first");

    // The audit: who, what, which card; never the reason.
    let logs = plain(&c.gw.logs());
    let made_line = logs
        .lines()
        .find(|l| l.contains("card.block"))
        .unwrap_or_else(|| panic!("no audit line for the block:\n{logs}"));
    assert!(made_line.contains("baylee_gateway::audit"), "{made_line}");
    assert!(made_line.contains("admin=viktor"), "{made_line}");
    assert!(made_line.contains(&id), "{made_line}");
    assert!(made_line.contains("Baleful Strix"), "{made_line}");
    let lifted_line = logs
        .lines()
        .find(|l| l.contains("card.unblock"))
        .unwrap_or_else(|| panic!("no audit line for the lift:\n{logs}"));
    assert!(lifted_line.contains("admin=viktor"), "{lifted_line}");
    assert!(lifted_line.contains(&id), "{lifted_line}");
    assert!(lifted_line.contains("Baleful Strix"), "{lifted_line}");
    assert!(
        !logs.contains("plumbus") && !logs.contains("second thoughts"),
        "a reason never reaches the log:\n{logs}"
    );
}

#[test]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
fn the_console_refuses_what_it_cannot_read() {
    let c = console("cb_bad");
    let strix = card("Baleful Strix");
    let blocks = |body: Value| admin_json(&c, "POST", "/admin/cards/blocks", &body).0;

    assert_eq!(
        blocks(json!({ "index": strix, "reason": "r", "extra": 1 })),
        400,
        "an unknown field"
    );
    assert_eq!(blocks(json!({ "reason": "r" })), 400, "no index");
    assert_eq!(blocks(json!({ "index": strix })), 400, "no reason");
    assert_eq!(blocks(json!({ "index": "x", "reason": "r" })), 400);
    assert_eq!(blocks(json!({ "index": strix, "reason": "" })), 400);
    assert_eq!(blocks(json!({ "index": strix, "reason": "   " })), 400);
    assert_eq!(
        blocks(json!({ "index": strix, "reason": "x".repeat(201) })),
        400,
        "over 200 characters"
    );
    assert_eq!(
        blocks(json!({ "index": strix, "reason": "two\nlines" })),
        400,
        "a control character"
    );
    for until in [
        json!("2000-01-01T00:00:00Z"),
        json!("next tuesday"),
        json!("2099-13-45T00:00:00Z"),
        json!(in_secs(-60)),
        json!(7),
    ] {
        assert_eq!(
            blocks(json!({ "index": strix, "reason": "r", "until": until })),
            400,
            "until {until}"
        );
    }
    // More than ten years ahead.
    let far = time::OffsetDateTime::now_utc() + time::Duration::days(3700);
    let far = format!(
        "{:04}-{:02}-{:02}T00:00:00Z",
        far.year(),
        u8::from(far.month()),
        far.day()
    );
    assert_eq!(
        blocks(json!({ "index": strix, "reason": "r", "until": far })),
        400,
        "{far}"
    );
    assert_eq!(
        blocks(json!({ "index": u32::MAX, "reason": "r" })),
        404,
        "an index the pool has not"
    );
    let (status, body) = admin(&c, "POST", "/admin/cards/blocks", "not json");
    assert_eq!(status, 400, "{body}");

    // A null `until` and an empty one are blocks without an end.
    let a = block(&c, strix, "r", None);
    assert!(a["until"].is_null());
    lift(&c, a["id"].as_str().unwrap());
    let (status, row) = admin_json(
        &c,
        "POST",
        "/admin/cards/blocks",
        &json!({ "index": strix, "reason": "r", "until": null }),
    );
    assert_eq!((status, row["until"].clone()), (201, Value::Null), "{row}");

    // Lifting: not a UUID is 400, an unknown one 404.
    let (status, body) = admin(&c, "DELETE", "/admin/cards/blocks/not-a-uuid", "");
    assert_eq!(status, 400, "{body}");
    let (status, body) = admin(
        &c,
        "DELETE",
        "/admin/cards/blocks/00000000-0000-4000-8000-000000000000",
        "",
    );
    assert_eq!(status, 404, "{body}");

    // A change wants X-Baylee-Admin, as every change on the console does.
    let (status, _, _) = ask(
        c.port,
        "POST",
        "/admin/cards/blocks",
        &[("Authorization", BEARER)],
        &json!({ "index": card("Counterspell"), "reason": "r" }).to_string(),
    );
    assert_eq!(status, 400, "no X-Baylee-Admin");
    let (_, _, listed) = ask(
        c.port,
        "GET",
        "/admin/cards/blocks",
        &[("Authorization", BEARER)],
        "",
    );
    assert!(
        !listed.contains("Counterspell"),
        "nothing was made: {listed}"
    );
    let (status, _, _) = ask(
        c.port,
        "DELETE",
        "/admin/cards/blocks/00000000-0000-4000-8000-000000000000",
        &[("Authorization", BEARER)],
        "",
    );
    assert_eq!(status, 400, "lifting wants it too");

    // And the token, as everywhere on the console.
    let (status, _, _) = ask(c.port, "GET", "/admin/cards/blocks", &[], "");
    assert_ne!(status, 200, "no token");
    let (status, _, _) = ask(c.port, "GET", "/admin/cards/search?q=forest", &[], "");
    assert_ne!(status, 200, "no token");
    // The public port has no console.
    let (status, _, _) = ask(
        c.gw.port,
        "GET",
        "/admin/cards/blocks",
        &[("Authorization", BEARER)],
        "",
    );
    assert_eq!(status, 404);
}

#[test]
fn players_are_told_which_cards_are_blocked_and_which_of_their_decks_hold_one() {
    let c = console("cb_players");
    let port = c.gw.port;
    let rift = card("Cyclonic Rift");
    let strix = card("Baleful Strix");
    let token = login(port, "tellme", "tellme");
    save_deck(port, &token, &deck_with("Strixes", "Baleful Strix"));
    save_deck(port, &token, &lands("Lands"));

    let (status, body) = http(port, "GET", "/cards/blocked", None, "");
    assert_eq!(status, 401, "signed-in only: {body}");
    let (status, body) = http(port, "GET", "/cards/blocked", Some("nope"), "");
    assert_eq!(status, 401, "{body}");

    let (status, none) = get(port, &token, "/cards/blocked");
    assert_eq!((status, none), (200, json!({ "blocked": [] })));
    assert_eq!(blocked_on(port, &token, "/decks", "Strixes"), json!([]));
    assert_eq!(
        blocked_on(port, &token, "/decks/shared", "Victory"),
        json!([])
    );

    // Two blocks, one with an end.
    block(&c, strix, REASON, None);
    let until = in_secs(3600);
    block(&c, rift, "rift reason", Some(&until));

    let (status, told) = get(port, &token, "/cards/blocked");
    assert_eq!(status, 200, "{told}");
    let mut told: Vec<Value> = told["blocked"].as_array().unwrap().clone();
    told.sort_by_key(|b| b["index"].as_u64());
    let mut want = vec![
        json!({ "index": strix, "name": "Baleful Strix", "reason": REASON, "until": null }),
        json!({ "index": rift, "name": "Cyclonic Rift", "reason": "rift reason", "until": until }),
    ];
    want.sort_by_key(|b| b["index"].as_u64());
    assert_eq!(told, want);

    // The deck that holds it says so; the other says nothing.
    assert_eq!(
        blocked_on(port, &token, "/decks", "Strixes"),
        json!([{ "index": strix, "name": "Baleful Strix", "reason": REASON, "until": null }]),
        "once, though the deck holds four"
    );
    assert_eq!(blocked_on(port, &token, "/decks", "Lands"), json!([]));
    // The house's own deck is marked the same way.
    let shared = blocked_on(port, &token, "/decks/shared", "Victory");
    let shared = shared.as_array().unwrap();
    assert!(
        shared
            .iter()
            .any(|b| b["index"] == rift && b["until"] == until.as_str()),
        "Victory holds Cyclonic Rift: {shared:?}"
    );

    // Lifting tells them so.
    let (_, blocks) = admin_json(&c, "GET", "/admin/cards/blocks", &Value::Null);
    for b in blocks.as_array().unwrap() {
        lift(&c, b["id"].as_str().unwrap());
    }
    let (_, none) = get(port, &token, "/cards/blocked");
    assert_eq!(none, json!({ "blocked": [] }));
    assert_eq!(blocked_on(port, &token, "/decks", "Strixes"), json!([]));
    assert_eq!(
        blocked_on(port, &token, "/decks/shared", "Victory"),
        json!([])
    );
}

#[test]
fn a_block_with_an_end_stops_counting_by_itself() {
    let c = console("cb_until");
    let port = c.gw.port;
    let token = login(port, "waiter", "waiter");
    let deck = save_deck(port, &token, &deck_with("Strixes", "Baleful Strix"));

    let row = block(&c, card("Baleful Strix"), REASON, Some(&in_secs(6)));
    let until = row["until"].as_str().expect("an end").to_owned();
    assert!(until.ends_with('Z'), "{until}");

    // While it lasts the deck is refused, and the refusal names the end.
    let (status, told) = get(port, &token, "/cards/blocked");
    assert_eq!(status, 200);
    assert_eq!(told["blocked"].as_array().unwrap().len(), 1, "{told}");
    let refused = post(
        port,
        &token,
        "/lobby/games",
        &json!({ "deck_id": deck, "seats": 2 }),
    );
    assert_eq!(refused.0, 409, "{}", refused.1);
    let said = error_of(&refused.1);
    let readable = format!("{} {}:{} UTC", &until[..10], &until[11..13], &until[14..16]);
    assert!(
        said.contains(&format!(" until {readable}: ")),
        "{said} / {until}"
    );

    // Then, with nobody lifting anything, it is over.
    std::thread::sleep(std::time::Duration::from_secs(7));
    let (_, told) = get(port, &token, "/cards/blocked");
    assert_eq!(told, json!({ "blocked": [] }), "the end passed");
    assert_eq!(blocked_on(port, &token, "/decks", "Strixes"), json!([]));
    let (status, body) = post(
        port,
        &token,
        "/lobby/games",
        &json!({ "deck_id": deck, "seats": 2 }),
    );
    assert_eq!(status, 200, "playable again: {body}");
    let (_, blocks) = admin_json(&c, "GET", "/admin/cards/blocks", &Value::Null);
    assert_eq!(blocks[0]["state"], "expired", "{blocks}");
    let (_, found) = admin_json(
        &c,
        "GET",
        "/admin/cards/search?q=baleful%20strix",
        &Value::Null,
    );
    assert!(found["cards"][0]["blocked"].is_null(), "{found}");
    // An ended block is not lifted: there is nothing to lift, and the card
    // can be blocked again.
    let id = row["id"].as_str().unwrap();
    let (status, body) = admin(&c, "DELETE", &format!("/admin/cards/blocks/{id}"), "");
    assert_eq!(status, 404, "{body}");
    block(&c, card("Baleful Strix"), "again", None);
}

#[test]
fn a_deck_holding_a_blocked_card_is_refused_where_it_is_chosen() {
    let c = console("cb_choose");
    let port = c.gw.port;
    let host = login(port, "chooser", "chooser");
    let guest = login(port, "guestchooser", "guestchooser");
    let clean = save_deck(port, &host, &lands("Clean"));
    let host_bad = save_deck(port, &host, &deck_with("Strixes", "Baleful Strix"));
    let guest_bad = save_deck(port, &guest, &deck_with("Guest Strixes", "Baleful Strix"));
    let guest_clean = save_deck(port, &guest, &lands("Guest Clean"));

    // A room to join, and one to arrange, made before the block.
    let room = open_room(port, &host, &clean);
    let id = block(&c, card("Baleful Strix"), REASON, None)["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Creating a room with it.
    let made = post(
        port,
        &host,
        "/lobby/games",
        &json!({ "deck_id": host_bad, "seats": 2 }),
    );
    assert_refused(&made, "Strixes", "Baleful Strix", REASON);
    assert!(
        !error_of(&made.1).contains(" until "),
        "no end, none said: {}",
        error_of(&made.1)
    );
    // Nothing was made by the refusal.
    let (_, listing) = get(port, &guest, "/lobby/games");
    assert_eq!(listing["games"].as_array().unwrap().len(), 1, "{listing}");

    // Joining with it, taking a chair with it, choosing it for one's own chair.
    let joined = post(
        port,
        &guest,
        &format!("/lobby/games/{room}/join"),
        &json!({ "deck_id": guest_bad }),
    );
    assert_refused(&joined, "Guest Strixes", "Baleful Strix", REASON);
    let seated = post(
        port,
        &host,
        &format!("/lobby/games/{room}/seats/0"),
        &json!({ "deck_id": host_bad }),
    );
    assert_refused(&seated, "Strixes", "Baleful Strix", REASON);

    // A deck without it is not in the way.
    let (status, body) = post(
        port,
        &guest,
        &format!("/lobby/games/{room}/join"),
        &json!({ "deck_id": guest_clean }),
    );
    assert_eq!(status, 200, "{body}");

    // Lifted, the same deck goes through every door.
    lift(&c, &id);
    let (status, body) = post(
        port,
        &host,
        &format!("/lobby/games/{room}/seats/0"),
        &json!({ "deck_id": host_bad }),
    );
    assert_eq!(status, 200, "{body}");
    let (status, body) = post(
        port,
        &host,
        "/lobby/games",
        &json!({ "deck_id": host_bad, "seats": 2 }),
    );
    assert_eq!(status, 200, "{body}");
}

#[test]
fn a_sideboard_card_and_a_commander_count_too() {
    let c = console("cb_zones");
    let port = c.gw.port;
    let host = login(port, "zones", "zones");
    let guest = login(port, "zonesguest", "zonesguest");

    let leader = baylee_cards::pool::rows()
        .iter()
        .find(|c| c.commander && c.identity == "G")
        .expect("this pool has a green commander")
        .english_name
        .clone();
    let stray = baylee_cards::pool::rows()
        .iter()
        .find(|c| !c.commander && !c.basic_land && c.identity == "R")
        .expect("this pool has a red card")
        .english_name
        .clone();
    let sided = save_deck(
        port,
        &host,
        &json!({ "name": "Sided", "cards": ["40 Forest", "20 Swamp"], "sideboard": ["2 Counterspell"] }),
    );
    let led = save_deck(
        port,
        &host,
        &json!({
            "name": "Led",
            "cards": [format!("1 {leader}"), format!("3 {stray}")],
            "sideboard": [],
            "commanders": [leader],
        }),
    );
    let plain_deck = save_deck(port, &guest, &lands("Plain"));
    let room = open_room(port, &guest, &plain_deck);

    // Not blocked: the sideboard is fine to sit with.
    let (status, body) = post(
        port,
        &host,
        &format!("/lobby/games/{room}/join"),
        &json!({ "deck_id": sided }),
    );
    assert_eq!(status, 200, "{body}");
    post(
        port,
        &host,
        &format!("/lobby/games/{room}/leave"),
        &json!({}),
    );

    // The sideboard.
    let counter = block(&c, card("Counterspell"), REASON, None);
    assert_eq!(
        blocked_on(port, &host, "/decks", "Sided")[0]["name"],
        "Counterspell",
        "a sideboard card is listed"
    );
    let answer = post(
        port,
        &host,
        &format!("/lobby/games/{room}/join"),
        &json!({ "deck_id": sided }),
    );
    assert_refused(&answer, "Sided", "Counterspell", REASON);
    lift(&c, counter["id"].as_str().unwrap());

    // The commander.
    block(&c, card(&leader), "the leader", None);
    assert_eq!(
        blocked_on(port, &host, "/decks", "Led")[0]["name"],
        leader.as_str(),
        "a commander is listed"
    );
    let answer = post(
        port,
        &host,
        &format!("/lobby/games/{room}/join"),
        &json!({ "deck_id": led }),
    );
    assert_refused(&answer, "Led", &leader, "the leader");
    let answer = post(
        port,
        &host,
        "/lobby/games",
        &json!({ "deck_id": led, "seats": 3 }),
    );
    assert_refused(&answer, "Led", &leader, "the leader");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ready_and_start_are_refused_for_a_card_blocked_after_the_deck_was_chosen() {
    let c = console("cb_ready");
    let port = c.gw.port;
    let _agent = attach_agent(&c.gw).await;
    let host = login(port, "readyhost", "readyhost");
    let guest = login(port, "readyguest", "readyguest");
    let host_deck = save_deck(port, &host, &deck_with("Host Strixes", "Baleful Strix"));
    let guest_deck = save_deck(port, &guest, &deck_with("Guest Strixes", "Baleful Strix"));
    let room = open_room(port, &host, &host_deck);
    let (status, body) = post(
        port,
        &guest,
        &format!("/lobby/games/{room}/join"),
        &json!({ "deck_id": guest_deck }),
    );
    assert_eq!(status, 200, "chosen while nothing was blocked: {body}");

    // Blocked afterwards: the guest cannot say ready.
    let first = block(&c, card("Baleful Strix"), REASON, None);
    let ready = post(
        port,
        &guest,
        &format!("/lobby/games/{room}/ready"),
        &json!({ "ready": true }),
    );
    assert_refused(&ready, "Guest Strixes", "Baleful Strix", REASON);
    // Saying not ready is not a thing to refuse.
    let (status, body) = post(
        port,
        &guest,
        &format!("/lobby/games/{room}/ready"),
        &json!({ "ready": false }),
    );
    assert_eq!(status, 200, "{body}");

    // Lifted, ready goes through.
    lift(&c, first["id"].as_str().unwrap());
    let (status, body) = post(
        port,
        &guest,
        &format!("/lobby/games/{room}/ready"),
        &json!({ "ready": true }),
    );
    assert_eq!(status, 200, "{body}");

    // Blocked again after everybody is ready: the game is not built.
    let second = block(&c, card("Baleful Strix"), REASON, None);
    let start = post(
        port,
        &host,
        &format!("/lobby/games/{room}/start"),
        &json!({}),
    );
    assert_refused(&start, "Host Strixes", "Baleful Strix", REASON);
    let (_, listing) = get(port, &guest, "/lobby/games");
    assert!(
        listing.to_string().contains("\"state\":\"waiting\""),
        "the room waits: {listing}"
    );

    // Lifted, the same room starts.
    lift(&c, second["id"].as_str().unwrap());
    let (status, body) = post(
        port,
        &host,
        &format!("/lobby/games/{room}/start"),
        &json!({}),
    );
    assert_eq!(status, 200, "start: {body}");
    let (_, listing) = get(port, &guest, "/lobby/games");
    assert!(
        listing.to_string().contains("\"state\":\"playing\""),
        "{listing}"
    );

    // A running game is never touched by a block made now.
    block(&c, card("Baleful Strix"), REASON, None);
    let (_, listing) = get(port, &guest, "/lobby/games");
    assert!(
        listing.to_string().contains("\"state\":\"playing\""),
        "still playing: {listing}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_house_deck_is_checked_for_an_ai_chair_and_the_one_tap_game() {
    let c = console("cb_house");
    let port = c.gw.port;
    let _agent = attach_agent(&c.gw).await;
    let host = login(port, "househost", "househost");
    let deck = save_deck(port, &host, &lands("Lands"));
    let rift = card("Cyclonic Rift");

    // A room with a waiting chair for the AI, made while nothing is blocked.
    let room = open_room(port, &host, &deck);
    let blocked = block(&c, rift, REASON, None);

    // The AI cannot be seated without a deck of its own while the house deck
    // holds the card.
    let seat = post(
        port,
        &host,
        &format!("/lobby/games/{room}/seats/1"),
        &json!({ "kind": "ai", "ai": "sharp" }),
    );
    assert_refused(&seat, "Victory", "Cyclonic Rift", REASON);
    let (_, listing) = get(port, &host, "/lobby/games");
    assert!(
        !listing.to_string().contains("\"kind\":\"ai\""),
        "the chair stayed open: {listing}"
    );

    // The one-tap game against the house is refused for the same reason.
    let one_tap = post(
        port,
        &host,
        "/lobby/games",
        &json!({ "deck_id": deck, "mode": "ai" }),
    );
    assert_refused(&one_tap, "Victory", "Cyclonic Rift", REASON);

    // Lifted, both go through.
    lift(&c, blocked["id"].as_str().unwrap());
    let (status, body) = post(
        port,
        &host,
        &format!("/lobby/games/{room}/seats/1"),
        &json!({ "kind": "ai", "ai": "sharp" }),
    );
    assert_eq!(status, 200, "{body}");

    // Blocked after the AI chair was seated: the start is refused, the
    // house deck being asked again where the game is built.
    let blocked = block(&c, rift, REASON, None);
    let start = post(
        port,
        &host,
        &format!("/lobby/games/{room}/start"),
        &json!({}),
    );
    assert_refused(&start, "Victory", "Cyclonic Rift", REASON);
    lift(&c, blocked["id"].as_str().unwrap());
    let (status, body) = post(
        port,
        &host,
        &format!("/lobby/games/{room}/start"),
        &json!({}),
    );
    assert_eq!(status, 200, "start: {body}");

    // A different account tests the one-tap path: the room's host is now
    // playing and may not open a second simultaneous game on this gateway.
    let one_tap_player = login(port, "house-one-tap", "OneTap");
    let one_tap_deck = save_deck(port, &one_tap_player, &lands("One Tap Lands"));
    let (status, body) = post(
        port,
        &one_tap_player,
        "/lobby/games",
        &json!({ "deck_id": one_tap_deck, "mode": "ai" }),
    );
    assert_eq!(status, 200, "one tap, nothing blocked: {body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_one_tap_game_refuses_the_players_own_blocked_card() {
    let c = console("cb_one_tap");
    let port = c.gw.port;
    let _agent = attach_agent(&c.gw).await;
    let host = login(port, "onetap", "onetap");
    let deck = save_deck(port, &host, &deck_with("Strixes", "Baleful Strix"));
    let row = block(&c, card("Baleful Strix"), REASON, None);

    let refused = post(
        port,
        &host,
        "/lobby/games",
        &json!({ "deck_id": deck, "mode": "ai", "ai": "sharp" }),
    );
    assert_refused(&refused, "Strixes", "Baleful Strix", REASON);

    lift(&c, row["id"].as_str().unwrap());
    let (status, body) = post(
        port,
        &host,
        "/lobby/games",
        &json!({ "deck_id": deck, "mode": "ai" }),
    );
    assert_eq!(status, 200, "{body}");
}
