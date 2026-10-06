//! The catalog's schema, its projection and their locks, and the reference
//! data seeded into them.

use super::*;

/// What [`Catalog::readiness`] found, and why they are two questions.
///
/// A catalog with rows in `cards` and none in `card_search` serves card text
/// and answers every search with nothing. Collapsing the pair into one
/// "healthy" bit would hide the only state that is broken without being an
/// error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Readiness {
    /// `cards` holds at least one printing — an ingest has run.
    pub cards: bool,
    /// `card_search` holds at least one row — the projection is built.
    pub projection: bool,
}

/// Holds [`PROJECT_LOCK`] for the rest of `conn`'s transaction.
///
/// `pg_advisory_xact_lock` and not the session flavour: a pooled connection
/// outlives the work, and a session lock left on one would stop every later
/// rebuild rather than this one.
pub(crate) async fn take_the_projection_lock(conn: &impl ConnectionTrait) -> Result<()> {
    conn.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1)",
        [Value::from(PROJECT_LOCK)],
    ))
    .await
    .context("taking the projection lock")?;
    Ok(())
}

/// Holds [`SCHEMA_LOCK`] for the rest of `conn`'s transaction.
///
/// The transaction flavour for the reason above it, and because the DDL it
/// guards is the same transaction: a migration that fails half way releases
/// the lock by rolling back rather than leaving every later one waiting on a
/// pooled connection nobody is using.
pub(crate) async fn take_the_schema_lock(conn: &impl ConnectionTrait) -> Result<()> {
    conn.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1)",
        [Value::from(SCHEMA_LOCK)],
    ))
    .await
    .context("taking the schema lock")?;
    Ok(())
}

/// Whether `card_search` needs rebuilding.
///
/// Asked of whatever connection is in hand, because where it is asked is what
/// makes [`PROJECT_LOCK`] worth taking: inside the locked transaction it is a
/// second gateway reading the first one's answer.
pub(crate) async fn projection_is_stale(conn: &impl ConnectionTrait) -> Result<bool> {
    let row = conn
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT (SELECT value FROM catalog_meta WHERE key = $1) AS stamped, \
                    EXISTS (SELECT 1 FROM card_search) AS projected, \
                    EXISTS (SELECT 1 FROM cards) AS ingested",
            [Value::from(VERSION_KEY.to_string())],
        ))
        .await
        .context("reading the projection's version")?
        .context("the version query returned no row")?;
    let stamped: Option<String> = row.try_get("", "stamped")?;
    let projected: bool = row.try_get("", "projected")?;
    let ingested: bool = row.try_get("", "ingested")?;
    Ok(stamped.as_deref() != Some(&SCHEMA_VERSION.to_string()) || (ingested && !projected))
}

/// Empties `card_search`, fills it, and stamps the version it was built at.
///
/// The stamp is the last statement and shares the caller's transaction, so a
/// rebuild that fails halfway leaves the old version standing and is tried
/// again on the next start.
pub(crate) async fn rebuild_the_projection(conn: &impl ConnectionTrait) -> Result<()> {
    for sql in ["TRUNCATE card_search", PROJECT_SQL] {
        conn.execute_raw(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .with_context(|| format!("projecting: {sql}"))?;
    }
    conn.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO catalog_meta (key, value) VALUES ($1, $2) \
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
        [
            Value::from(VERSION_KEY.to_string()),
            Value::from(SCHEMA_VERSION.to_string()),
        ],
    ))
    .await
    .context("stamping the projection's version")?;
    Ok(())
}

