//! End-to-end test for `GET /catalog/text`: the JSON a client reads.
//!
//! `baylee-cardtext` pins the wire types' field names, and the catalog's
//! tests pin which printing speaks for a card. What neither can see is the
//! route between them — the query parameters, the id filter, which of the
//! two lookups a request reaches — so this asks a real gateway over HTTP,
//! with a catalog seeded into its own schema.
//!
//! No test here asks for a printing the catalog lacks: asked by printing,
//! the gateway fetches it from Scryfall, and a test must not reach the
//! network.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_catalog::{CardTextEntry, Catalog, scryfall};
use common::{http, spawn_gateway};

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

    // By card: the pinned shape, whole. A malformed id beside the good one
    // is dropped rather than failing the batch.
    let (status, body) = http(
        gw.port,
        "GET",
        &format!("/catalog/text?lang=DE&oracle_ids=not-an-id,{STONE}"),
        None,
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
        None,
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
        None,
        "",
    );
    assert_eq!(status, 200, "{body}");
    let by_print = entries(&body);
    assert_eq!(by_print.len(), 1, "{body}");
    assert_eq!(by_print[0].scryfall_id, ENGLISH);
    assert_eq!(by_print[0].faces[0].oracle_text, GERMAN);

    // Neither parameter is no question, and no error.
    let (status, body) = http(gw.port, "GET", "/catalog/text?lang=de", None, "");
    assert_eq!((status, body.as_str()), (200, "[]"));
}
