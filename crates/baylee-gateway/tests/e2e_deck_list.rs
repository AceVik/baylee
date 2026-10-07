//! What `GET /decks` says about a deck beyond its name (WG-3): when it was
//! last saved, how much of it this build cannot play, the picture a deck
//! without a commander is shown by, and who painted it — the last only when
//! the gateway's catalog knows.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_cards::pool::{PoolCard, rows};
use baylee_catalog::{Catalog, scryfall};
use common::{http, json_field, login, spawn_gateway};

/// The first registry card matching, by the deck-row name a player writes.
fn card(what: &str, test: impl Fn(&PoolCard) -> bool) -> &'static PoolCard {
    rows()
        .iter()
        .find(|c| test(c))
        .unwrap_or_else(|| panic!("the pool has {what}"))
}

/// The account's one deck, as the list describes it.
fn listed(port: u16, token: &str) -> serde_json::Value {
    let (status, body) = http(port, "GET", "/decks", Some(token), "");
    assert_eq!(status, 200, "{body}");
    let list: serde_json::Value = serde_json::from_str(&body).expect("json");
    list.as_array().expect("a list")[0].clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_list_says_when_what_cannot_play_and_whose_art() {
    let gw = spawn_gateway("deck-list");
    let port = gw.port;
    let token = login(port, "lister", "Lister");

    let stub = card("a stub", |c| c.coverage == "unimplemented" && !c.commander);
    let spell = card("a 5-drop", |c| {
        c.coverage == "implemented" && !c.commander && c.cmc == 5 && !c.kinds.contains(&"Land")
    });
    let cards = format!(
        r#"["30 Forest","2 {}","1 {}"]"#,
        stub.english_name, spell.english_name
    );
    let body = format!(r#"{{"name":"List","cards":{cards}}}"#);
    let (status, answer) = http(port, "POST", "/decks", Some(&token), &body);
    assert_eq!(status, 200, "a stub is a card a deck may hold: {answer}");
    let id = json_field(&answer, "deck_id").to_string();

    let deck = listed(port, &token);
    assert_eq!(deck["unplayable"], 2, "{deck}");
    assert_eq!(deck["signature"]["name"], spell.english_name, "{deck}");
    assert_eq!(deck["signature"]["scryfall_id"], spell.scryfall_id);
    assert!(
        deck["signature"].get("artist").is_none(),
        "an artist without a catalog that knows one: {deck}"
    );
    let saved = deck["updated_at"].as_u64().expect("updated_at");
    assert!(saved > 1_700_000_000, "unix seconds: {deck}");

    // A save moves it (whole seconds, so wait one out).
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (status, answer) = http(port, "PUT", &format!("/decks/{id}"), Some(&token), &body);
    assert_eq!(status, 204, "{answer}");
    let deck = listed(port, &token);
    assert!(
        deck["updated_at"].as_u64().expect("updated_at") > saved,
        "a save left updated_at where it was: {deck}"
    );

    // The catalog learns the printing, and the list credits its painter.
    let catalog = Catalog::connect(&gw.database_url())
        .await
        .expect("connecting to the gateway's schema");
    catalog
        .upsert(&[scryfall::Card {
            id: spell.scryfall_id.to_owned(),
            oracle_id: Some(spell.oracle_id.to_owned()),
            lang: "en".to_owned(),
            set: "tst".to_owned(),
            collector_number: "1".to_owned(),
            layout: Some("normal".to_owned()),
            name: spell.english_name.clone(),
            artist: Some("Test Painter".to_owned()),
            ..scryfall::Card::default()
        }])
        .await
        .expect("seeding");
    let deck = listed(port, &token);
    assert_eq!(deck["signature"]["artist"], "Test Painter", "{deck}");
}
