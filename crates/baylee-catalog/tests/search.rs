//! What only a real catalog can answer: a search returns cards, not printings.
//!
//! The catalog keeps one row per *printing per language*, which is right —
//! a player reads the piece of cardboard they own. A deck builder asks the
//! other question, *which card do you mean*, and for most of this crate's
//! life it was answered with every printing of it: a live search for
//! `Blitzschlag` came back twelve rows deep in one card. Nothing in the crate
//! could notice, because a `Vec<SearchHit>` of twelve is a perfectly good
//! `Vec` and only a filled catalog makes it wrong.
//!
//! Each test runs in a **schema of its own**, created and dropped around it,
//! so it never touches the half a gigabyte of ingested card text a
//! developer's `public` holds — and `CREATE TABLE IF NOT EXISTS` looks at
//! the schema it is creating in, so the catalog's own DDL builds fresh
//! tables here while `public` stays on the path for `similarity()`.
//!
//! Without `DATABASE_URL` they fail rather than skip, for the reason
//! `baylee-db`'s schema tests give: a green suite with no server says a
//! query works when nothing ran it.

use baylee_catalog::{Catalog, scryfall};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use std::sync::atomic::{AtomicU32, Ordering};

/// A catalog in a schema of its own.
struct Sandbox {
    catalog: Catalog,
    admin: DatabaseConnection,
    schema: String,
}

/// Distinguishes two schemas made in the same process in the same second.
static NTH: AtomicU32 = AtomicU32::new(0);

impl Sandbox {
    async fn open(what: &str) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| {
                panic!(
                    "DATABASE_URL is not set, and these tests are about PostgreSQL.\n  \
                     docker compose up -d\n  \
                     export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee"
                )
            });

        let schema = format!(
            "t_cat_{what}_{}_{}",
            std::process::id(),
            NTH.fetch_add(1, Ordering::Relaxed)
        );

        let admin = Database::connect(&url).await.expect("connecting");
        admin
            .execute_unprepared(&format!("CREATE SCHEMA \"{schema}\""))
            .await
            .expect("creating a schema");

        let sep = if url.contains('?') { '&' } else { '?' };
        let scoped = format!("{url}{sep}options=-c%20search_path%3D{schema},public");
        let catalog = Catalog::connect(&scoped).await.expect("connecting scoped");
        catalog
            .migrate()
            .await
            .expect("building the catalog schema");

        Self {
            catalog,
            admin,
            schema,
        }
    }

    async fn close(self) {
        self.admin
            .execute_unprepared(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .await
            .expect("dropping the schema");
    }
}

/// The oracle id every printing below shares.
const BOLT: &str = "4457ed35-7c10-48c8-9776-456485fdf070";

/// One printing, as Scryfall would hand it over.
fn printing(
    id: &str,
    lang: &str,
    set: &str,
    released: &str,
    printed_name: Option<&str>,
    printed_type: Option<&str>,
) -> scryfall::Card {
    scryfall::Card {
        id: id.to_string(),
        oracle_id: Some(BOLT.to_string()),
        lang: lang.to_string(),
        set: set.to_string(),
        released_at: Some(released.to_string()),
        name: "Lightning Bolt".to_string(),
        printed_name: printed_name.map(str::to_string),
        type_line: Some("Instant".to_string()),
        printed_type_line: printed_type.map(str::to_string),
        oracle_text: Some("Lightning Bolt deals 3 damage to any target.".to_string()),
        ..scryfall::Card::default()
    }
}

/// Five printings of one card: three English, two German, and the newest
/// German one carrying a printed name but no printed type line — which is
/// not a contrivance but 6489 of the 59465 German faces in the live
/// catalog, `tle` among them.
fn one_card_five_ways() -> Vec<scryfall::Card> {
    vec![
        printing(
            "00000000-0000-4000-8000-000000000001",
            "en",
            "lea",
            "1993-08-05",
            None,
            None,
        ),
        printing(
            "00000000-0000-4000-8000-000000000002",
            "en",
            "m10",
            "2009-07-17",
            None,
            None,
        ),
        printing(
            "00000000-0000-4000-8000-000000000003",
            "en",
            "2x2",
            "2022-07-08",
            None,
            None,
        ),
        printing(
            "00000000-0000-4000-8000-000000000004",
            "de",
            "m11",
            "2010-07-16",
            Some("Blitzschlag"),
            Some("Spontanzauber"),
        ),
        printing(
            "00000000-0000-4000-8000-000000000005",
            "de",
            "tle",
            "2025-11-21",
            Some("Blitzschlag"),
            None,
        ),
    ]
}

#[tokio::test]
async fn a_search_answers_with_cards_and_not_with_printings() {
    let sandbox = Sandbox::open("one").await;
    sandbox
        .catalog
        .upsert(&one_card_five_ways())
        .await
        .expect("upserting five printings");

    let hits = sandbox
        .catalog
        .search("Blitzschlag", "de", 20)
        .await
        .expect("searching");

    assert_eq!(
        hits.len(),
        1,
        "five printings of one card are one answer, not five: {hits:?}"
    );
    assert_eq!(hits[0].name, "Blitzschlag");
    assert_eq!(hits[0].english_name, "Lightning Bolt");

    sandbox.close().await;
}

#[tokio::test]
async fn the_printing_that_stands_for_a_card_is_the_one_that_says_what_it_is() {
    let sandbox = Sandbox::open("rep").await;
    sandbox
        .catalog
        .upsert(&one_card_five_ways())
        .await
        .expect("upserting five printings");

    let hits = sandbox
        .catalog
        .search("Blitzschlag", "de", 20)
        .await
        .expect("searching");

    // The newest German printing is `tle`, and it has no printed type line.
    // Taking it would head a German card `Instant`.
    assert_eq!(
        hits[0].type_line, "Spontanzauber",
        "the representative printing must carry the translated type line"
    );

    // And it must be the same one twice: a `DISTINCT ON` whose sort has ties
    // keeps whichever row it happened to see first.
    let again = sandbox
        .catalog
        .search("Blitzschlag", "de", 20)
        .await
        .expect("searching again");
    assert_eq!(hits[0].scryfall_id, again[0].scryfall_id);

    sandbox.close().await;
}

#[tokio::test]
async fn the_limit_counts_cards() {
    let sandbox = Sandbox::open("limit").await;
    let mut cards = one_card_five_ways();
    // A second card, so a limit of one has something to leave out.
    for (nth, set) in ["lea", "m10", "2x2"].iter().enumerate() {
        cards.push(scryfall::Card {
            id: format!("00000000-0000-4000-8000-0000000001{nth:02}"),
            oracle_id: Some("aaaaaaaa-0000-4000-8000-000000000001".to_string()),
            lang: "de".to_string(),
            set: (*set).to_string(),
            released_at: Some("2000-01-01".to_string()),
            name: "Rhystic Lightning".to_string(),
            printed_name: Some("Rhystischer Blitzschlag".to_string()),
            type_line: Some("Instant".to_string()),
            printed_type_line: Some("Spontanzauber".to_string()),
            ..scryfall::Card::default()
        });
    }
    sandbox
        .catalog
        .upsert(&cards)
        .await
        .expect("upserting eight");

    let hits = sandbox
        .catalog
        .search("Blitzschlag", "de", 20)
        .await
        .expect("searching");
    assert_eq!(hits.len(), 2, "eight printings, two cards: {hits:?}");

    let one = sandbox
        .catalog
        .search("Blitzschlag", "de", 1)
        .await
        .expect("searching with a limit of one");
    assert_eq!(one.len(), 1, "a limit of one is one card, not one printing");

    sandbox.close().await;
}
