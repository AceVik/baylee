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

    /// What Postgres says a column in this sandbox's own schema is.
    async fn column_type(&self, table: &str, column: &str) -> String {
        let row = self
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                format!(
                    "SELECT data_type FROM information_schema.columns \
                     WHERE table_schema = '{}' AND table_name = '{table}' \
                       AND column_name = '{column}'",
                    self.schema
                ),
            ))
            .await
            .expect("asking what a column is")
            .expect("no such column");
        row.try_get("", "data_type").expect("a type name")
    }

    /// What a single-number query in this sandbox's own schema answers, with
    /// a literal `{s}` in the SQL standing for the schema's name.
    ///
    /// Qualified rather than left bare for the reason the module header
    /// gives: an unqualified name walks the `search_path`, and a developer's
    /// `public` holds half a gigabyte of ingested card text that would answer
    /// instead. The count comes back as `n`.
    async fn count(&self, sql: &str) -> i64 {
        let row = self
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                sql.replace("{s}", &self.schema),
            ))
            .await
            .expect("counting")
            .expect("the question returned no row");
        row.try_get("", "n").expect("a count")
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

/// A release date is a date, and a catalog that stored it as text converts.
///
/// Three things have to hold at once and none of them fails at compile time.
/// The bind is a `String` going into a `date` column, so the placeholder has
/// to cast; `printings()` renders the column back to the ISO string its wire
/// shape promises, which `to_char` does and `::text` only does on a server
/// whose `DateStyle` happens to be ISO; and an install that predates the
/// change has to convert itself, so the column is put back to `text` here and
/// `migrate()` is asked to find it.
#[tokio::test]
async fn a_release_date_is_stored_as_a_date_and_an_old_catalog_converts() {
    let sandbox = Sandbox::open_behind("retype").await;
    let mut cards = one_card_five_ways();
    // Scryfall omits `released_at` on a printing that has no release date —
    // a null, which the bind has to tell apart from a date it cannot parse.
    cards.push(scryfall::Card {
        released_at: None,
        ..printing(
            "00000000-0000-4000-8000-000000000006",
            "en",
            "sld",
            "",
            None,
            None,
        )
    });
    sandbox.fill(&cards).await;

    assert_eq!(sandbox.column_type("cards", "released_at").await, "date");
    let iso = |hits: &[baylee_catalog::Printing]| {
        let mut dates: Vec<String> = hits.iter().map(|p| p.released_at.clone()).collect();
        dates.sort();
        dates.dedup();
        dates
    };
    let before = sandbox
        .catalog
        .printings(&[BOLT.to_string()])
        .await
        .expect("listing printings");
    assert_eq!(
        iso(&before),
        vec![
            String::new(),
            "1993-08-05".to_string(),
            "2009-07-17".to_string(),
            "2010-07-16".to_string(),
            "2022-07-08".to_string(),
            "2025-11-21".to_string(),
        ],
        "the wire wants ISO whatever the server's DateStyle is, \
         and an empty string for a printing that has no date"
    );

    // What an install from before this change looks like. Asserted, because a
    // conversion test that starts already converted proves nothing.
    sandbox
        .admin
        .execute_unprepared(&format!(
            "ALTER TABLE \"{}\".cards ALTER COLUMN released_at TYPE text \
             USING to_char(released_at, 'YYYY-MM-DD')",
            sandbox.schema
        ))
        .await
        .expect("putting the column back to text");
    assert_eq!(sandbox.column_type("cards", "released_at").await, "text");

    sandbox.catalog.migrate().await.expect("migrating again");

    assert_eq!(
        sandbox.column_type("cards", "released_at").await,
        "date",
        "migrate() left an older catalog on text"
    );
    let after = sandbox
        .catalog
        .printings(&[BOLT.to_string()])
        .await
        .expect("listing printings after the conversion");
    assert_eq!(iso(&after), iso(&before), "a date was lost in the crossing");

    // And once more, with an *older* catalog sitting behind this one on the
    // path — which is what makes the guard's `current_schema()` load-bearing
    // rather than decorative. Without it the guard reads
    // `information_schema.columns` for every schema on the path, sees the
    // stranger's `text`, and runs `ALTER … USING nullif(released_at, '')` on
    // a column that is already a `date`: `invalid input syntax for type
    // date: ""`, and every migration against this server fails until the
    // stranger is converted too.
    let behind = sandbox.behind.clone().expect("a schema behind this one");
    sandbox
        .admin
        .execute_unprepared(&format!(
            "CREATE TABLE \"{behind}\".cards (released_at text)"
        ))
        .await
        .expect("planting an older catalog behind this one");

    sandbox
        .catalog
        .migrate()
        .await
        .expect("migrating with a stranger's text column on the path");
    assert_eq!(sandbox.column_type("cards", "released_at").await, "date");

    sandbox.close().await;
}

