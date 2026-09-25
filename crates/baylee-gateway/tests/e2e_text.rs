//! End-to-end test for `GET /catalog/text`: the JSON a client reads.
//!
//! `baylee-cardtext` pins the wire types' field names, and the catalog's
//! tests pin which printing speaks for a card. What neither can see is the
//! route between them — the query parameters, the id filter, which of the
//! two lookups a request reaches — so this asks a real gateway over HTTP,
//! with a catalog seeded into its own schema, signed in: the catalog
//! answers a session only (#270).
//!
//! No test here asks for a printing the catalog lacks: asked by printing,
//! the gateway fetches it from Scryfall, and a test must not reach the
//! network.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_catalog::{CardTextEntry, Catalog, scryfall};
use common::{http, login, spawn_gateway};

const STONE: &str = "00000000-0000-4000-8000-00000000a001";
const ENGLISH: &str = "00000000-0000-4000-8000-000000000001";
const GERMAN_ID: &str = "00000000-0000-4000-8000-000000000002";
const ORACLE: &str = "{T}: Add {C}.\n{1}, {T}, Sacrifice this artifact: Draw a card.";
const GERMAN: &str = "{T}: Erzeuge {C}.\n{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.";

fn printing(id: &str, lang: &str, printed: Option<&str>) -> scryfall::Card {
    let german = lang == "de";
    scryfall::Card {
        id: id.to_owned(),
        oracle_id: Some(STONE.to_owned()),
        lang: lang.to_owned(),
        set: "fic".to_owned(),
        collector_number: "353".to_owned(),
        released_at: Some("2025-06-13".to_owned()),
        layout: Some("normal".to_owned()),
        name: "Mind Stone".to_owned(),
        printed_name: german.then(|| "Gedankenstein".to_owned()),
        type_line: Some("Artifact".to_owned()),
        oracle_text: Some(ORACLE.to_owned()),
        printed_text: printed.map(str::to_owned),
        mana_cost: Some("{2}".to_owned()),
        ..scryfall::Card::default()
    }
}

