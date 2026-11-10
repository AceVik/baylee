//! End-to-end test for `GET /pool/facts`: the table a client filters by, over
//! HTTP, from a catalog seeded into the gateway's own schema.
//!
//! The cache's reads are `poolfacts.rs`'s own tests. What only this can see is
//! the route: that a session is asked for, that the rows are keyed on the
//! registry index `/pool` serves, and that `have` is answered `304`.

#![allow(clippy::missing_docs_in_private_items)]

use crate::common;

use baylee_catalog::{Catalog, Legality, PoolFacts, PoolFactsBody, Rarity, scryfall};
use common::{http, json_field, login, spawn_gateway};

/// A pool card, as `/pool` lists it.
fn pool_row(port: u16, token: &str, name: &str) -> (u32, String) {
    let (status, body) = http(port, "GET", "/pool?lang=en", Some(token), "");
    assert_eq!(status, 200, "{body}");
    let pool: serde_json::Value = serde_json::from_str(&body).expect("the pool is JSON");
    let card = pool["cards"]
        .as_array()
        .expect("a list of cards")
        .iter()
        .find(|card| card["english_name"] == name)
        .expect("the card is in the pool");
    (
        u32::try_from(card["index"].as_u64().expect("an index")).expect("an index fits"),
        card["oracle_id"].as_str().expect("an oracle id").to_owned(),
    )
}

fn printing(n: u32, oracle_id: &str, set: &str, rarity: &str, word: &str) -> scryfall::Card {
    scryfall::Card {
        id: format!("00000000-0000-4000-8000-{n:012}"),
        oracle_id: Some(oracle_id.to_owned()),
        lang: "en".to_owned(),
        set: set.to_owned(),
        set_name: Some(format!("Set {set}")),
        collector_number: n.to_string(),
        released_at: Some(format!("20{n:02}-01-01")),
        rarity: Some(rarity.to_owned()),
        name: "Mind Stone".to_owned(),
        type_line: Some("Artifact".to_owned()),
        legalities: [("modern".to_owned(), word.to_owned())].into(),
        ..scryfall::Card::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_table_is_served_kept_and_refreshed_by_version() {
    let gw = spawn_gateway("poolfacts");
    let token = login(gw.port, "archivist", "Archivist");
    let (index, oracle_id) = pool_row(gw.port, &token, "Mind Stone");
    let catalog = Catalog::connect(&gw.database_url())
        .await
        .expect("connecting to the gateway's schema");

    // A session first, as `/pool` asks.
    let (status, _) = http(gw.port, "GET", "/pool/facts", None, "");
    assert_eq!(status, 401);

    // An empty catalog: an empty table, not an error, and no version a client
    // could keep it by.
    let (status, body) = http(gw.port, "GET", "/pool/facts", Some(&token), "");
    assert_eq!(status, 200, "{body}");
    assert_eq!(json_field(&body, "version"), "");
    let empty: PoolFactsBody = serde_json::from_str(&body).expect("a table");
    assert!(PoolFacts::from(empty).is_empty());

    catalog
        .upsert(&[
            printing(1, &oracle_id, "abc", "uncommon", "legal"),
            printing(2, &oracle_id, "xyz", "rare", "legal"),
        ])
        .await
        .expect("seeding");
    catalog.bump_data_version().await.expect("the stamp");

    let (status, body) = http(gw.port, "GET", "/pool/facts", Some(&token), "");
    assert_eq!(status, 200, "{body}");
    let table: PoolFactsBody = serde_json::from_str(&body).expect("a table");
    let version = table.version.clone();
    assert_eq!(version.len(), 16, "{body}");
    assert_eq!(table.formats, ["modern"]);
    assert_eq!(
        table
            .sets
            .iter()
            .map(|set| set.code.as_str())
            .collect::<Vec<_>>(),
        ["xyz", "abc"],
        "newest first"
    );
    let table = PoolFacts::from(table);
    assert_eq!(table.len(), 1, "only the card the catalog knows");
    let card = table.of(index).expect("keyed on the index /pool serves");
    assert_eq!(card.legality("modern"), Some(Legality::Legal));
    let mut rarities: Vec<_> = card.printings().map(|p| (p.set, p.rarity)).collect();
    rarities.sort_by_key(|(set, _)| *set);
    assert_eq!(
        rarities,
        [("abc", Some(Rarity::Uncommon)), ("xyz", Some(Rarity::Rare))]
    );

    // Named by its version it is not sent again; any other name is.
    let (status, body) = http(
        gw.port,
        "GET",
        &format!("/pool/facts?have={version}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 304, "{body}");
    assert_eq!(body, "");
    let (status, _) = http(gw.port, "GET", "/pool/facts?have=0000", Some(&token), "");
    assert_eq!(status, 200);

    // An ingest that changes a verdict gives the table a new version, so the
    // copy a client keeps stops being the one.
    catalog.forget_verdicts().await.expect("forgetting");
    catalog
        .upsert(&[printing(1, &oracle_id, "abc", "uncommon", "banned")])
        .await
        .expect("an ingest's batch");
    catalog.bump_data_version().await.expect("the stamp");
    let (status, body) = http(
        gw.port,
        "GET",
        &format!("/pool/facts?have={version}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{body}");
    let table = PoolFacts::from(serde_json::from_str::<PoolFactsBody>(&body).expect("a table"));
    assert_ne!(table.version(), version);
    assert_eq!(
        table.of(index).expect("the card").legality("modern"),
        Some(Legality::Banned)
    );
}