/// A card nobody ever printed in German is still found by a German word.
///
/// This is the half the projection could not do on its own. Merging every
/// *printed* type line into one `tsvector` already makes `同盟者` and `Ally`
/// reach the same cards — but only cards somebody printed that way, and 3634
/// cards in the live catalog have no German printing at all. Here the card
/// exists only in English, and a German player typing the German word for its
/// subtype has to find it anyway.
///
/// It is also what pins `data/type-names.tsv` to the build: the assertion
/// passes only if the seeded dictionary carries `Ally` in German, so a
/// migration that stopped shipping the file fails here rather than silently
/// answering less.
#[tokio::test]
async fn a_card_printed_in_no_german_is_found_by_a_german_type_word() {
    let sandbox = Sandbox::open("translated").await;
    sandbox
        .fill(&[scryfall::Card {
            id: "00000000-0000-4000-8000-000000000401".to_string(),
            oracle_id: Some("eeeeeeee-0000-4000-8000-000000000401".to_string()),
            lang: "en".to_string(),
            set: "zen".to_string(),
            released_at: Some("2009-10-02".to_string()),
            name: "Hagra Diabolist".to_string(),
            type_line: Some("Creature — Human Shaman Ally".to_string()),
            ..scryfall::Card::default()
        }])
        .await;

    for (word, what) in [
        ("Verbündeter", "the subtype"),
        ("Schamane", "a second subtype"),
        ("Kreatur", "the card type"),
    ] {
        let hits = sandbox
            .catalog
            .search(word, "de", 20)
            .await
            .expect("searching in German");
        assert_eq!(
            hits.len(),
            1,
            "{what} `{word}` found: {hits:?} — is it in data/type-names.tsv?"
        );
        assert_eq!(hits[0].name, "Hagra Diabolist");
    }

    // The English words keep working, and the card is still answered as the
    // English printing it is — translating the *index* translates no text.
    let english = sandbox
        .catalog
        .search("Ally", "de", 20)
        .await
        .expect("searching in English");
    assert_eq!(english.len(), 1, "the English word still finds it");
    assert_eq!(english[0].lang, "en");
    assert_eq!(english[0].type_line, "Creature — Human Shaman Ally");

    sandbox.close().await;
}

/// A type name a self-hoster corrected is not overwritten by the next start.
///
/// `migrate` is run on every gateway start, so the seed runs again every
/// time. `ON CONFLICT DO NOTHING` is what makes that harmless, and it is the
/// same promise `languages` makes — a row is a starting point, not a
/// setting the catalog keeps resetting.
#[tokio::test]
async fn a_corrected_type_name_survives_the_next_migration() {
    let sandbox = Sandbox::open("corrected").await;
    sandbox
        .admin
        .execute_unprepared(&format!(
            "UPDATE \"{}\".type_names SET printed = 'Gefährte' \
             WHERE english = 'Ally' AND lang = 'de'",
            sandbox.schema
        ))
        .await
        .expect("correcting a name");

    sandbox.catalog.migrate().await.expect("migrating again");

    let row = sandbox
        .admin
        .query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            format!(
                "SELECT printed FROM \"{}\".type_names \
                 WHERE english = 'Ally' AND lang = 'de'",
                sandbox.schema
            ),
        ))
        .await
        .expect("reading the corrected name")
        .expect("the row is still there");
    let printed: String = row.try_get("", "printed").expect("the printed column");
    assert_eq!(printed, "Gefährte");

    sandbox.close().await;
}