fn entries(body: &str) -> Vec<CardTextEntry> {
    serde_json::from_str(body).unwrap_or_else(|e| panic!("{e}: {body}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn text_is_served_by_card_and_by_printing() {
    let gw = spawn_gateway("text");
    let catalog = Catalog::connect(&gw.database_url())
        .await
        .expect("connecting to the gateway's schema");
    catalog
        .upsert(&[
            printing(ENGLISH, "en", Some("TAP: ADD C")),
            printing(GERMAN_ID, "de", Some(GERMAN)),
        ])
        .await
        .expect("seeding");
    let token = login(gw.port, "reader", "Reader");

    // By card: the pinned shape, whole. A malformed id beside the good one
    // is dropped rather than failing the batch.
    let (status, body) = http(
        gw.port,
        "GET",
        &format!("/catalog/text?lang=DE&oracle_ids=not-an-id,{STONE}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{body}");
    let text = GERMAN.replace('\n', "\\n");
    assert_eq!(
        body,
        format!(
            r#"[{{"scryfall_id":"{GERMAN_ID}","oracle_id":"{STONE}","lang":"de","layout":"normal","faces":[{{"name":"Gedankenstein","english_name":"Mind Stone","type_line":"Artifact","oracle_text":"{text}","mana_cost":"{{2}}","printed":"{text}"}}]}}]"#
        )
    );

    // English is the Oracle and never a printed layer, the promo's own
    // wording included.
    let (status, body) = http(
        gw.port,
        "GET",
        &format!("/catalog/text?lang=en&oracle_ids={STONE}"),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{body}");
    let english = entries(&body);
    assert_eq!(english.len(), 1, "{body}");
    assert_eq!(english[0].faces[0].printed, None);
    assert_eq!(english[0].faces[0].oracle_text, ORACLE);

    // By printing, as a client that predates `oracle_ids` asks: the same
    // card's text, under the id it asked for.
    let (status, body) = http(
        gw.port,
        "GET",
        &format!("/catalog/text?lang=de&ids={}", ENGLISH.to_uppercase()),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{body}");
    let by_print = entries(&body);
    assert_eq!(by_print.len(), 1, "{body}");
    assert_eq!(by_print[0].scryfall_id, ENGLISH);
    assert_eq!(by_print[0].faces[0].oracle_text, GERMAN);

    // Neither parameter is no question, and no error.
    let (status, body) = http(gw.port, "GET", "/catalog/text?lang=de", Some(&token), "");
    assert_eq!((status, body.as_str()), (200, "[]"));
}

/// Mind Stone as the pool names it. Unlike [`STONE`], a pool card's text is
/// held in memory, which is what the test below is about.
const POOL_STONE: &str = "c97361b5-af16-4a7b-af85-a429dbaf4ad2";
const OLD_GERMAN: &str =
    "{T}: Erhöhe deinen Manavorrat um {1}.\n{1}, {T}, opfere den Gedankenstein: Ziehe eine Karte.";

fn pool_printing(id: &str, released: &str, printed: &str) -> scryfall::Card {
    scryfall::Card {
        id: id.to_owned(),
        oracle_id: Some(POOL_STONE.to_owned()),
        released_at: Some(released.to_owned()),
        set: format!("s{}", &id[id.len() - 4..]),
        ..printing(id, "de", Some(printed))
    }
}

/// An ingest writes from a process of its own. What it wrote is served,
/// by `/catalog/text` and by `/pool` alike, once it moves the catalog's
/// stamp, and not before: the gateway holds the pool's text until then.
#[tokio::test(flavor = "multi_thread")]
async fn a_pool_card_is_held_until_the_catalog_is_stamped() {
    let gw = spawn_gateway("text-held");
    let catalog = Catalog::connect(&gw.database_url())
        .await
        .expect("connecting to the gateway's schema");
    catalog
        .upsert(&[pool_printing(
            "00000000-0000-4000-8000-00000000b001",
            "2007-07-13",
            OLD_GERMAN,
        )])
        .await
        .expect("seeding");
    let token = login(gw.port, "holder", "Holder");
    let printed = || {
        let (status, body) = http(
            gw.port,
            "GET",
            &format!("/catalog/text?lang=de&oracle_ids={POOL_STONE}"),
            Some(&token),
            "",
        );
        assert_eq!(status, 200, "{body}");
        let found = entries(&body);
        assert_eq!(found.len(), 1, "{body}");
        found[0].faces[0].printed.clone()
    };
    let pool = || {
        let (status, body) = http(gw.port, "GET", "/pool?lang=de", None, "");
        assert_eq!(status, 200);
        body
    };
    assert_eq!(printed().as_deref(), Some(OLD_GERMAN));
    assert!(pool().contains("Erhöhe deinen Manavorrat"));

    catalog
        .upsert(&[pool_printing(
            "00000000-0000-4000-8000-00000000b002",
            "2025-06-13",
            GERMAN,
        )])
        .await
        .expect("an ingest's batch");
    assert_eq!(printed().as_deref(), Some(OLD_GERMAN), "no stamp yet");
    assert!(!pool().contains("Erzeuge {C}"), "no stamp yet");

    catalog
        .bump_data_version()
        .await
        .expect("the ingest's stamp");
    assert_eq!(printed().as_deref(), Some(GERMAN));
    let pool = pool();
    assert!(pool.contains("Erzeuge {C}"), "the pool moved with the text");
    assert!(!pool.contains("Erhöhe deinen Manavorrat"));
}

/// #270: the catalog is Scryfall's data, and a route anyone could call
/// re-served it to whoever asked. Text and search answer a session only,
/// asked before anything else: without one, or with one that has ended,
/// 401. With one, both answer.
#[tokio::test(flavor = "multi_thread")]
async fn the_catalog_answers_a_session_and_nobody_else() {
    let gw = spawn_gateway("text-session");
    let catalog = Catalog::connect(&gw.database_url())
        .await
        .expect("connecting to the gateway's schema");
    catalog
        .upsert(&[printing(ENGLISH, "en", None)])
        .await
        .expect("seeding");
    catalog
        .project()
        .await
        .expect("search reads the projection");
    let asks = [
        format!("/catalog/text?lang=en&oracle_ids={STONE}"),
        format!("/catalog/text?lang=en&ids={ENGLISH}"),
        "/catalog/search?q=Mind%20Stone".to_string(),
    ];
    for ask in &asks {
        let (status, body) = http(gw.port, "GET", ask, None, "");
        assert_eq!(status, 401, "{ask} without a session: {body}");
        let (status, body) = http(gw.port, "GET", ask, Some("not-a-session"), "");
        assert_eq!(status, 401, "{ask} with a made-up one: {body}");
    }

    let token = login(gw.port, "session", "Session");
    for ask in &asks {
        let (status, body) = http(gw.port, "GET", ask, Some(&token), "");
        assert_eq!(status, 200, "{ask} with a session: {body}");
        assert!(body.contains("Mind Stone"), "{ask}: {body}");
    }

    let (status, _) = http(gw.port, "POST", "/auth/logout", Some(&token), "");
    assert_eq!(status, 204);
    let (status, body) = http(gw.port, "GET", &asks[0], Some(&token), "");
    assert_eq!(status, 401, "once the session has ended: {body}");
}
