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
//! tables here while `public` stays on the path for the `unaccent`
//! dictionary. That last half is why the schema installs the extension
//! `WITH SCHEMA public`: an unqualified `CREATE EXTENSION` would land in
//! whichever sandbox ran first, `IF NOT EXISTS` would make every later one
//! skip it, and dropping that sandbox would take it away from all of them.
//!
//! Without `DATABASE_URL` they fail rather than skip, for the reason
//! `baylee-db`'s schema tests give: a green suite with no server says a
//! query works when nothing ran it.

use baylee_catalog::{Catalog, scryfall};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use std::sync::atomic::{AtomicU32, Ordering};

/// A catalog in a schema of its own.
struct Sandbox {
    catalog: Catalog,
    admin: DatabaseConnection,
    schema: String,
    /// A second schema on the `search_path`, behind the first.
    behind: Option<String>,
}

/// Distinguishes two schemas made in the same process in the same second.
static NTH: AtomicU32 = AtomicU32::new(0);

impl Sandbox {
    async fn open(what: &str) -> Self {
        Self::open_with(what, false).await
    }

    /// The same, with an **empty second schema behind it** on the path.
    ///
    /// That is where the hazard lives: an unqualified `DROP INDEX IF EXISTS`
    /// does not stop at the schema the migration is building in, it walks the
    /// whole path. With one schema there is nothing behind the sandbox but
    /// `public`, which a test may not write to, so the reach is unobservable.
    async fn open_behind(what: &str) -> Self {
        Self::open_with(what, true).await
    }

    async fn open_with(what: &str, behind: bool) -> Self {
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

        let behind = behind.then(|| format!("{schema}_behind"));

        let admin = Database::connect(&url).await.expect("connecting");
        for name in [Some(&schema), behind.as_ref()].into_iter().flatten() {
            admin
                .execute_unprepared(&format!("CREATE SCHEMA \"{name}\""))
                .await
                .expect("creating a schema");
        }

        let path = match &behind {
            Some(second) => format!("{schema},{second},public"),
            None => format!("{schema},public"),
        };
        let sep = if url.contains('?') { '&' } else { '?' };
        let scoped = format!("{url}{sep}options=-c%20search_path%3D{path}");
        let catalog = Catalog::connect(&scoped).await.expect("connecting scoped");
        catalog
            .migrate()
            .await
            .expect("building the catalog schema");

        Self {
            catalog,
            admin,
            schema,
            behind,
        }
    }

