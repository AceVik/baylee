//! Every card-text fixture is what the catalog says, word for word.
//!
//! `baylee-cardtext`'s fixture file is the only place German, French and
//! Japanese card text enters this repository's tests, and it is held to one
//! rule: nothing in it is typed by hand. This reads each printing back from
//! an **ingested** catalog — `public`, filled by `baylee-catalog ingest` —
//! and fails on any field that differs, which is what stops the next
//! invented sentence at the door.
//!
//! It is `#[ignore]`d because CI's Postgres is empty: the job has a server
//! and no ingest, so the test would find no rows there. Run it on purpose:
//!
//! ```text
//! cargo test -p baylee-catalog --test cardtext_provenance -- --ignored
//! ```
//!
//! Run that way it **fails** without `DATABASE_URL` and against an empty
//! `cards` table, rather than passing over nothing.

use sea_orm::{ConnectionTrait, Database, DbBackend, Statement, Value};
use serde::Deserialize;

#[derive(Deserialize)]
struct File {
    cards: Vec<Card>,
}

#[derive(Deserialize)]
struct Card {
    name: String,
    lang: String,
    oracle_id: String,
    oracle: Vec<String>,
    printings: Vec<Printing>,
}

#[derive(Deserialize)]
struct Printing {
    scryfall_id: String,
    set: String,
    collector_number: String,
    released_at: String,
    layout: String,
    printed: Vec<Option<String>>,
}

#[tokio::test]
#[ignore = "needs an ingested catalog; see the module header"]
async fn every_fixture_row_is_the_catalog_s_row() {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .expect("DATABASE_URL is not set, and this test reads an ingested catalog");
    let db = Database::connect(&url).await.expect("connecting");
    let file: File = serde_json::from_str(include_str!(
        "../../baylee-cardtext/fixtures/printings.json"
    ))
    .expect("the fixture file parses");

    let mut wrong = Vec::new();
    let mut read = 0_usize;
    for card in &file.cards {
        for printing in &card.printings {
            let rows = db
                .query_all_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT c.oracle_id::text AS oracle_id, c.lang, c.set_code, \
                            c.collector_number, to_char(c.released_at, 'YYYY-MM-DD') AS released_at, \
                            c.layout, f.face_index, f.oracle_text, f.printed_text \
                     FROM cards c JOIN card_faces f USING (scryfall_id) \
                     WHERE c.scryfall_id = $1::uuid ORDER BY f.face_index",
                    [Value::from(printing.scryfall_id.clone())],
                ))
                .await
                .expect("reading a printing");
            let at = format!(
                "{} ({}) {} {}",
                card.name, card.lang, printing.set, printing.collector_number
            );
            if rows.is_empty() {
                wrong.push(format!("{at}: no such printing in the catalog"));
                continue;
            }
            read += 1;
            let text = |row: &sea_orm::QueryResult, column: &str| -> Option<String> {
                row.try_get::<Option<String>>("", column)
                    .expect("a text column")
            };
            let first = &rows[0];
            let header = [
                ("oracle_id", Some(card.oracle_id.clone())),
                ("lang", Some(card.lang.clone())),
                ("set_code", Some(printing.set.clone())),
                ("collector_number", Some(printing.collector_number.clone())),
                ("released_at", Some(printing.released_at.clone())),
                ("layout", Some(printing.layout.clone())),
            ];
            for (column, want) in header {
                let got = text(first, column);
                if got != want {
                    wrong.push(format!("{at}: {column} is {got:?}, the file says {want:?}"));
                }
            }
            let printed: Vec<Option<String>> =
                rows.iter().map(|r| text(r, "printed_text")).collect();
            if printed != printing.printed {
                wrong.push(format!("{at}: printed text differs from the catalog's"));
            }
            let oracle: Vec<String> = rows
                .iter()
                .map(|r| text(r, "oracle_text").unwrap_or_default())
                .collect();
            if oracle != card.oracle {
                wrong.push(format!("{at}: Oracle text differs from the catalog's"));
            }
        }
    }
    assert!(
        read >= 90,
        "only {read} fixture printings were found: is the catalog ingested?"
    );
    assert!(
        wrong.is_empty(),
        "{} fixture row(s) are not the catalog's:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The catalog serves each fixture card from the printing the rule picks
/// over the fixture's printings.
///
/// The file was cut only where the cut left the pick unchanged, so over the
/// whole catalog the answer has to be the same printing. A query that drops
/// a printing, orders faces wrongly or reads the wrong column would change it,
/// and nothing short of a full catalog can see that: every sandbox holds only
/// the rows its own test wrote. A card no printing translated has no printed
/// layer at all.
#[tokio::test]
#[ignore = "needs an ingested catalog; see the module header"]
async fn the_catalog_serves_the_printing_the_rule_picks() {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .expect("DATABASE_URL is not set, and this test reads an ingested catalog");
    let catalog = baylee_catalog::Catalog::connect(&url)
        .await
        .expect("connecting");
    let file: File = serde_json::from_str(include_str!(
        "../../baylee-cardtext/fixtures/printings.json"
    ))
    .expect("the fixture file parses");

    let mut wrong = Vec::new();
    let mut picked = 0_usize;
    let mut untranslated = Vec::new();
    for card in &file.cards {
        let at = format!("{} ({})", card.name, card.lang);
        let entries = catalog
            .text_by_card(std::slice::from_ref(&card.oracle_id), &card.lang)
            .await
            .expect("looking up text");
        let [entry] = entries.as_slice() else {
            wrong.push(format!("{at}: {} entries", entries.len()));
            continue;
        };
        let oracle: Vec<&str> = card.oracle.iter().map(String::as_str).collect();
        let printings: Vec<baylee_cardtext::Printing> = card
            .printings
            .iter()
            .map(|p| baylee_cardtext::Printing {
                scryfall_id: p.scryfall_id.clone(),
                released_at: p.released_at.clone(),
                collector_number: p.collector_number.clone(),
                layout: p.layout.clone(),
                printed: p.printed.clone(),
            })
            .collect();
        let Some(layer) = baylee_cardtext::pick(&card.lang, &oracle, &printings) else {
            untranslated.push(at.clone());
            if entry.faces.iter().any(|f| f.printed.is_some()) {
                wrong.push(format!(
                    "{at}: nothing is translated, and a layer was served"
                ));
            }
            continue;
        };
        picked += 1;
        if entry.scryfall_id != layer.printing.scryfall_id {
            wrong.push(format!(
                "{at}: served {} where the rule picks {}",
                entry.scryfall_id, layer.printing.scryfall_id
            ));
            continue;
        }
        let want: Vec<Option<&str>> = layer
            .printing
            .printed
            .iter()
            .zip(&oracle)
            .map(|(printed, oracle)| {
                printed
                    .as_deref()
                    .filter(|t| !baylee_cardtext::untranslated(oracle, Some(t)))
            })
            .collect();
        let got: Vec<Option<&str>> = entry.faces.iter().map(|f| f.printed.as_deref()).collect();
        if got != want {
            wrong.push(format!("{at}: printed {got:?}, the rule says {want:?}"));
        }
    }
    // Measured 2026-09-24: three fixture cards have no translated printing,
    // so the other 24 are served a picked one.
    assert_eq!(
        untranslated,
        [
            "Elven Palisade (de)",
            "Kavaron, Memorial World (de)",
            "Skullclamp (en)"
        ],
        "which cards no printing translates"
    );
    assert_eq!(picked, 24, "is the catalog ingested?");
    assert!(
        wrong.is_empty(),
        "{} card(s) are not served as the rule picks:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