/// A comma-joined column back into a list, without empty entries.
///
/// `finishes` and `frame_effects` are short, closed sets of tags; storing them
/// as text keeps the whole crate on one `Value` type and one parameter
/// binding, and neither is ever queried *into* — only read back with the row.
///
/// `text[]` is the obvious correction and it was measured before it was
/// declined: the same 542 177 rows written with both columns as arrays make
/// `cards` **98 MB against 79 MB**, because an array's header costs more than
/// the seven distinct strings `finishes` ever holds. A `@>` nobody writes is
/// not worth 19 MB.
pub(crate) fn split_list(joined: &str) -> Vec<String> {
    joined
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// The schema, as idempotent statements.
///
/// Kept as plain DDL rather than a migration chain: the catalog is a cache of
/// someone else's data, so the recovery for any schema problem is to drop it
/// and ingest again, and a version table would only add ceremony to that.
///
/// This half is what Scryfall handed over — the printings, their faces and
/// what each format says about the card — and each table's
/// `ADD COLUMN IF NOT EXISTS` sits with it rather than in a block of its own,
/// because the question a reader has is "does this catalog have `set_type`",
/// not "which release added it".
pub(crate) fn schema_statements() -> Vec<String> {
    let mut statements = vec![
        // `WITH SCHEMA public` and not merely `IF NOT EXISTS`. A test runs the
        // whole catalog in a schema of its own, and an unqualified
        // `CREATE EXTENSION` installs into the *first* schema on the path —
        // so the first test to run would put `unaccent` in its own sandbox,
        // `IF NOT EXISTS` is database-wide and would make every later test
        // skip creating it, and dropping that sandbox takes the extension
        // with it. Demonstrated live while writing this: one
        // `DROP SCHEMA … CASCADE` removed `unaccent` from the development
        // database and the next ingest could not normalise a name.
        "CREATE EXTENSION IF NOT EXISTS unaccent WITH SCHEMA public".to_string(),
        "CREATE TABLE IF NOT EXISTS cards (
            scryfall_id      uuid PRIMARY KEY,
            oracle_id        uuid NOT NULL,
            lang             text NOT NULL,
            set_code         text NOT NULL DEFAULT '',
            collector_number text NOT NULL DEFAULT '',
            rarity           text,
            layout           text,
            released_at      date,
            set_name         text,
            artist           text,
            finishes         text NOT NULL DEFAULT 'nonfoil',
            frame_effects    text NOT NULL DEFAULT '',
            border_color     text,
            promo            boolean NOT NULL DEFAULT false,
            set_type         text,
            digital          boolean NOT NULL DEFAULT false,
            games            text NOT NULL DEFAULT '',
            updated_at       timestamptz NOT NULL DEFAULT now()
        )"
        .to_string(),
        // The printing columns arrived after the first ingests did. A catalog
        // filled before them keeps its rows and gains empty columns, which the
        // next ingest fills — dropping and re-ingesting 118k printings to add
        // a set name would be an absurd price for a picker.
        "ALTER TABLE cards \
             ADD COLUMN IF NOT EXISTS set_name text, \
             ADD COLUMN IF NOT EXISTS artist text, \
             ADD COLUMN IF NOT EXISTS finishes text NOT NULL DEFAULT 'nonfoil', \
             ADD COLUMN IF NOT EXISTS frame_effects text NOT NULL DEFAULT '', \
             ADD COLUMN IF NOT EXISTS border_color text, \
             ADD COLUMN IF NOT EXISTS promo boolean NOT NULL DEFAULT false, \
             ADD COLUMN IF NOT EXISTS set_type text, \
             ADD COLUMN IF NOT EXISTS digital boolean NOT NULL DEFAULT false, \
             ADD COLUMN IF NOT EXISTS games text NOT NULL DEFAULT ''"
            .to_string(),
        // `released_at` was stored as text, which sorted correctly only by
        // the accident that Scryfall writes ISO dates — and every one of the
        // 542 177 printings in the live catalog is well-formed, which is what
        // makes this conversion lossless rather than hopeful. It is a `date`
        // now: 5824 kB of text becomes 2118 kB, and a release date is a date.
        //
        // `current_schema()` and not a bare lookup, for the third time in
        // this file: `information_schema.columns` describes every schema on
        // the path, so a test sandbox would find `public.cards` already
        // converted and skip its own table. The same reach that took away
        // `unaccent` and deleted two live indexes.
        "DO $retype$ BEGIN \
           IF EXISTS (SELECT 1 FROM information_schema.columns \
                      WHERE table_schema = current_schema() \
                        AND table_name = 'cards' \
                        AND column_name = 'released_at' \
                        AND data_type = 'text') THEN \
             ALTER TABLE cards \
               ALTER COLUMN released_at TYPE date USING nullif(released_at, '')::date; \
           END IF; \
         END $retype$"
            .to_string(),
        "CREATE TABLE IF NOT EXISTS card_faces (
            scryfall_id       uuid NOT NULL REFERENCES cards(scryfall_id) ON DELETE CASCADE,
            face_index        smallint NOT NULL,
            name              text NOT NULL DEFAULT '',
            printed_name      text,
            flavor_name       text,
            type_line         text,
            printed_type_line text,
            oracle_text       text,
            printed_text      text,
            mana_cost         text,
            power             text,
            toughness         text,
            loyalty           text,
            PRIMARY KEY (scryfall_id, face_index)
        )"
        .to_string(),
        // `CREATE TABLE IF NOT EXISTS` does nothing to a table that exists, so
        // a catalog ingested before flavor names knew about them needs the
        // column added. The projection's version stamp then refills it.
        "ALTER TABLE card_faces ADD COLUMN IF NOT EXISTS flavor_name text".to_string(),
        // The lookup that serves every game: identity, then language.
        "CREATE INDEX IF NOT EXISTS cards_oracle_lang ON cards (oracle_id, lang)".to_string(),
        // What each format says about a card, as Scryfall's own map:
        // `{"commander": "legal", "modern": "banned", …}` over two dozen
        // formats.
        //
        // Keyed on `oracle_id` and not on a printing, which is the whole
        // reason it is its own table: legality is a property of the *card*,
        // so storing it beside a printing would keep the same answer 542 177
        // times. `jsonb` rather than a row per format, because every caller
        // wants the whole map for one card, and a GIN index still answers
        // `legalities ->> 'commander' = 'legal'` across the catalog.
        //
        // The deckbuilder is what will want it — "may this card go in this
        // deck" is a format question, and the pool it offers has had no way
        // to ask one. The corpus asks something much weaker: whether *any*
        // format has heard of the card.
        "CREATE TABLE IF NOT EXISTS card_legalities (
            oracle_id  uuid PRIMARY KEY,
            legalities jsonb NOT NULL DEFAULT '{}'::jsonb
        )"
        .to_string(),
        "CREATE INDEX IF NOT EXISTS card_legalities_gin \
             ON card_legalities USING gin (legalities)"
            .to_string(),
    ];
    statements.extend(projection_schema());
    statements
}

