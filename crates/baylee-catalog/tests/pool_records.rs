//! What the pool's table is read from: the verdicts an ingest stores, and
//! what `Catalog::pool_records` reads back. Each test runs in a schema of its
//! own, as `tests/search.rs` explains, against a small fixture of its own:
//! without `DATABASE_URL` they fail rather than skip.

use baylee_catalog::{Catalog, PoolSet, ingest, scryfall};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};

struct Sandbox {
    catalog: Catalog,
    admin: DatabaseConnection,
    schema: String,
}

static NTH: AtomicU32 = AtomicU32::new(0);

impl Sandbox {
    async fn open(what: &str) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .expect("DATABASE_URL is not set, and these tests are about PostgreSQL");
        let schema = format!(
            "t_cat_pool_{what}_{}_{}",
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
        catalog.migrate().await.expect("building the schema");
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

const BOLT: &str = "aaaaaaaa-0000-4000-8000-000000000001";
const MOX: &str = "aaaaaaaa-0000-4000-8000-000000000002";

fn verdicts(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(format, word)| ((*format).to_string(), (*word).to_string()))
        .collect()
}

/// One printing, `n` its id's last digits.
fn printing(
    n: u32,
    oracle_id: &str,
    set: &str,
    lang: &str,
    rarity: &str,
    legalities: &[(&str, &str)],
) -> scryfall::Card {
    scryfall::Card {
        id: format!("00000000-0000-4000-8000-{n:012}"),
        oracle_id: Some(oracle_id.to_string()),
        lang: lang.to_string(),
        set: set.to_string(),
        set_name: Some(format!("The {set} set")),
        collector_number: n.to_string(),
        released_at: Some(format!("20{:02}-01-01", n % 100)),
        rarity: Some(rarity.to_string()),
        name: "A Card".to_string(),
        type_line: Some("Instant".to_string()),
        legalities: verdicts(legalities),
        ..scryfall::Card::default()
    }
}

fn feed(cards: &[scryfall::Card]) -> std::io::Cursor<String> {
    std::io::Cursor::new(
        cards
            .iter()
            .map(|card| serde_json::to_string(card).expect("serializing") + "\n")
            .collect(),
    )
}

async fn stored(catalog: &Catalog, card: &str) -> BTreeMap<String, String> {
    catalog
        .pool_records(&[card.to_string()])
        .await
        .expect("reading the records")
        .verdicts
        .remove(card)
        .unwrap_or_default()
}

/// Old School admits printings, so a card's printings disagree there: the
/// feed of 4.11.2026 has 779 such cards, and the row used to be whichever
/// printing came last. Asked in both orders and across two batches, because
/// a card's printings do not arrive in one.
#[tokio::test]
async fn a_cards_printings_disagreeing_leave_the_strongest_word() {
    let sandbox = Sandbox::open("merged").await;
    let catalog = &sandbox.catalog;
    let old = || printing(1, MOX, "leb", "en", "rare", &[("oldschool", "restricted")]);
    let reprint = || printing(2, MOX, "30a", "en", "rare", &[("oldschool", "not_legal")]);

    catalog
        .upsert(&[old(), reprint()])
        .await
        .expect("one batch");
    assert_eq!(
        stored(catalog, MOX).await,
        verdicts(&[("oldschool", "restricted")])
    );

    catalog.upsert(&[reprint()]).await.expect("a later batch");
    assert_eq!(
        stored(catalog, MOX).await,
        verdicts(&[("oldschool", "restricted")]),
        "a reprint written afterwards does not take the card out"
    );

    let bird = |n, word| printing(n, BOLT, "4ed", "ja", "rare", &[("oldschool", word)]);
    for word in ["legal", "banned", "not_legal"] {
        catalog.upsert(&[bird(3, word)]).await.expect("a batch");
    }
    assert_eq!(
        stored(catalog, BOLT).await,
        verdicts(&[("oldschool", "banned")]),
        "banned is the narrower statement than legal"
    );
    sandbox.close().await;
}

/// A batch merges into what is stored, so the old rows must go first: a card
/// unbanned between two ingests has to come out `legal`.
#[tokio::test]
async fn an_ingest_replaces_what_the_last_one_said() {
    let sandbox = Sandbox::open("reingest").await;
    let catalog = &sandbox.catalog;
    let banned = feed(&[printing(
        1,
        BOLT,
        "m21",
        "en",
        "rare",
        &[("modern", "banned")],
    )]);
    ingest::from_feed(catalog, move || Ok(banned))
        .await
        .expect("the first ingest");
    assert_eq!(
        stored(catalog, BOLT).await,
        verdicts(&[("modern", "banned")])
    );

    let lifted = feed(&[printing(
        1,
        BOLT,
        "m21",
        "en",
        "rare",
        &[("modern", "legal")],
    )]);
    ingest::from_feed(catalog, move || Ok(lifted))
        .await
        .expect("the second ingest");
    assert_eq!(
        stored(catalog, BOLT).await,
        verdicts(&[("modern", "legal")])
    );
    sandbox.close().await;
}

/// A feed that cannot be opened writes nothing, so it must not take the
/// verdicts away either.
#[tokio::test]
async fn a_feed_that_never_opens_leaves_the_verdicts_standing() {
    let sandbox = Sandbox::open("nofeed").await;
    let catalog = &sandbox.catalog;
    catalog
        .upsert(&[printing(
            1,
            BOLT,
            "m21",
            "en",
            "rare",
            &[("modern", "banned")],
        )])
        .await
        .expect("seeding");
    let failed = ingest::from_feed(catalog, || -> anyhow::Result<std::io::Cursor<String>> {
        anyhow::bail!("the download failed")
    })
    .await;
    assert!(failed.is_err());
    assert_eq!(
        stored(catalog, BOLT).await,
        verdicts(&[("modern", "banned")])
    );
    sandbox.close().await;
}

/// The whole read: languages and repeats fold into one pair each, sets come
/// newest first, and a card nobody asked about stays out.
#[tokio::test]
async fn the_records_hold_each_pair_once_and_only_what_was_asked() {
    let sandbox = Sandbox::open("read").await;
    let catalog = &sandbox.catalog;
    catalog
        .upsert(&[
            printing(2, BOLT, "lea", "en", "common", &[("modern", "legal")]),
            printing(3, BOLT, "lea", "de", "common", &[("modern", "legal")]),
            printing(20, BOLT, "m21", "en", "uncommon", &[("modern", "legal")]),
            printing(21, MOX, "sld", "en", "rare", &[("modern", "banned")]),
        ])
        .await
        .expect("seeding");
    let records = catalog
        .pool_records(&[BOLT.to_string()])
        .await
        .expect("reading the records");
    assert_eq!(
        records.sets,
        [
            PoolSet {
                code: "m21".to_string(),
                name: "The m21 set".to_string()
            },
            PoolSet {
                code: "lea".to_string(),
                name: "The lea set".to_string()
            }
        ]
    );
    let mut printed = records.printed[BOLT].clone();
    printed.sort();
    assert_eq!(
        printed,
        [
            ("lea".to_string(), "common".to_string()),
            ("m21".to_string(), "uncommon".to_string())
        ]
    );
    assert_eq!(records.verdicts.len(), 1, "{records:?}");
    assert!(
        catalog
            .pool_records(&[])
            .await
            .expect("asking about nothing")
            .sets
            .is_empty()
    );
    sandbox.close().await;
}
