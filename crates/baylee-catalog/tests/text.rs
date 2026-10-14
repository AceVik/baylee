//! What a card's text is read from, asked of a real catalog.
//!
//! `card_entry`'s own tests hold the rule over printings built in Rust; these
//! hold the query that gathers them — that it hands the rule every printing
//! in the language, the newest English one, and each printing's faces in
//! order — because a query that drops a printing does not fail, it serves
//! the wrong one.
//!
//! Each test runs in a schema of its own, as `search.rs`'s do and for the
//! reasons its header gives; the sandbox here is that one cut to what text
//! needs. Without `DATABASE_URL` they fail rather than skip.

use baylee_catalog::{CardTextEntry, Catalog, ingest, scryfall, upgrade};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use std::sync::atomic::{AtomicU32, Ordering};

struct Sandbox {
    catalog: Catalog,
    admin: DatabaseConnection,
    schema: String,
}

static NTH: AtomicU32 = AtomicU32::new(0);

impl Sandbox {
    async fn open(what: &str, cards: &[scryfall::Card]) -> Self {
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
            "t_text_{what}_{}_{}",
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
        catalog.upsert(cards).await.expect("upserting");
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

const STONE: &str = "00000000-0000-4000-8000-00000000a001";
const PATHWAY: &str = "00000000-0000-4000-8000-00000000a002";
const ORACLE: &str = "{T}: Add {C}.\n{1}, {T}, Sacrifice this artifact: Draw a card.";
const GERMAN: &str = "{T}: Erzeuge {C}.\n{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.";

/// A printing of the one-faced card, `n` its id's last digits.
fn stone(n: u32, lang: &str, date: &str, number: &str, printed: Option<&str>) -> scryfall::Card {
    let german = lang == "de";
    scryfall::Card {
        id: format!("00000000-0000-4000-8000-{n:012}"),
        oracle_id: Some(STONE.to_owned()),
        lang: lang.to_owned(),
        set: format!("s{n}"),
        collector_number: number.to_owned(),
        released_at: Some(date.to_owned()),
        layout: Some("normal".to_owned()),
        name: "Mind Stone".to_owned(),
        printed_name: german.then(|| "Gedankenstein".to_owned()),
        type_line: Some("Artifact".to_owned()),
        printed_type_line: german.then(|| "Artefakt".to_owned()),
        oracle_text: Some(ORACLE.to_owned()),
        printed_text: printed.map(str::to_owned),
        mana_cost: Some("{2}".to_owned()),
        ..scryfall::Card::default()
    }
}

/// A modal double-faced printing whose back is translated or not.
fn pathway(n: u32, date: &str, back: Option<&str>) -> scryfall::Card {
    let face =
        |name: &str, printed_name: &str, oracle: &str, printed: Option<&str>| scryfall::Face {
            name: name.to_owned(),
            printed_name: Some(printed_name.to_owned()),
            type_line: Some("Land".to_owned()),
            oracle_text: Some(oracle.to_owned()),
            printed_text: printed.map(str::to_owned),
            ..scryfall::Face::default()
        };
    scryfall::Card {
        id: format!("00000000-0000-4000-8000-{n:012}"),
        oracle_id: Some(PATHWAY.to_owned()),
        lang: "de".to_owned(),
        set: format!("p{n}"),
        collector_number: "1".to_owned(),
        released_at: Some(date.to_owned()),
        layout: Some("modal_dfc".to_owned()),
        name: "Clearwater Pathway // Murkwater Pathway".to_owned(),
        card_faces: Some(vec![
            face(
                "Clearwater Pathway",
                "Klarwasser-Pfad",
                "{T}: Add {U}.",
                Some("{T}: Erzeuge {U}."),
            ),
            face(
                "Murkwater Pathway",
                "Trübwasser-Pfad",
                "{T}: Add {B}.",
                back,
            ),
        ]),
        ..scryfall::Card::default()
    }
}

fn only(entries: &[CardTextEntry]) -> &CardTextEntry {
    assert_eq!(entries.len(), 1, "{entries:#?}");
    &entries[0]
}

fn stones() -> Vec<scryfall::Card> {
    vec![
        stone(1, "en", "2026-01-01", "7", None),
        // Newer English printings a German reader never sees, and one an
        // English reader must not see either: a promo's own wording.
        stone(2, "en", "2026-09-02", "113", Some("TAP: ADD C")),
        // The newest German printing prints no text at all.
        stone(3, "de", "2025-06-13", "353", None),
        // Two German printings tie on the date; the regular frame's lower
        // number speaks.
        stone(4, "de", "2024-02-09", "327", Some(GERMAN)),
        stone(5, "de", "2024-02-09", "263", Some(GERMAN)),
    ]
}

#[tokio::test]
async fn a_card_is_served_from_the_printing_the_rule_chooses() {
    let mut cards = stones();
    let sandbox = Sandbox::open("pick", &cards).await;
    let want = "00000000-0000-4000-8000-000000000005";

    let first = sandbox
        .catalog
        .text_by_card(&[STONE.to_owned()], "de")
        .await
        .expect("text");
    let entry = only(&first);
    assert_eq!(entry.scryfall_id, want);
    assert_eq!(
        (entry.oracle_id.as_str(), entry.lang.as_str()),
        (STONE, "de")
    );
    assert_eq!(entry.faces[0].printed.as_deref(), Some(GERMAN));
    assert_eq!(entry.faces[0].name, "Gedankenstein");

    // Stored the other way round, the answer does not move.
    cards.reverse();
    let other = Sandbox::open("pick_reversed", &cards).await;
    let second = other
        .catalog
        .text_by_card(&[STONE.to_owned()], "de")
        .await
        .expect("text");
    assert_eq!(second, first);
    sandbox.close().await;
    other.close().await;
}

/// Under `en` the newest English printing names the card, and the Oracle is
/// its text even where that printing prints its own.
#[tokio::test]
async fn english_is_the_oracle() {
    let sandbox = Sandbox::open("english", &stones()).await;
    let entries = sandbox
        .catalog
        .text_by_card(&[STONE.to_owned()], "en")
        .await
        .expect("text");
    let entry = only(&entries);
    assert_eq!(entry.scryfall_id, "00000000-0000-4000-8000-000000000002");
    assert_eq!(entry.lang, "en");
    assert_eq!(entry.faces[0].printed, None);
    assert_eq!(entry.faces[0].oracle_text, ORACLE);
    sandbox.close().await;
}

/// A language with no translated printing is answered in English, never
/// with a printing in it that says nothing: the untranslated German one is
/// still what names the card in German.
#[tokio::test]
async fn nothing_translated_is_the_oracle_under_the_language_s_name() {
    let cards = [
        stone(1, "en", "2026-01-01", "7", None),
        stone(3, "de", "2025-06-13", "353", Some(ORACLE)),
    ];
    let sandbox = Sandbox::open("untranslated", &cards).await;
    let entries = sandbox
        .catalog
        .text_by_card(&[STONE.to_owned()], "de")
        .await
        .expect("text");
    let entry = only(&entries);
    assert_eq!(entry.faces[0].name, "Gedankenstein");
    assert_eq!(entry.faces[0].printed, None);
    assert_eq!(entry.faces[0].oracle_text, ORACLE);

    let entries = sandbox
        .catalog
        .text_by_card(&[STONE.to_owned()], "fr")
        .await
        .expect("text");
    assert_eq!(only(&entries).lang, "en");
    sandbox.close().await;
}

/// Both faces come from one printing, in printed order, with the layout
/// the client's alignment needs.
#[tokio::test]
async fn a_double_faced_card_is_read_from_one_printing() {
    let cards = [
        pathway(11, "2025-01-01", None),
        pathway(12, "2020-09-25", Some("{T}: Erzeuge {B}.")),
    ];
    let sandbox = Sandbox::open("mdfc", &cards).await;
    let entries = sandbox
        .catalog
        .text_by_card(&[PATHWAY.to_owned()], "de")
        .await
        .expect("text");
    let entry = only(&entries);
    assert_eq!(entry.scryfall_id, "00000000-0000-4000-8000-000000000012");
    assert_eq!(entry.layout, "modal_dfc");
    let faces: Vec<(&str, Option<&str>)> = entry
        .faces
        .iter()
        .map(|f| (f.name.as_str(), f.printed.as_deref()))
        .collect();
    assert_eq!(
        faces,
        [
            ("Klarwasser-Pfad", Some("{T}: Erzeuge {U}.")),
            ("Trübwasser-Pfad", Some("{T}: Erzeuge {B}.")),
        ]
    );
    sandbox.close().await;
}

/// Asked by printing, the card's answer comes back under the id that was
/// asked for — an English printing asked for in German reads German — and
/// a card the catalog does not know is not answered.
#[tokio::test]
async fn a_printing_is_answered_as_its_card_under_its_own_id() {
    let sandbox = Sandbox::open("by_print", &stones()).await;
    let asked = "00000000-0000-4000-8000-000000000001".to_owned();
    let unknown = "00000000-0000-4000-8000-00000000ffff".to_owned();
    let entries = sandbox
        .catalog
        .text(&[asked.clone(), unknown.clone()], "de")
        .await
        .expect("text");
    let entry = only(&entries);
    assert_eq!(entry.scryfall_id, asked);
    assert_eq!(entry.faces[0].printed.as_deref(), Some(GERMAN));
    assert!(
        sandbox
            .catalog
            .text_by_card(&[unknown], "de")
            .await
            .expect("text")
            .is_empty()
    );
    sandbox.close().await;
}

/// The stamp a gateway's cache of card text is keyed on: none until the
/// first move, and moved by nothing but [`Catalog::bump_data_version`].
/// Not by an upsert: an ingest writes about 1 356 of them, and a gateway
/// would rebuild at each.
#[tokio::test]
async fn only_a_bump_moves_the_data_stamp() {
    let sandbox = Sandbox::open("stamp", &[]).await;
    let catalog = &sandbox.catalog;
    assert_eq!(catalog.data_version().await.expect("the stamp"), 0);
    catalog.upsert(&stones()).await.expect("upserting");
    assert_eq!(catalog.data_version().await.expect("the stamp"), 0);
    assert_eq!(catalog.bump_data_version().await.expect("moving"), 1);
    assert_eq!(catalog.bump_data_version().await.expect("moving"), 2);
    assert_eq!(catalog.data_version().await.expect("the stamp"), 2);
    sandbox.close().await;
}

/// Which card a printing is, for the printings the catalog knows; the
/// rest are left out rather than failing the lookup.
#[tokio::test]
async fn a_known_printing_names_its_card() {
    let sandbox = Sandbox::open("cards_of", &stones()).await;
    let known = "00000000-0000-4000-8000-000000000001".to_owned();
    let unknown = "00000000-0000-4000-8000-00000000ffff".to_owned();
    let cards = sandbox
        .catalog
        .cards_of(&[unknown, known.clone()])
        .await
        .expect("looking up");
    assert_eq!(cards, [(known, STONE.to_owned())]);
    assert!(
        sandbox
            .catalog
            .cards_of(&[])
            .await
            .expect("nothing")
            .is_empty()
    );
    sandbox.close().await;
}

/// The languages a gateway holds text in are the catalog's own list, the
/// one `migrate` seeds.
#[tokio::test]
async fn the_catalog_names_its_languages() {
    let sandbox = Sandbox::open("languages", &[]).await;
    let languages = sandbox.catalog.languages().await.expect("listing");
    assert_eq!(languages.len(), 19, "{languages:?}");
    assert!(languages.iter().any(|l| l == "de"));
    assert!(languages.iter().any(|l| l == "en"));
    sandbox.close().await;
}

/// An ingest moves the stamp before its first write as well as after its
/// last, so one that fails has still moved it: a gateway does not go on
/// serving what it read before a half-written catalog.
#[tokio::test]
async fn an_ingest_moves_the_stamp_before_and_after() {
    let sandbox = Sandbox::open("ingest", &[]).await;
    let catalog = &sandbox.catalog;
    let feed: String = stones()
        .iter()
        .map(|card| serde_json::to_string(card).expect("serializing") + "\n")
        .collect();
    let stored = ingest::from_feed(catalog, move || Ok(std::io::Cursor::new(feed)))
        .await
        .expect("ingesting");
    assert_eq!(stored, stones().len());
    assert_eq!(catalog.data_version().await.expect("the stamp"), 2);

    let failed = ingest::from_feed(catalog, || -> anyhow::Result<std::io::Cursor<String>> {
        anyhow::bail!("the download failed")
    })
    .await;
    assert!(failed.is_err());
    assert_eq!(catalog.data_version().await.expect("the stamp"), 3);
    sandbox.close().await;
}

fn authoritative_stones() -> Vec<scryfall::Card> {
    let mut cards = stones();
    for card in &mut cards {
        card.border_color = Some("black".to_owned());
    }
    cards[0].watermark = Some("azorius".to_owned());
    cards[1].security_stamp = Some("acorn".to_owned());
    cards
}

fn feed_of(cards: &[scryfall::Card]) -> std::io::Cursor<String> {
    std::io::Cursor::new(
        cards
            .iter()
            .map(|card| serde_json::to_string(card).expect("serializing") + "\n")
            .collect(),
    )
}

fn no_upgrade_download() -> anyhow::Result<std::io::Cursor<String>> {
    panic!("a current or empty catalog must not download a feed")
}

#[tokio::test]
async fn catalog_upgrade_repairs_old_facts_once_despite_current_search() {
    let sandbox = Sandbox::open("upgrade_old", &stones()).await;
    let catalog = &sandbox.catalog;
    // Reproduces production: all rows predate rules_metadata, but migration
    // has already declared the search projection current.
    catalog.migrate().await.expect("migrating");
    let ready = catalog.readiness().await.expect("readiness");
    assert!(ready.cards && ready.projection);
    let cards = authoritative_stones();
    let ids: Vec<_> = cards.iter().map(|card| card.id.clone()).collect();
    assert!(catalog.rules_of(&ids).await.expect("rules").is_empty());
    let feed = feed_of(&cards);
    assert_eq!(
        upgrade::from_feed(catalog, move || Ok(feed))
            .await
            .expect("upgrading"),
        upgrade::Outcome::Refreshed(cards.len())
    );
    let facts = catalog.rules_of(&ids).await.expect("rules");
    assert_eq!(facts.len(), cards.len());
    assert_eq!(facts[0].watermarks, vec![Some("azorius".to_owned())]);
    assert!(facts[1].is_un);
    assert_eq!(facts[2].watermarks, vec![None]);
    assert!(!facts[2].is_un);
    assert_eq!(catalog.data_version().await.expect("stamp"), 2);
    catalog.migrate().await.expect("migrating again");
    assert_eq!(
        upgrade::from_feed(catalog, no_upgrade_download)
            .await
            .expect("already current"),
        upgrade::Outcome::Current
    );
    assert_eq!(catalog.data_version().await.expect("stamp"), 2);
    sandbox.close().await;
}

#[tokio::test]
async fn catalog_upgrade_leaves_empty_installs_opt_in() {
    let sandbox = Sandbox::open("upgrade_empty", &[]).await;
    assert_eq!(
        upgrade::from_feed(&sandbox.catalog, no_upgrade_download)
            .await
            .expect("empty"),
        upgrade::Outcome::Empty
    );
    assert_eq!(sandbox.catalog.data_version().await.expect("stamp"), 0);
    sandbox.close().await;
}

#[tokio::test]
async fn catalog_upgrade_partial_feed_does_not_invent_foreign_print_facts() {
    let sandbox = Sandbox::open("upgrade_partial", &stones()).await;
    let cards = authoritative_stones();
    let feed = feed_of(&cards[..2]); // English only; old German rows stay NULL.
    let result = upgrade::from_feed(&sandbox.catalog, move || Ok(feed)).await;
    assert!(
        result
            .expect_err("incomplete")
            .to_string()
            .contains("missing rules metadata")
    );
    let ids: Vec<_> = cards[2..].iter().map(|card| card.id.clone()).collect();
    assert!(
        sandbox
            .catalog
            .rules_of(&ids)
            .await
            .expect("rules")
            .is_empty()
    );
    sandbox
        .catalog
        .migrate()
        .await
        .expect("DDL cannot repair facts");
    let feed = feed_of(&cards);
    assert_eq!(
        upgrade::from_feed(&sandbox.catalog, move || Ok(feed))
            .await
            .expect("retrying"),
        upgrade::Outcome::Refreshed(cards.len())
    );
    sandbox.close().await;
}

#[tokio::test]
async fn catalog_upgrade_failed_or_empty_feed_can_be_retried() {
    let sandbox = Sandbox::open("upgrade_failed", &stones()).await;
    let catalog = &sandbox.catalog;
    let failed = upgrade::from_feed(catalog, || -> anyhow::Result<std::io::Cursor<String>> {
        anyhow::bail!("download interrupted")
    })
    .await;
    assert!(failed.is_err());
    assert!(
        upgrade::from_feed(catalog, || Ok(std::io::Cursor::new("")))
            .await
            .is_err()
    );
    let cards = authoritative_stones();
    let feed = feed_of(&cards);
    assert_eq!(
        upgrade::from_feed(catalog, move || Ok(feed))
            .await
            .expect("retrying"),
        upgrade::Outcome::Refreshed(cards.len())
    );
    // A normal ingest also participates in completion versioning. Failure
    // must invalidate the previous success even when all facts are non-NULL.
    assert!(
        ingest::from_feed(catalog, || -> anyhow::Result<std::io::Cursor<String>> {
            anyhow::bail!("later ingest failed")
        })
        .await
        .is_err()
    );
    let feed = feed_of(&cards);
    assert_eq!(
        upgrade::from_feed(catalog, move || Ok(feed))
            .await
            .expect("retrying later ingest"),
        upgrade::Outcome::Refreshed(cards.len())
    );
    sandbox.close().await;
}

#[tokio::test]
async fn catalog_upgrade_manual_ingest_stamps_but_new_versions_refresh() {
    let sandbox = Sandbox::open("upgrade_version", &stones()).await;
    let cards = authoritative_stones();
    let feed = feed_of(&cards);
    ingest::from_feed(&sandbox.catalog, move || Ok(feed))
        .await
        .expect("manual ingest");
    assert_eq!(
        upgrade::from_feed(&sandbox.catalog, no_upgrade_download)
            .await
            .expect("current"),
        upgrade::Outcome::Current
    );
    sandbox
        .admin
        .execute_unprepared(&format!(
            "UPDATE \"{}\".catalog_meta SET value = '0' WHERE key = 'authoritative_ingest'",
            sandbox.schema
        ))
        .await
        .expect("simulating a preceding ingest version");
    let feed = feed_of(&cards);
    assert_eq!(
        upgrade::from_feed(&sandbox.catalog, move || Ok(feed))
            .await
            .expect("new version"),
        upgrade::Outcome::Refreshed(cards.len())
    );
    sandbox.close().await;
}

#[tokio::test]
async fn catalog_upgrade_refuses_a_competing_download_and_releases_its_lock() {
    use sea_orm::TransactionTrait as _;
    let sandbox = Sandbox::open("upgrade_lock", &stones()).await;
    let lock = sandbox.admin.begin().await.expect("lock transaction");
    lock.execute_unprepared(&format!(
        "SELECT pg_advisory_xact_lock(hashtextextended('{}:baylee-catalog-upgrade', 0))",
        sandbox.schema
    ))
    .await
    .expect("another deploy holds the lock");
    let err = upgrade::from_feed(&sandbox.catalog, no_upgrade_download)
        .await
        .expect_err("busy");
    assert!(err.to_string().contains("already running"));
    lock.rollback().await.expect("releasing competing lock");
    let cards = authoritative_stones();
    let feed = feed_of(&cards);
    assert_eq!(
        upgrade::from_feed(&sandbox.catalog, move || Ok(feed))
            .await
            .expect("retrying"),
        upgrade::Outcome::Refreshed(cards.len())
    );
    sandbox.close().await;
}

#[tokio::test]
async fn catalog_upgrade_interrupted_after_a_committed_batch_is_not_current() {
    struct BrokenTail;
    impl std::io::Read for BrokenTail {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("download cut off"))
        }
    }