/// The projection the search reads, and the stamp that says which version of
/// it is stored.
///
/// The other half of [`schema_statements`], and the seam is the one the crate
/// already thinks along: everything above is what Scryfall handed over, and
/// everything here is derived from it and thrown away whenever the derivation
/// changes.
pub(crate) fn projection_schema() -> Vec<String> {
    vec![
        // What the projection is stamped with, so an upgrade can be told from
        // a fresh install.
        "CREATE TABLE IF NOT EXISTS catalog_meta (
            key   text PRIMARY KEY,
            value text NOT NULL
        )"
        .to_string(),
        format!(
            "CREATE OR REPLACE FUNCTION catalog_norm(s text) RETURNS text \
                 LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE AS $fn$ {NORM_BODY} $fn$"
        ),
        format!(
            "CREATE OR REPLACE FUNCTION catalog_bigrams(s text) RETURNS text[] \
                 LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE AS $fn$ {BIGRAMS_BODY} $fn$"
        ),
        // The search projection: one row per oracle face, every language that
        // face prints in, and the two things a search asks about.
        //
        // `bg` is `GENERATED` and `names_norm` is not, and the asymmetry is
        // the point. `catalog_bigrams` is genuinely immutable over its
        // argument, so Postgres may store its result; `catalog_norm` reaches
        // an extension's dictionary through the search path, which is a
        // promise this crate keeps rather than one the server can enforce —
        // so its result is written by `project()`, where a rebuild can
        // correct it, and never by the server behind the code's back.
        "CREATE TABLE IF NOT EXISTS card_search (
            oracle_id  uuid NOT NULL,
            face_index smallint NOT NULL,
            names      jsonb NOT NULL,
            names_norm text NOT NULL,
            tsv        tsvector NOT NULL,
            bg         text[] GENERATED ALWAYS AS (catalog_bigrams(names_norm)) STORED,
            PRIMARY KEY (oracle_id, face_index)
        )"
        .to_string(),
        "CREATE INDEX IF NOT EXISTS card_search_bg ON card_search USING gin (bg)".to_string(),
        "CREATE INDEX IF NOT EXISTS card_search_tsv ON card_search USING gin (tsv)".to_string(),
        // The two the projection replaces. Left in place they would cost an
        // existing install 124 MB to answer nothing: 74 MB of expression GIN
        // over 554 242 faces and 50 MB of trigrams over the same names the
        // projection now holds 41 991 of.
        //
        // Named through `current_schema()` and not bare, which is the one
        // asymmetry in this list. Every `CREATE … IF NOT EXISTS` above builds
        // in the schema it is run in; a bare `DROP … IF EXISTS` **resolves
        // along the whole search path** and reaches out of it. A test runs
        // the catalog in a sandbox schema with `public` behind it, so the
        // first such test dropped the developer's live indexes — observed,
        // not imagined, while writing this.
        "DO $drop$ BEGIN \
           EXECUTE format('DROP INDEX IF EXISTS %I.card_faces_search', current_schema()); \
           EXECUTE format('DROP INDEX IF EXISTS %I.card_faces_name_trgm', current_schema()); \
         END $drop$"
            .to_string(),
    ]
    .into_iter()
    .chain(reference_tables())
    .collect()
}