/// A Secret Lair prints a different name on the card and the real one in
/// small type beside it. The rules never see it — `oracle_id` and the Oracle
/// name are unchanged — but a player holding the cardboard reads the printed
/// one, so the search has to answer it, and answer it as a **name**.
///
/// The decoy is what makes that second half a claim rather than a hope. A
/// flavor name reaches `tsv` as well as `names_norm`, so a card found only
/// through the text tier still comes back — ranked beside every card whose
/// rules text happens to mention the words, and behind a shorter one. Only
/// the fenced `names_norm` entry puts it at the tier a name belongs to.
///
/// Both printings are asked about, because a flavor name belongs to the
/// printing and not the card: Command Tower carries six of them, and a
/// projection that kept one printing per language would answer the first
/// query and silently fail the second.
#[tokio::test]
async fn a_card_is_found_by_the_name_a_secret_lair_printed_on_it() {
    let sandbox = Sandbox::open("flavor").await;
    let tower = "dddddddd-0000-4000-8000-000000000001";
    let printing = |id: &str, num: &str, flavor: &str| scryfall::Card {
        id: id.to_string(),
        oracle_id: Some(tower.to_string()),
        lang: "en".to_string(),
        set: "sld".to_string(),
        collector_number: num.to_string(),
        released_at: Some("2023-01-01".to_string()),
        name: "Command Tower".to_string(),
        type_line: Some("Land".to_string()),
        flavor_name: Some(flavor.to_string()),
        ..scryfall::Card::default()
    };
    sandbox
        .fill(&[
            printing("00000000-0000-4000-8000-0000000000a1", "1", "Cybertron"),
            printing("00000000-0000-4000-8000-0000000000a2", "2", "Croft Manor"),
            // Shorter name, same words in its rules text: it wins every
            // tie-break the ranking has left once the tiers are equal.
            scryfall::Card {
                id: "00000000-0000-4000-8000-0000000000a3".to_string(),
                oracle_id: Some("dddddddd-0000-4000-8000-000000000002".to_string()),
                lang: "en".to_string(),
                set: "usg".to_string(),
                released_at: Some("1998-10-12".to_string()),
                name: "Decoy".to_string(),
                type_line: Some("Instant".to_string()),
                oracle_text: Some("Cybertron and Croft Manor are not places.".to_string()),
                ..scryfall::Card::default()
            },
        ])
        .await;

    for printed in ["Cybertron", "Croft Manor"] {
        let hits = sandbox
            .catalog
            .search(printed, "en", 20)
            .await
            .expect("searching by a printed name");
        assert_eq!(
            hits.first().map(|h| h.english_name.as_str()),
            Some("Command Tower"),
            "{printed} is a name, and did not outrank a card that merely says it: {hits:?}"
        );
        assert!(
            hits.iter().any(|h| h.english_name == "Decoy"),
            "{printed} stopped reaching the text tier at all: {hits:?}"
        );
    }

    // The counter-test: a name nobody printed still finds nothing, so the
    // join widened the search rather than loosening it.
    let none = sandbox
        .catalog
        .search("Autobot City", "en", 20)
        .await
        .expect("searching for a name that was never printed");
    assert!(none.is_empty(), "an unprinted name matched: {none:?}");

    sandbox.close().await;
}