    /// Whether an index of that name exists in one of this sandbox's schemas.
    async fn has_index(&self, schema: &str, index: &str) -> bool {
        let row = self
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                format!("SELECT to_regclass('\"{schema}\".\"{index}\"') IS NOT NULL AS there"),
            ))
            .await
            .expect("asking whether an index is there")
            .expect("the question returned no row");
        row.try_get("", "there").expect("a boolean")
    }

    /// Upsert, then project — in that order, because the search reads the
    /// projection and `upsert` deliberately does not write it.
    async fn fill(&self, cards: &[scryfall::Card]) {
        self.catalog.upsert(cards).await.expect("upserting");
        self.catalog.project().await.expect("projecting");
    }

    async fn close(self) {
        for name in [Some(&self.schema), self.behind.as_ref()]
            .into_iter()
            .flatten()
        {
            self.admin
                .execute_unprepared(&format!("DROP SCHEMA \"{name}\" CASCADE"))
                .await
                .expect("dropping the schema");
        }
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
    sandbox.fill(&one_card_five_ways()).await;

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
    sandbox.fill(&one_card_five_ways()).await;

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
    sandbox.fill(&cards).await;

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

/// A two-character name in a script with no spaces.
///
/// This is the search that made the projection necessary. `稲妻` is Lightning
/// Bolt in Japanese, and against the live catalog it cost **2742 ms** — a
/// sequential scan over 554 242 faces building a `tsvector` for each one,
/// because the `ILIKE '%稲妻%'` half of the old predicate could not be
/// indexed and so neither half was. It is not a `pg_trgm` limitation:
/// `show_trgm('稲妻')` returns three trigrams, because whole words are
/// padded. A `%…%` *pattern* is not.
#[tokio::test]
async fn a_two_character_name_is_found_in_a_script_that_has_no_spaces() {
    let sandbox = Sandbox::open("cjk").await;
    let mut cards = one_card_five_ways();
    cards.push(printing(
        "00000000-0000-4000-8000-000000000006",
        "ja",
        "4ed",
        "1995-04-01",
        Some("稲妻"),
        Some("インスタント"),
    ));
    // A longer name that contains it, so a whole-name match has to outrank a
    // name that merely starts with the same two characters.
    cards.push(scryfall::Card {
        id: "00000000-0000-4000-8000-000000000007".to_string(),
        oracle_id: Some("bbbbbbbb-0000-4000-8000-000000000001".to_string()),
        lang: "ja".to_string(),
        set: "chk".to_string(),
        released_at: Some("2004-10-01".to_string()),
        name: "Lightning Storm".to_string(),
        printed_name: Some("稲妻の嵐".to_string()),
        type_line: Some("Instant".to_string()),
        printed_type_line: Some("インスタント".to_string()),
        ..scryfall::Card::default()
    });
    sandbox.fill(&cards).await;

    let hits = sandbox
        .catalog
        .search("稲妻", "ja", 20)
        .await
        .expect("searching");
    assert_eq!(hits.len(), 2, "both cards contain it: {hits:?}");
    assert_eq!(
        hits[0].name, "稲妻",
        "the whole name outranks the name that contains it"
    );
    assert_eq!(hits[1].name, "稲妻の嵐");

    // One character, and a whole word in Japanese.
    let one = sandbox
        .catalog
        .search("嵐", "ja", 20)
        .await
        .expect("searching for one character");
    assert_eq!(one.len(), 1, "one character is a word here: {one:?}");

    sandbox.close().await;
}

/// A player types what is on the card in front of them, and a keyboard is not
/// always the one the card was printed for.
///
/// `Ｌｉｇｈｔｎｉｎｇ` is what a Japanese IME produces in its Latin mode —
/// U+FF2C and friends, not U+004C — and `NFKC` is what folds the two
/// together. The accent half is the same rule read the other way: a player
/// who cannot type `Æ` types `ae`, and one who cannot type `é` types `e`.
#[tokio::test]
async fn a_name_is_found_however_the_keyboard_spells_it() {
    let sandbox = Sandbox::open("fold").await;
    let mut cards = one_card_five_ways();
    cards.push(scryfall::Card {
        id: "00000000-0000-4000-8000-000000000008".to_string(),
        oracle_id: Some("cccccccc-0000-4000-8000-000000000001".to_string()),
        lang: "en".to_string(),
        set: "usg".to_string(),
        released_at: Some("1998-10-12".to_string()),
        name: "Æther Flash".to_string(),
        type_line: Some("Enchantment".to_string()),
        ..scryfall::Card::default()
    });
    sandbox.fill(&cards).await;

    let wide = sandbox
        .catalog
        .search("Ｌｉｇｈｔｎｉｎｇ", "en", 20)
        .await
        .expect("searching in full-width Latin");
    assert_eq!(wide.len(), 1, "full-width Latin is the same name: {wide:?}");
    assert_eq!(wide[0].english_name, "Lightning Bolt");

    let plain = sandbox
        .catalog
        .search("aether", "en", 20)
        .await
        .expect("searching without the ligature");
    assert_eq!(plain.len(), 1, "Æther is reachable as aether: {plain:?}");
    assert_eq!(plain[0].english_name, "Æther Flash");

    sandbox.close().await;
}

/// A whole name, then a name starting with it, then a word inside a name
/// starting with it, then the query anywhere at all — and rules text last.
///
/// The old search ranked on `similarity()` alone, which put a card whose
/// *text* mentions the word beside one that is named it.
#[tokio::test]
async fn a_whole_name_outranks_a_name_that_merely_contains_it() {
    let sandbox = Sandbox::open("tier").await;
    let cards = vec![
        // Tier 3: the query is inside a word.
        contrived("d0", "Sheetlightning", None),
        // Tier 4: only the rules text says it.
        contrived("d1", "Storm Front", Some("Lightning strikes twice.")),
        // Tier 2: a word inside the name starts with it.
        contrived("d2", "Ball Lightning", None),
        // Tier 1: the name starts with it.
        contrived("d3", "Lightning Bolt", None),
        // Tier 0: the name is it.
        contrived("d4", "Lightning", None),
    ];
    sandbox.fill(&cards).await;

    let hits = sandbox
        .catalog
        .search("lightning", "en", 20)
        .await
        .expect("searching");
    let order: Vec<&str> = hits.iter().map(|h| h.name.as_str()).collect();
    assert_eq!(
        order,
        vec![
            "Lightning",
            "Lightning Bolt",
            "Ball Lightning",
            "Sheetlightning",
            "Storm Front",
        ],
        "the tiers are out of order"
    );

    sandbox.close().await;
}

/// One English card with a name and, optionally, rules text.
fn contrived(nth: &str, name: &str, text: Option<&str>) -> scryfall::Card {
    scryfall::Card {
        id: format!("00000000-0000-4000-8000-0000000002{nth}"),
        oracle_id: Some(format!("dddddddd-0000-4000-8000-0000000002{nth}")),
        lang: "en".to_string(),
        set: "tst".to_string(),
        released_at: Some("2020-01-01".to_string()),
        name: name.to_string(),
        type_line: Some("Instant".to_string()),
        oracle_text: text.map(str::to_string),
        ..scryfall::Card::default()
    }
}

/// The upgrade nobody runs a command for.
///
/// A self-hoster who pulls this version has a full `cards` and no projection
/// at all, and `CREATE TABLE IF NOT EXISTS` cannot tell that from a fresh
/// install — so a gateway would come up, build an empty `card_search`, and
/// answer every search with nothing, for ever, without logging an error.
/// `migrate()` is what has to notice.
#[tokio::test]
async fn an_upgrade_rebuilds_the_projection_without_being_asked() {
    let sandbox = Sandbox::open("upgrade").await;
    sandbox.fill(&one_card_five_ways()).await;

    // Exactly the state an upgrade arrives in: the cards are there, the
    // projection is not, and nothing says so.
    sandbox
        .admin
        .execute_unprepared(&format!(
            "TRUNCATE \"{}\".card_search; DELETE FROM \"{}\".catalog_meta",
            sandbox.schema, sandbox.schema
        ))
        .await
        .expect("emptying the projection");
    assert!(
        sandbox
            .catalog
            .search("Blitzschlag", "de", 20)
            .await
            .expect("searching an empty projection")
            .is_empty(),
        "the premise: an empty projection answers nothing"
    );

    sandbox.catalog.migrate().await.expect("migrating again");

    let hits = sandbox
        .catalog
        .search("Blitzschlag", "de", 20)
        .await
        .expect("searching after the rebuild");
    assert_eq!(hits.len(), 1, "migrate() rebuilt it: {hits:?}");
    assert_eq!(hits[0].name, "Blitzschlag");

    sandbox.close().await;
}

/// The migration retires the old expression indexes **in its own schema** and
/// reaches no further.
///
/// Observed, not imagined. `DROP INDEX IF EXISTS card_faces_search` resolves
/// along the whole `search_path`, so a test run against
/// `search_path = <sandbox>, public` found nothing of that name in its
/// sandbox, walked on, and deleted the developer's live index out of
/// `public`. Both halves are asserted here, because dropping nothing at all
/// would also pass half of it: the index in the schema being migrated is
/// gone, and the one in the schema *behind* it on the path is untouched.
#[tokio::test]
async fn a_migration_retires_an_index_only_in_the_schema_it_is_building() {
    let sandbox = Sandbox::open_behind("drop").await;
    let here = sandbox.schema.clone();
    let behind = sandbox.behind.clone().expect("a schema behind this one");

    // A `card_faces_search` in each: the one this migration owns, and one it
    // must not be able to see.
    sandbox
        .admin
        .execute_unprepared(&format!(
            "CREATE INDEX card_faces_search ON \"{here}\".card_faces (name); \
             CREATE TABLE \"{behind}\".card_faces (name text); \
             CREATE INDEX card_faces_search ON \"{behind}\".card_faces (name)"
        ))
        .await
        .expect("planting both indexes");
    assert!(sandbox.has_index(&here, "card_faces_search").await);
    assert!(sandbox.has_index(&behind, "card_faces_search").await);

    sandbox.catalog.migrate().await.expect("migrating again");

    assert!(
        !sandbox.has_index(&here, "card_faces_search").await,
        "the migration's own schema keeps a retired index"
    );
    assert!(
        sandbox.has_index(&behind, "card_faces_search").await,
        "the migration reached past its own schema and dropped a stranger's index"
    );

    sandbox.close().await;
}

/// A card nobody can be shown does not eat a place in the limit.
///
/// `card_search` holds one row per card across *all* languages, and the join
/// back to a printing keeps only the asked-for language or English — so a
/// card printed in neither would be ranked, counted against the `LIMIT`, and
/// then dropped on the way out, handing a caller who asked for twenty
/// nineteen. Eight cards in the live catalog are like that (the Japanese
/// Dreamcast promos), few enough that it would have read as a search that
/// simply found less. Here the unshowable card also ranks *first*, so a limit
/// of one is the whole test: it comes back with the card that can be drawn.
#[tokio::test]
async fn a_card_printed_in_no_language_the_player_reads_is_not_counted() {
    let sandbox = Sandbox::open("unshowable").await;
    sandbox
        .fill(&[
            scryfall::Card {
                id: "00000000-0000-4000-8000-000000000301".to_string(),
                oracle_id: Some("eeeeeeee-0000-4000-8000-000000000301".to_string()),
                lang: "ja".to_string(),
                set: "psdg".to_string(),
                released_at: Some("1999-01-01".to_string()),
                name: "Blitz".to_string(),
                type_line: Some("Instant".to_string()),
                ..scryfall::Card::default()
            },
            scryfall::Card {
                id: "00000000-0000-4000-8000-000000000302".to_string(),
                oracle_id: Some("eeeeeeee-0000-4000-8000-000000000302".to_string()),
                lang: "en".to_string(),
                set: "tst".to_string(),
                released_at: Some("2020-01-01".to_string()),
                name: "Blitz Bolt".to_string(),
                type_line: Some("Instant".to_string()),
                ..scryfall::Card::default()
            },
        ])
        .await;

    // `Blitz` is the whole of the Japanese card's name — tier 0, ahead of the
    // English one — and it is the English one that has to come back.
    let hits = sandbox
        .catalog
        .search("Blitz", "en", 1)
        .await
        .expect("searching in English");
    assert_eq!(hits.len(), 1, "a limit of one answered with: {hits:?}");
    assert_eq!(hits[0].name, "Blitz Bolt");

    // It is a filter on what this player can be shown, not a card the catalog
    // has stopped keeping.
    let both = sandbox
        .catalog
        .search("Blitz", "ja", 20)
        .await
        .expect("searching in Japanese");
    assert_eq!(both.len(), 2, "a Japanese player sees both: {both:?}");

    sandbox.close().await;
}
