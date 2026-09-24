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

use baylee_catalog::{CardTextEntry, Catalog, ingest, scryfall};
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