    // One complete ingestion batch, then an I/O error: every existing row
    // gains facts, but that alone must not certify a completed download.
    let mut cards: Vec<_> = (1000..1400)
        .map(|n| stone(n, "en", "2026-01-01", "7", None))
        .collect();
    let sandbox = Sandbox::open("upgrade_interrupted", &cards).await;
    for card in &mut cards {
        card.border_color = Some("black".to_owned());
    }
    let feed = std::io::Read::chain(feed_of(&cards), BrokenTail);
    assert!(
        upgrade::from_feed(&sandbox.catalog, move || Ok(std::io::BufReader::new(feed)))
            .await
            .is_err()
    );
    let ids: Vec<_> = cards.iter().map(|card| card.id.clone()).collect();
    assert_eq!(
        sandbox
            .catalog
            .rules_of(&ids)
            .await
            .expect("committed batch")
            .len(),
        cards.len()
    );
    assert_eq!(
        sandbox.catalog.data_version().await.expect("start stamp"),
        1
    );
    let feed = feed_of(&cards);
    assert_eq!(
        upgrade::from_feed(&sandbox.catalog, move || Ok(feed))
            .await
            .expect("retrying incomplete feed"),
        upgrade::Outcome::Refreshed(cards.len())
    );
    assert_eq!(
        upgrade::from_feed(&sandbox.catalog, no_upgrade_download)
            .await
            .expect("completed"),
        upgrade::Outcome::Current
    );
    sandbox.close().await;
}

#[tokio::test]
async fn catalog_upgrade_works_with_a_single_connection_ingestion_pool() {
    let sandbox = Sandbox::open("upgrade_pool_one", &stones()).await;
    let url = std::env::var("DATABASE_URL").expect("local PostgreSQL URL");
    let sep = if url.contains('?') { '&' } else { '?' };
    let mut options = sea_orm::ConnectOptions::new(format!(
        "{url}{sep}options=-c%20search_path%3D{},public",
        sandbox.schema
    ));
    options.max_connections(1).min_connections(1);
    let db = Database::connect(options).await.expect("one-slot pool");
    let catalog = Catalog::from_connection(db.clone());
    let cards = authoritative_stones();
    let feed = feed_of(&cards);
    assert_eq!(
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            upgrade::from_feed(&catalog, move || Ok(feed))
        )
        .await
        .expect("upgrade must not deadlock its own pool")
        .expect("upgrading with one ingest connection"),
        upgrade::Outcome::Refreshed(cards.len())
    );
    assert_eq!(
        upgrade::from_feed(&catalog, no_upgrade_download)
            .await
            .expect("current"),
        upgrade::Outcome::Current
    );
    db.close().await.expect("closing one-slot pool");
    sandbox.close().await;
}