/// The two tables that are seeded rather than derived.
///
/// Both answer a question the printings cannot: `SELECT DISTINCT lang` gives
/// codes and not the name a person picks from, and no row anywhere says what
/// `Ally` is called in German. Both are seeded `ON CONFLICT DO NOTHING`, so a
/// self-hoster may correct a row and keep the correction across every later
/// start.
pub(crate) fn reference_tables() -> Vec<String> {
    vec![
        // Which languages exist, named in themselves. A player picking a
        // language reads its own name for it, never an English one — this is
        // the table a picker is drawn from, and it is the catalog's because
        // the catalog is what knows which of them a card is printed in.
        "CREATE TABLE IF NOT EXISTS languages (
            code text PRIMARY KEY,
            name text NOT NULL
        )"
        .to_string(),
        language_seed(),
        // What every card type and subtype is called in each language.
        //
        // Keyed on the **English name**, which is Scryfall's own key and the
        // one thing both halves of this workspace already agree on without
        // sharing a build. Not on `baylee_core::SubtypeId`: those ids are a
        // running index into one alphabetically sorted range partitioned by
        // kind, so a single new creature type renumbers every artifact,
        // enchantment, land, planeswalker and spell subtype after it — and a
        // catalog keyed that way would need 542 177 printings re-ingested
        // every time Scryfall publishes a new set.
        "CREATE TABLE IF NOT EXISTS type_names (
            english text NOT NULL,
            lang text NOT NULL,
            printed text NOT NULL,
            PRIMARY KEY (english, lang)
        )"
        .to_string(),
        "CREATE INDEX IF NOT EXISTS type_names_english ON type_names (english)".to_string(),
        type_name_seed(),
    ]
}

/// Every language Scryfall prints a card in, named in itself.
///
/// Seeded rather than derived, because `SELECT DISTINCT lang` gives codes and
/// not names, and a name is what a person picks from. `ON CONFLICT DO NOTHING`
/// so a self-hoster may correct or add a row and keep it.
pub(crate) fn language_seed() -> String {
    const LANGUAGES: [(&str, &str); 19] = [
        ("en", "English"),
        ("es", "Español"),
        ("fr", "Français"),
        ("de", "Deutsch"),
        ("it", "Italiano"),
        ("pt", "Português"),
        ("ja", "日本語"),
        ("ko", "한국어"),
        ("ru", "Русский"),
        ("zhs", "简体中文"),
        ("zht", "繁體中文"),
        ("he", "עברית"),
        ("la", "Latina"),
        ("grc", "Ἑλληνική"),
        ("ar", "العربية"),
        ("sa", "संस्कृतम्"),
        ("ph", "Phyrexian"),
        ("qya", "Quenya"),
        ("dw", "Dwarvish"),
    ];
    let rows: Vec<String> = LANGUAGES
        .iter()
        .map(|(code, name)| format!("('{code}', '{name}')"))
        .collect();
    format!(
        "INSERT INTO languages (code, name) VALUES {} ON CONFLICT (code) DO NOTHING",
        rows.join(", ")
    )
}

/// The mined dictionary, committed rather than derived at install time.
///
/// Mining it needs a *full* catalog — every language, 542 177 printings — and
/// CI has an empty Postgres, so the file is the artifact and
/// `baylee-catalog mine-types` is the developer's tool that rewrites it. Same
/// bargain as the `CardIndex` ledger.
pub(crate) const TYPE_NAMES_TSV: &str = include_str!("../../../data/type-names.tsv");

/// What each type and subtype is called, as one `INSERT`.
///
/// `ON CONFLICT DO NOTHING` so a self-hoster may correct a row and keep it,
/// exactly as [`language_seed`] allows.
pub(crate) fn type_name_seed() -> String {
    let rows: Vec<String> = TYPE_NAMES_TSV
        .lines()
        .filter_map(|line| {
            let mut cell = line.split('\t');
            let (english, lang, printed) = (cell.next()?, cell.next()?, cell.next()?);
            let q = |s: &str| s.replace('\'', "''");
            Some(format!(
                "('{}', '{}', '{}')",
                q(english),
                q(lang),
                q(printed)
            ))
        })
        .collect();
    format!(
        "INSERT INTO type_names (english, lang, printed) VALUES {} \
         ON CONFLICT (english, lang) DO NOTHING",
        rows.join(", ")
    )
}