/// A reversible card is one piece of cardboard printed with the same card on
/// both sides, and Scryfall puts its `oracle_id` on the faces rather than at
/// the top. The catalog asked only the top level, so **every** such printing
/// was dropped in silence: `ecl` ran 345, 346, then nothing until 352, and
/// the five missing numbers are the shocklands in the borderless treatment.
/// `1 Hallowed Fountain (ECL) 347` in `data/decks/allytifact.txt` resolved to
/// no card at all (#46).
///
/// Two things are asserted, because fixing the first would be easy to do by
/// storing the card twice: the printing is **there**, and the card is still
/// one row in the projection. `card_search` is keyed on
/// `(oracle_id, face_index)`, so a second face would answer every search for
/// Hallowed Fountain a second time.
#[tokio::test]
async fn a_reversible_printing_is_stored_once_beside_the_ordinary_one() {
    let sandbox = Sandbox::open("reversible").await;
    let fountain = "f1750962-a87c-49f6-b731-02ae971ac6ea";
    let face = |flavor: Option<&str>| scryfall::Face {
        name: "Hallowed Fountain".to_string(),
        oracle_id: Some(fountain.to_string()),
        type_line: Some("Land \u{2014} Plains Island".to_string()),
        oracle_text: Some("As this land enters, you may pay 2 life.".to_string()),
        // Invented: the real ECL 347 prints no flavor name, and the one
        // printing in the world whose halves carry different ones is Birds
        // of Paradise SLD 1675. What is being asked here is the mechanism —
        // a name on the half that is collapsed away has to reach the row
        // that is kept, or the collapse costs a search.
        flavor_name: flavor.map(str::to_string),
        ..scryfall::Face::default()
    };
    sandbox
        .fill(&[
            // The ordinary printing: id at the top, one face, nothing new.
            scryfall::Card {
                id: "00000000-0000-4000-8000-0000000000b1".to_string(),
                oracle_id: Some(fountain.to_string()),
                lang: "en".to_string(),
                set: "ecl".to_string(),
                collector_number: "265".to_string(),
                released_at: Some("2026-01-01".to_string()),
                name: "Hallowed Fountain".to_string(),
                type_line: Some("Land \u{2014} Plains Island".to_string()),
                oracle_text: Some("As this land enters, you may pay 2 life.".to_string()),
                ..scryfall::Card::default()
            },
            // The borderless one, exactly as Scryfall hands it over.
            scryfall::Card {
                id: "00000000-0000-4000-8000-0000000000b2".to_string(),
                oracle_id: None,
                lang: "en".to_string(),
                set: "ecl".to_string(),
                collector_number: "347".to_string(),
                released_at: Some("2026-01-01".to_string()),
                name: "Hallowed Fountain // Hallowed Fountain".to_string(),
                layout: Some("reversible_card".to_string()),
                card_faces: Some(vec![face(None), face(Some("Sacred Spring"))]),
                ..scryfall::Card::default()
            },
        ])
        .await;

    assert_eq!(
        sandbox
            .count("SELECT count(*) AS n FROM \"{s}\".cards WHERE layout = 'reversible_card'")
            .await,
        1,
        "the reversible printing was dropped"
    );
    assert_eq!(
        sandbox
            .count("SELECT count(*) AS n FROM \"{s}\".card_search")
            .await,
        1,
        "one card, one projected row"
    );

    let hits = sandbox
        .catalog
        .search("Hallowed Fountain", "en", 20)
        .await
        .expect("searching");
    assert_eq!(hits.len(), 1, "the card answered twice: {hits:?}");
    assert_eq!(hits[0].english_name, "Hallowed Fountain");

    // The name printed on the half that was collapsed away still finds it,
    // which is what stops the collapse from quietly costing a search.
    let flavor = sandbox
        .catalog
        .search("Sacred Spring", "en", 20)
        .await
        .expect("searching by the name on the other half");
    assert_eq!(
        flavor.first().map(|h| h.english_name.as_str()),
        Some("Hallowed Fountain"),
        "the flavor name went with the row: {flavor:?}"
    );

    sandbox.close().await;
}
