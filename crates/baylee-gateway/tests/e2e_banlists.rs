//! End-to-end tests for ban lists (`docs/protocol.md` §"Ban lists"): the
//! routes on a real store, who may see what, a table's check and its
//! reasons, and a running game's frozen version.
//!
//! The store's own questions are `baylee-db/tests/banlists.rs`, and what a
//! list means is `baylee_cards::formats`. What only this can see is the
//! gateway's part: that a session is asked for, that a list that is not the
//! caller's reads as none, that a deck is turned away with its reasons at
//! every door, and that an edit or an ingest cannot reach a running game.

#![allow(clippy::missing_docs_in_private_items)]

use crate::common;

use baylee_catalog::{Catalog, scryfall};
use common::{attach_agent, http, json_field, login, spawn_gateway};
use serde_json::{Value, json};

/// A card's oracle id, as `/pool` lists it.
fn oracle_of(port: u16, token: &str, name: &str) -> String {
    let (status, body) = http(port, "GET", "/pool?lang=en", Some(token), "");
    assert_eq!(status, 200, "{body}");
    let pool: Value = serde_json::from_str(&body).expect("the pool is JSON");
    pool["cards"]
        .as_array()
        .expect("a list of cards")
        .iter()
        .find(|card| card["english_name"] == name)
        .expect("the card is in the pool")["oracle_id"]
        .as_str()
        .expect("an oracle id")
        .to_owned()
}

fn printing(n: u32, name: &str, oracle_id: &str, verdicts: &[(&str, &str)]) -> scryfall::Card {
    scryfall::Card {
        id: format!("00000000-0000-4000-8000-{n:012}"),
        oracle_id: Some(oracle_id.to_owned()),
        lang: "en".to_owned(),
        set: "abc".to_owned(),
        set_name: Some("Set abc".to_owned()),
        collector_number: n.to_string(),
        released_at: Some("2020-01-01".to_owned()),
        rarity: Some("rare".to_owned()),
        name: name.to_owned(),
        type_line: Some("Artifact".to_owned()),
        legalities: verdicts
            .iter()
            .map(|(format, word)| ((*format).to_owned(), (*word).to_owned()))
            .collect(),
        ..scryfall::Card::default()
    }
}

/// A deck of `rows` for this account, and its id.
fn make_deck(port: u16, token: &str, name: &str, rows: &[&str], format: &str) -> String {
    let body = json!({"name": name, "cards": rows, "format": format}).to_string();
    let (status, body) = http(port, "POST", "/decks", Some(token), &body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

fn new_list(port: u16, token: &str, name: &str, format: &str) -> Value {
    let body = json!({"name": name, "format": format}).to_string();
    let (status, body) = http(port, "POST", "/banlists", Some(token), &body);
    assert_eq!(status, 200, "new list: {body}");
    serde_json::from_str(&body).expect("a list")
}

fn id_of(list: &Value) -> &str {
    list["id"].as_str().expect("an id")
}

/// Writes the cards of a list.
fn write_cards(port: u16, token: &str, id: &str, entries: &[(&str, &str)]) -> Value {
    let entries: Vec<Value> = entries
        .iter()
        .map(|(oracle_id, status)| json!({"oracle_id": oracle_id, "status": status}))
        .collect();
    let body = json!({"entries": entries}).to_string();
    let (status, body) = http(port, "PUT", &format!("/banlists/{id}"), Some(token), &body);
    assert_eq!(status, 200, "write list: {body}");
    serde_json::from_str(&body).expect("a list")
}

/// A room, opened by `host`, and its id.
fn open_room(port: u16, host: &str) -> String {
    let (status, body) = http(port, "POST", "/lobby/games", Some(host), r#"{"seats":2}"#);
    assert_eq!(status, 200, "open room: {body}");
    json_field(&body, "game_id").to_string()
}

fn pick_list(port: u16, host: &str, game: &str, list: &str) {
    let body = json!({"list_id": list}).to_string();
    let path = format!("/lobby/games/{game}/list");
    let (status, body) = http(port, "POST", &path, Some(host), &body);
    assert_eq!(status, 200, "pick a list: {body}");
}

/// The table's row in the listing, as `token` sees it.
fn table(port: u16, token: &str, game: &str) -> Value {
    let (status, body) = http(port, "GET", "/lobby/games", Some(token), "");
    assert_eq!(status, 200, "{body}");
    let page: Value = serde_json::from_str(&body).expect("a page");
    page["games"]
        .as_array()
        .expect("games")
        .iter()
        .find(|row| row["id"] == game)
        .expect("the table is listed")
        .clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_list_is_read_and_written_by_its_owner_alone() {
    let gw = spawn_gateway("banlist_own");
    let alice = login(gw.port, "alice", "Alice");
    let bob = login(gw.port, "bob", "Bob");

    let (status, _) = http(gw.port, "GET", "/banlists", None, "");
    assert_eq!(status, 401, "a session first");
    let (status, _) = http(
        gw.port,
        "POST",
        "/banlists",
        None,
        r#"{"name":"x","format":"modern"}"#,
    );
    assert_eq!(status, 401);

    let list = new_list(gw.port, &alice, "Kitchen table", "modern");
    let id = id_of(&list);
    assert_eq!(list["mine"], true);
    assert_eq!(list["builtin"], false);
    assert_eq!(list["version"], 1);
    assert_eq!(list["entries"], json!([]));
    assert!(
        list.get("owner").is_none(),
        "nothing says whose a list is: {list}"
    );

    let (status, body) = http(gw.port, "GET", "/banlists", Some(&bob), "");
    assert_eq!(status, 200);
    assert!(
        !body.contains("Kitchen table"),
        "another's list is in nobody's listing: {body}"
    );
    let path = format!("/banlists/{id}");
    for (method, body) in [("GET", ""), ("PUT", r#"{"name":"Mine"}"#), ("DELETE", "")] {
        let (status, answer) = http(gw.port, method, &path, Some(&bob), body);
        assert_eq!(status, 404, "{method}: {answer}");
        assert_eq!(
            json_field(&answer, "error"),
            "no such list",
            "and reads as none at all"
        );
    }
    let copy = json!({"name": "Stolen", "format": "modern", "copy_of": id}).to_string();
    let (status, _) = http(gw.port, "POST", "/banlists", Some(&bob), &copy);
    assert_eq!(status, 404, "nor can it be copied");

    let ring = oracle_of(gw.port, &alice, "Sol Ring");
    let written = write_cards(gw.port, &alice, id, &[(&ring, "restricted")]);

    // The listing carries a list's cards by index, which is what `list:`
    // searches by, and only to the one who may read it.
    let (status, body) = http(gw.port, "GET", "/banlists", Some(&alice), "");
    assert_eq!(status, 200, "{body}");
    let listed: Value = serde_json::from_str(&body).expect("lists");
    assert_eq!(
        listed["lists"][0]["indices"],
        json!([written["entries"][0]["index"]]),
        "{body}"
    );
    let (_, theirs) = http(gw.port, "GET", "/banlists", Some(&bob), "");
    assert!(!theirs.contains("indices"), "{theirs}");
    assert!(
        written.get("indices").is_none(),
        "one list says it per entry: {written}"
    );

    assert_eq!(written["version"], 2);
    assert_eq!(written["cards"], 1);
    assert_eq!(written["entries"][0]["name"], "Sol Ring");
    assert_eq!(written["entries"][0]["status"], "restricted");
    assert!(written["entries"][0]["index"].is_u64());

    let (status, _) = http(gw.port, "DELETE", &path, Some(&alice), "");
    assert_eq!(status, 204);
    let (status, _) = http(gw.port, "GET", &path, Some(&alice), "");
    assert_eq!(status, 404);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_list_is_held_to_its_bounds() {
    let gw = spawn_gateway("banlist_bounds");
    let alice = login(gw.port, "alice", "Alice");
    for bad in [
        json!({"name": "", "format": "modern"}),
        json!({"name": "x".repeat(61), "format": "modern"}),
        json!({"name": "Fine", "format": "Not A Key"}),
    ] {
        let (status, body) = http(gw.port, "POST", "/banlists", Some(&alice), &bad.to_string());
        assert_eq!(status, 400, "{bad}: {body}");
    }
    let list = new_list(gw.port, &alice, "Fine", "freeform");
    let path = format!("/banlists/{}", id_of(&list));
    let ring = oracle_of(gw.port, &alice, "Sol Ring");
    let twice = json!({"entries": [
        {"oracle_id": ring, "status": "banned"},
        {"oracle_id": ring, "status": "restricted"},
    ]});
    let (status, body) = http(gw.port, "PUT", &path, Some(&alice), &twice.to_string());
    assert_eq!(status, 400, "{body}");
    let unknown = json!({"entries": [{"oracle_id": "not-a-card", "status": "banned"}]});
    let (status, _) = http(gw.port, "PUT", &path, Some(&alice), &unknown.to_string());
    assert_eq!(status, 400);
}

/// The official lists come from the feed, nobody edits them, and a copy of
/// one is the player's.
#[tokio::test(flavor = "multi_thread")]
async fn an_official_list_follows_the_feed_and_is_copied_whole() {
    let gw = spawn_gateway("banlist_official");
    let alice = login(gw.port, "alice", "Alice");
    let ring = oracle_of(gw.port, &alice, "Sol Ring");
    let stone = oracle_of(gw.port, &alice, "Mind Stone");

    let (status, body) = http(gw.port, "GET", "/banlists", Some(&alice), "");
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_str::<Value>(&body).expect("lists")["lists"],
        json!([]),
        "an empty catalog has no official lists"
    );

    let catalog = Catalog::connect(&gw.database_url())
        .await
        .expect("connecting to the gateway's schema");
    catalog
        .upsert(&[
            printing(
                1,
                "Sol Ring",
                &ring,
                &[("modern", "banned"), ("vintage", "restricted")],
            ),
            printing(
                2,
                "Mind Stone",
                &stone,
                &[("modern", "legal"), ("vintage", "legal")],
            ),
            // A card outside the pool stays out of what this gateway serves.
            printing(
                3,
                "Not Of This Pool",
                "00000000-0000-4000-8000-00000000abcd",
                &[("modern", "banned")],
            ),
        ])
        .await
        .expect("seeding");
    catalog.bump_data_version().await.expect("the stamp");

    let (status, body) = http(gw.port, "GET", "/banlists", Some(&alice), "");
    assert_eq!(status, 200, "{body}");
    let lists: Value = serde_json::from_str(&body).expect("lists");
    let official: Vec<&Value> = lists["lists"].as_array().expect("lists").iter().collect();
    assert_eq!(official.len(), 2, "{body}");
    assert!(official.iter().all(|list| list["builtin"] == true));
    assert!(official.iter().all(|list| list["mine"] == false));
    let modern = official
        .iter()
        .find(|list| list["format"] == "modern")
        .expect("Modern's list");
    assert_eq!(modern["cards"], 1, "the card it bans");
    let id = modern["id"].as_str().expect("an id");

    let path = format!("/banlists/{id}");
    let (status, body) = http(gw.port, "PUT", &path, Some(&alice), r#"{"name":"Mine"}"#);
    assert_eq!(status, 403, "{body}");
    assert_eq!(json_field(&body, "error"), "official lists are read-only");
    let (status, _) = http(gw.port, "DELETE", &path, Some(&alice), "");
    assert_eq!(status, 404, "and nobody's to delete");

    let copy = json!({"name": "My Modern", "format": "modern", "copy_of": id}).to_string();
    let (status, body) = http(gw.port, "POST", "/banlists", Some(&alice), &copy);
    assert_eq!(status, 200, "{body}");
    let copy: Value = serde_json::from_str(&body).expect("a list");
    assert_eq!(copy["builtin"], false);
    assert_eq!(copy["mine"], true);
    assert_eq!(copy["entries"][0]["oracle_id"], ring.as_str());
    assert_eq!(copy["entries"][0]["status"], "banned");
}

/// A table with a list turns a deck away at every door and says why; a table
/// without one takes everything.
#[tokio::test(flavor = "multi_thread")]
async fn a_table_with_a_list_turns_a_deck_away_with_its_reasons() {
    let gw = spawn_gateway("banlist_join");
    let host = login(gw.port, "host", "Host");
    let guest = login(gw.port, "guest", "Guest");
    let ring = oracle_of(gw.port, &host, "Sol Ring");

    let clean = make_deck(gw.port, &guest, "Clean", &["60 Forest"], "freeform");
    let ringed = make_deck(
        gw.port,
        &guest,
        "Ringed",
        &["59 Forest", "1 Sol Ring"],
        "freeform",
    );

    // No list: everything may.
    let game = open_room(gw.port, &host);
    let join = json!({"deck_id": ringed}).to_string();
    let path = format!("/lobby/games/{game}/join");
    let (status, body) = http(gw.port, "POST", &path, Some(&guest), &join);
    assert_eq!(status, 200, "{body}");
    let (status, _) = http(
        gw.port,
        "POST",
        &format!("/lobby/games/{game}/leave"),
        Some(&guest),
        "",
    );
    assert_eq!(status, 204);

    let list = new_list(gw.port, &host, "No rings", "freeform");
    write_cards(gw.port, &host, id_of(&list), &[(&ring, "banned")]);
    pick_list(gw.port, &host, &game, id_of(&list));

    // The door: the deck stays outside and the player reads why.
    let (status, body) = http(gw.port, "POST", &path, Some(&guest), &join);
    assert_eq!(status, 409, "{body}");
    let refusal: Value = serde_json::from_str(&body).expect("a refusal");
    assert_eq!(refusal["refused"]["list"], "No rings");
    assert_eq!(refusal["refused"]["problems"][0]["kind"], "list_banned");
    assert!(refusal["refused"]["problems"][0]["card"].is_u64());
    assert!(
        refusal["error"]
            .as_str()
            .expect("words")
            .contains("Sol Ring"),
        "and one that cannot word it reads the sentence: {body}"
    );
    let row = table(gw.port, &host, &game);
    assert_eq!(row["seats"][1]["taken"], false, "the chair stayed empty");

    // A deck that keeps to it sits down.
    let join_clean = json!({"deck_id": clean}).to_string();
    let (status, body) = http(gw.port, "POST", &path, Some(&guest), &join_clean);
    assert_eq!(status, 200, "{body}");

    // The chair's own door is the same one.
    let seat = format!("/lobby/games/{game}/seats/1");
    let swap = json!({"deck_id": ringed}).to_string();
    let (status, body) = http(gw.port, "POST", &seat, Some(&guest), &swap);
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("list_banned"), "{body}");
}

/// A list that changes under a seated deck: the player reads why at their
/// next ready, and the host learns which chair and never which cards.
#[tokio::test(flavor = "multi_thread")]
async fn a_deck_a_new_list_rejects_is_told_at_ready_and_its_cards_stay_its_own() {
    let gw = spawn_gateway("banlist_ready");
    let _agent = attach_agent(&gw).await;
    let host = login(gw.port, "host", "Host");
    let guest = login(gw.port, "guest", "Guest");
    let ring = oracle_of(gw.port, &host, "Sol Ring");
    let mine = make_deck(gw.port, &host, "Mine", &["60 Forest"], "freeform");
    let ringed = make_deck(
        gw.port,
        &guest,
        "Ringed",
        &["59 Forest", "1 Sol Ring"],
        "freeform",
    );
    let open = json!({"seats": 2, "deck_id": mine}).to_string();
    let (status, body) = http(gw.port, "POST", "/lobby/games", Some(&host), &open);
    assert_eq!(status, 200, "{body}");
    let game = json_field(&body, "game_id").to_string();
    let join = json!({"deck_id": ringed}).to_string();
    let (status, _) = http(
        gw.port,
        "POST",
        &format!("/lobby/games/{game}/join"),
        Some(&guest),
        &join,
    );
    assert_eq!(status, 200);
    let ready = format!("/lobby/games/{game}/ready");
    let (status, _) = http(gw.port, "POST", &ready, Some(&guest), "{}");
    assert_eq!(status, 200);

    // The host picks a list the seated deck breaks: a new table, so the
    // chair's ready goes.
    let list = new_list(gw.port, &host, "No rings", "freeform");
    write_cards(gw.port, &host, id_of(&list), &[(&ring, "banned")]);
    pick_list(gw.port, &host, &game, id_of(&list));
    let row = table(gw.port, &guest, &game);
    assert_eq!(row["seats"][1]["ready"], false, "{row}");
    assert_eq!(row["seats"][1]["deck"], "Ringed", "it stays in its chair");
    assert_eq!(row["seats"][1]["off_list"], true, "and the table says so");
    assert_eq!(row["seats"][0]["off_list"], false, "of that deck alone");

    let (status, body) = http(gw.port, "POST", &ready, Some(&guest), "{}");
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("list_banned"), "{body}");

    // A deck that passed, and a list edited after it said ready: the start
    // is refused, names the chair and not the cards, and takes the ready.
    let free = make_deck(
        gw.port,
        &guest,
        "Free",
        &["59 Forest", "1 Mind Stone"],
        "freeform",
    );
    let swap = json!({"deck_id": free}).to_string();
    let (status, body) = http(
        gw.port,
        "POST",
        &format!("/lobby/games/{game}/seats/1"),
        Some(&guest),
        &swap,
    );
    assert_eq!(status, 200, "{body}");
    let row = table(gw.port, &guest, &game);
    assert_eq!(row["seats"][1]["off_list"], false, "another deck passes");
    let (status, body) = http(gw.port, "POST", &ready, Some(&guest), "{}");
    assert_eq!(status, 200, "{body}");
    let stone = oracle_of(gw.port, &host, "Mind Stone");
    write_cards(
        gw.port,
        &host,
        id_of(&list),
        &[(&ring, "banned"), (&stone, "banned")],
    );
    let start = format!("/lobby/games/{game}/start");
    let (status, body) = http(gw.port, "POST", &start, Some(&host), "");
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("seat 1"), "{body}");
    assert!(
        !body.contains("Mind Stone") && !body.contains("refused"),
        "the host does not read another player's deck: {body}"
    );
    assert_eq!(table(gw.port, &guest, &game)["seats"][1]["ready"], false);
}

/// Who sees what of a room's list.
#[tokio::test(flavor = "multi_thread")]
async fn a_rooms_list_is_seen_by_those_at_the_table() {
    let gw = spawn_gateway("banlist_seen");
    let host = login(gw.port, "host", "Host");
    let guest = login(gw.port, "guest", "Guest");
    let passer = login(gw.port, "passer", "Passer");
    let ring = oracle_of(gw.port, &host, "Sol Ring");
    let list = new_list(gw.port, &host, "Our secret list", "freeform");
    write_cards(gw.port, &host, id_of(&list), &[(&ring, "banned")]);
    let game = open_room(gw.port, &host);

    // Someone else's list cannot be picked, and reads as none.
    let other = new_list(gw.port, &guest, "Theirs", "freeform");
    let body = json!({"list_id": id_of(&other)}).to_string();
    let path = format!("/lobby/games/{game}/list");
    let (status, _) = http(gw.port, "POST", &path, Some(&host), &body);
    assert_eq!(status, 404);
    // Only the host picks, even a list of the caller's own.
    let body = json!({"list_id": id_of(&other)}).to_string();
    let (status, _) = http(gw.port, "POST", &path, Some(&guest), &body);
    assert_eq!(status, 403);
    // A format that is not the list's is no table anyone can read.
    let modern = new_list(gw.port, &host, "Modern", "modern");
    let body = json!({"list_id": id_of(&modern), "format": "legacy"}).to_string();
    let (status, _) = http(gw.port, "POST", &path, Some(&host), &body);
    assert_eq!(status, 400);
    pick_list(gw.port, &host, &game, id_of(&list));

    // A passer-by sees that there is a list, not what it is called.
    let row = table(gw.port, &passer, &game);
    assert_eq!(row["rules"]["list"]["builtin"], false, "{row}");
    assert_eq!(row["rules"]["list"]["name"], Value::Null, "{row}");
    assert_eq!(row["rules"]["list"]["version"], Value::Null);
    let (status, _) = http(gw.port, "GET", &path, Some(&passer), "");
    assert_eq!(status, 403, "and cannot read it");

    // Those at the table read all of it, and nothing says whose it is.
    let (status, _) = http(
        gw.port,
        "POST",
        &format!("/lobby/games/{game}/join"),
        Some(&guest),
        "{}",
    );
    assert_eq!(status, 200);
    let row = table(gw.port, &guest, &game);
    assert_eq!(row["rules"]["list"]["name"], "Our secret list");
    assert_eq!(row["rules"]["list"]["frozen"], false);
    let (status, body) = http(gw.port, "GET", &path, Some(&guest), "");
    assert_eq!(status, 200, "{body}");
    let held: Value = serde_json::from_str(&body).expect("a list");
    assert_eq!(held["entries"][0]["name"], "Sol Ring");
    assert_eq!(held["mine"], false, "not even to its owner: {held}");
    assert!(held.get("owner").is_none());

    // Taking the list away takes the check away.
    let (status, _) = http(gw.port, "POST", &path, Some(&host), "{}");
    assert_eq!(status, 200);
    assert_eq!(table(gw.port, &guest, &game)["rules"]["list"], Value::Null);
}

/// The owner deleting a list takes nothing from a room that picked it.
#[tokio::test(flavor = "multi_thread")]
async fn a_room_keeps_the_list_it_read() {
    let gw = spawn_gateway("banlist_kept");
    let host = login(gw.port, "host", "Host");
    let guest = login(gw.port, "guest", "Guest");
    let ring = oracle_of(gw.port, &host, "Sol Ring");
    let list = new_list(gw.port, &host, "No rings", "freeform");
    write_cards(gw.port, &host, id_of(&list), &[(&ring, "banned")]);
    let game = open_room(gw.port, &host);
    pick_list(gw.port, &host, &game, id_of(&list));
    let (status, _) = http(
        gw.port,
        "DELETE",
        &format!("/banlists/{}", id_of(&list)),
        Some(&host),
        "",
    );
    assert_eq!(status, 204);
    let ringed = make_deck(gw.port, &guest, "Ringed", &["1 Sol Ring"], "freeform");
    let join = json!({"deck_id": ringed}).to_string();
    let (status, body) = http(
        gw.port,
        "POST",
        &format!("/lobby/games/{game}/join"),
        Some(&guest),
        &join,
    );
    assert_eq!(status, 409, "{body}");
}

/// Starts a game of the host's deck against the house under this list.
fn start_under(port: u16, host: &str, deck: &str, list: &str) -> String {
    let open = json!({"seats": 2, "deck_id": deck}).to_string();
    let (status, body) = http(port, "POST", "/lobby/games", Some(host), &open);
    assert_eq!(status, 200, "{body}");
    let game = json_field(&body, "game_id").to_string();
    pick_list(port, host, &game, list);
    let seat = format!("/lobby/games/{game}/seats/1");
    let (status, body) = http(port, "POST", &seat, Some(host), r#"{"kind":"ai"}"#);
    assert_eq!(status, 200, "{body}");
    let start = format!("/lobby/games/{game}/start");
    let (status, body) = http(port, "POST", &start, Some(host), "");
    assert_eq!(status, 200, "{body}");
    game
}

/// **The frozen version.** A game keeps the version its decks passed: an edit
/// of the list reaches neither its listing nor what those at the table read.
#[tokio::test(flavor = "multi_thread")]
async fn an_edit_does_not_reach_a_running_game() {
    let gw = spawn_gateway("banlist_frozen");
    let _agent = attach_agent(&gw).await;
    let host = login(gw.port, "host", "Host");
    let ring = oracle_of(gw.port, &host, "Sol Ring");
    let stone = oracle_of(gw.port, &host, "Mind Stone");
    let list = new_list(gw.port, &host, "Own", "freeform");
    let written = write_cards(gw.port, &host, id_of(&list), &[(&ring, "banned")]);
    let frozen = written["version"].as_i64().expect("a version");
    let deck = make_deck(
        gw.port,
        &host,
        "Stones",
        &["59 Forest", "1 Mind Stone"],
        "freeform",
    );
    let game = start_under(gw.port, &host, &deck, id_of(&list));

    let row = table(gw.port, &host, &game);
    assert_eq!(row["state"], "playing", "{row}");
    assert_eq!(row["rules"]["list"]["frozen"], true);
    assert_eq!(row["rules"]["list"]["version"], frozen);

    // The edit lands, and would refuse this very deck.
    let edited = write_cards(gw.port, &host, id_of(&list), &[(&stone, "banned")]);
    assert_eq!(edited["version"], frozen + 1);

    let row = table(gw.port, &host, &game);
    assert_eq!(row["state"], "playing", "the game plays on: {row}");
    assert_eq!(row["rules"]["list"]["version"], frozen, "{row}");
    let path = format!("/lobby/games/{game}/list");
    let (status, body) = http(gw.port, "GET", &path, Some(&host), "");
    assert_eq!(status, 200, "{body}");
    let held: Value = serde_json::from_str(&body).expect("a list");
    assert_eq!(held["version"], frozen);
    assert_eq!(held["entries"].as_array().expect("cards").len(), 1);
    assert_eq!(held["entries"][0]["oracle_id"], ring.as_str());

    // A room that still waits reads the list as it stands.
    let guest = login(gw.port, "guest", "Guest");
    let room = open_room(gw.port, &guest);
    let theirs = new_list(gw.port, &guest, "Theirs", "freeform");
    pick_list(gw.port, &guest, &room, id_of(&theirs));
    write_cards(gw.port, &guest, id_of(&theirs), &[(&stone, "banned")]);
    let path = format!("/lobby/games/{room}/list");
    let (status, body) = http(gw.port, "GET", &path, Some(&guest), "");
    assert_eq!(status, 200, "{body}");
    let read: Value = serde_json::from_str(&body).expect("a list");
    assert_eq!(read["version"], 2, "{read}");
    assert_eq!(read["entries"][0]["oracle_id"], stone.as_str());
}

/// A refresh of an official list is the same question: the ingest moves the
/// list and the game keeps what its decks passed.
#[tokio::test(flavor = "multi_thread")]
async fn a_refresh_of_an_official_list_does_not_reach_a_running_game() {
    let gw = spawn_gateway("banlist_refresh");
    let _agent = attach_agent(&gw).await;
    let host = login(gw.port, "host", "Host");
    let ring = oracle_of(gw.port, &host, "Sol Ring");
    let stone = oracle_of(gw.port, &host, "Mind Stone");
    let forest = oracle_of(gw.port, &host, "Forest");
    let catalog = Catalog::connect(&gw.database_url())
        .await
        .expect("connecting to the gateway's schema");
    catalog
        .upsert(&[
            printing(1, "Sol Ring", &ring, &[("modern", "banned")]),
            printing(2, "Mind Stone", &stone, &[("modern", "legal")]),
            printing(3, "Forest", &forest, &[("modern", "legal")]),
        ])
        .await
        .expect("seeding");
    catalog.bump_data_version().await.expect("the stamp");
    let (status, body) = http(gw.port, "GET", "/banlists", Some(&host), "");
    assert_eq!(status, 200, "{body}");
    let lists: Value = serde_json::from_str(&body).expect("lists");
    let official = lists["lists"][0].clone();
    let frozen = official["version"].as_i64().expect("a version");

    let deck = make_deck(
        gw.port,
        &host,
        "Stones",
        &["59 Forest", "1 Mind Stone"],
        "modern",
    );
    let id = official["id"].as_str().expect("an id");
    let game = start_under(gw.port, &host, &deck, id);

    // The feed bans the card this very deck holds.
    catalog
        .upsert(&[printing(2, "Mind Stone", &stone, &[("modern", "banned")])])
        .await
        .expect("seeding");
    catalog.bump_data_version().await.expect("the stamp");
    let (status, body) = http(gw.port, "GET", &format!("/banlists/{id}"), Some(&host), "");
    assert_eq!(status, 200, "{body}");
    let list: Value = serde_json::from_str(&body).expect("a list");
    assert_eq!(list["cards"], 2, "the list moved: {body}");
    assert!(list["version"].as_i64().expect("a version") > frozen);

    let row = table(gw.port, &host, &game);
    assert_eq!(row["state"], "playing", "{row}");
    assert_eq!(row["rules"]["list"]["version"], frozen, "{row}");
    assert_eq!(row["rules"]["list"]["frozen"], true);
    let path = format!("/lobby/games/{game}/list");
    let (status, body) = http(gw.port, "GET", &path, Some(&host), "");
    assert_eq!(status, 200, "{body}");
    let held: Value = serde_json::from_str(&body).expect("a list");
    assert_eq!(held["entries"].as_array().expect("cards").len(), 1);
    assert_eq!(
        held["entries"][0]["oracle_id"],
        ring.as_str(),
        "the game's list is the one it started under"
    );
}

/// A deck is saved whatever it is, and told so, also one that became illegal
/// later: an ingest moves the verdict and the list says it.
#[tokio::test(flavor = "multi_thread")]
async fn a_deck_is_never_refused_for_being_illegal_and_is_told_so() {
    let gw = spawn_gateway("banlist_saved");
    let alice = login(gw.port, "alice", "Alice");
    let ring = oracle_of(gw.port, &alice, "Sol Ring");
    let forest = oracle_of(gw.port, &alice, "Forest");
    let catalog = Catalog::connect(&gw.database_url())
        .await
        .expect("connecting to the gateway's schema");
    catalog
        .upsert(&[
            printing(1, "Sol Ring", &ring, &[("modern", "legal")]),
            printing(2, "Forest", &forest, &[("modern", "legal")]),
        ])
        .await
        .expect("seeding");
    catalog.bump_data_version().await.expect("the stamp");

    let standing = |name: &str| -> Value {
        let (status, body) = http(gw.port, "GET", "/decks", Some(&alice), "");
        assert_eq!(status, 200, "{body}");
        let decks: Vec<Value> = serde_json::from_str(&body).expect("decks");
        decks
            .into_iter()
            .find(|deck| deck["name"] == name)
            .expect("the deck is listed")["legality"]
            .clone()
    };

    // Five copies: saved, and said.
    make_deck(
        gw.port,
        &alice,
        "Five",
        &["55 Forest", "5 Sol Ring"],
        "modern",
    );
    let five = standing("Five");
    assert_eq!(five["standing"], "illegal", "{five}");
    assert_eq!(five["problems"][0]["kind"], "too_many_copies");

    // Legal today.
    make_deck(
        gw.port,
        &alice,
        "Rings",
        &["56 Forest", "4 Sol Ring"],
        "modern",
    );
    assert_eq!(standing("Rings")["standing"], "legal");

    // Illegal tomorrow.
    catalog
        .upsert(&[printing(1, "Sol Ring", &ring, &[("modern", "banned")])])
        .await
        .expect("seeding");
    catalog.bump_data_version().await.expect("the stamp");
    let rings = standing("Rings");
    assert_eq!(rings["standing"], "illegal", "{rings}");
    assert_eq!(rings["problems"][0]["kind"], "banned");

    // A deck of no format has nothing to keep to.
    make_deck(gw.port, &alice, "Free", &["5 Sol Ring"], "freeform");
    assert_eq!(standing("Free")["standing"], "legal");
}
