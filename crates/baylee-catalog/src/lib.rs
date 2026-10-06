//! baylee-catalog — every card printing Scryfall knows, stored once and
//! served in the player's language.
//!
//! # Why the gateway owns this and the engine does not
//!
//! Rules text is presentation. The engine identifies a card by `CardIndex` and
//! a printing by `PrintRef`, and never reads either as prose — carrying
//! hundreds of megabytes of localized text through a rules kernel would cost
//! memory in the one process that has to stay fast and deterministic.
//!
//! So text takes the same route as card images: Scryfall → gateway → client,
//! cached at each hop. The gateway is where it stops being a stream of JSON and
//! becomes a queryable catalog, because the deck builder needs to *search* it,
//! not just look cards up by id.
//!
//! # Why hand-written SQL here and entities next door
//!
//! The whole value of this crate is in one projection and two queries: a
//! lateral join that resolves "the same card in my language", and a search
//! that has to stay off a sequential scan over half a million printings.
//! Both are shaped by the query planner rather than by the entity model, so
//! they are written as SQL and `SeaORM` supplies the pool, the parameter
//! binding and the backend abstraction.
//!
//! [`baylee_db`] is the same database and the opposite case, and the two
//! together are what the choice actually looks like: two dozen small,
//! ordinary reads and writes on six tables, where what is worth having is
//! that a column's Rust type and its Postgres type cannot drift apart. So
//! that one has entities and this one has statements, and neither is the
//! house style — the question is whether the planner or the type checker is
//! the thing you are arguing with.
//!
//! They own their schemas separately for the same reason. These tables are
//! rebuilt wholesale by an ingest and are `CREATE TABLE IF NOT EXISTS`; those
//! are migrated in place and have to survive the data in them. One migrator
//! over both would have to pretend that is one lifecycle.
//!
//! # Legal
//!
//! `docs/legal.md` §3: Scryfall encourages caching and publishes bulk data for
//! exactly this. No images are stored here — only card data — and clients keep
//! the "data provided by Scryfall" attribution.

#![warn(missing_docs)]

pub mod ingest;
mod rows;
mod schema;
pub mod scryfall;
mod sql;

use anyhow::{Context, Result};
#[cfg(test)]
use rows::{CARD_INSERT_COLUMNS, FACE_INSERT_COLUMNS, FaceRow, MAX_BIND_PARAMS};
#[cfg(test)]
use schema::language_seed;
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, TransactionTrait, Value,
};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// What `/catalog/text` answers: the one wire shape both ends link, from
/// the crate that also holds the rule a printing is picked by.
pub use baylee_cardtext::{CardTextEntry, FaceText};
use baylee_cardtext::{TextFace, TextPrinting, card_entry};
use rows::{
    CARD_COLUMNS, FACE_COLUMNS, LEGALITY_COLUMNS, cards_statement, face_rows, faces_statement,
    legalities_statement, rows_per_statement,
};
pub use schema::Readiness;
use schema::{
    projection_is_stale, rebuild_the_projection, schema_statements, split_list,
    take_the_projection_lock, take_the_schema_lock,
};
use sql::{
    BIGRAMS_BODY, CORPUS_SQL, MINE_ROUNDS, NORM_BODY, PROJECT_LOCK, PROJECT_SQL, SCHEMA_LOCK,
    TEXT_SQL, search_sql,
};

/// One search result, for the deck builder.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SearchHit {
    /// Printing id.
    pub scryfall_id: String,
    /// Language of the printing.
    pub lang: String,
    /// Name in that language.
    pub name: String,
    /// English name.
    pub english_name: String,
    /// Type line in that language.
    pub type_line: String,
}

/// One printing, as a picker has to show it.
///
/// Everything here is *about the piece of cardboard*, not about the card: two
/// `Printing`s with the same `oracle_id` are the same card in the rules and
/// two different things to own. The deck row that comes out of a pick is
/// `set` + `collector_number` + `lang` + `finish`, and `scryfall_id` pins it
/// exactly when the player wants no ambiguity at all.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Printing {
    /// Printing id — the deck row's `scryfall=` form, and the art key.
    pub scryfall_id: String,
    /// Rules identity, shared with every other printing of this card.
    pub oracle_id: String,
    /// Language of this printing.
    pub lang: String,
    /// Set code, as a deck row writes it.
    pub set: String,
    /// Full set name.
    pub set_name: String,
    /// Collector number within the set.
    pub collector_number: String,
    /// Rarity.
    pub rarity: String,
    /// Release date, ISO-8601.
    pub released_at: String,
    /// Illustrator.
    pub artist: String,
    /// Finishes this printing was sold in: `nonfoil`, `foil`, `etched`.
    pub finishes: Vec<String>,
    /// Frame treatments (`showcase`, `extendedart`, …).
    pub frame_effects: Vec<String>,
    /// Border color, `borderless` included.
    pub border_color: String,
    /// Whether it is a promo.
    pub promo: bool,
    /// Front-face name in this printing's language.
    pub name: String,
    /// Printed layout, used to distinguish true back images from split faces.
    #[serde(default)]
    pub layout: String,
}

/// A card's name in one language.
///
/// The deck builder searches in any language but shows one row per card, so
/// it needs every name a card answers to without needing a row per printing:
/// a few thousand strings for the whole registry, against a hundred thousand
/// printings.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct LocalName {
    /// Which card.
    pub oracle_id: String,
    /// Which language.
    pub lang: String,
    /// The name printed on it.
    pub name: String,
}

/// A connection to the card catalog.
#[derive(Clone, Debug)]
pub struct Catalog {
    db: DatabaseConnection,
}

/// What [`Catalog::project`] writes, as a number this code owns.
///
/// `CREATE TABLE IF NOT EXISTS` cannot tell a fresh install from an upgrade:
/// a self-hoster who pulls this version has a full `cards` and an empty
/// `card_search`, and a search over an empty projection answers nothing, for
/// ever, without erroring. So the projection carries a version and
/// [`Catalog::migrate`] rebuilds whenever it does not match. That is also the
/// only thing that catches a changed [`BIGRAMS_BODY`]: `CREATE OR REPLACE`
/// leaves the arrays already in the stored column exactly as they were.
///
/// Bump it whenever the projection's *content* changes — the fill query, one
/// of the two function bodies, or a column's meaning. 4 is #35: `names->>'en'`
/// became the Oracle name rather than the newest printing's styling.
const SCHEMA_VERSION: i32 = 4;

/// The key `SCHEMA_VERSION` is stamped under.
const VERSION_KEY: &str = "search_projection";

/// The key [`Catalog::data_version`] is stamped under.
const DATA_VERSION_KEY: &str = "data_version";

impl Catalog {
    /// Connects to Postgres.
    ///
    /// # Errors
    /// When the URL is unusable or the server refuses the connection.
    pub async fn connect(url: &str) -> Result<Self> {
        let db = Database::connect(url)
            .await
            .context("connecting to the card catalog")?;
        Ok(Self { db })
    }

    /// Share a connection the caller already has.
    ///
    /// The gateway opens exactly one pool and hands it here, rather than
    /// dialling a second time with the same URL. Two pools against one
    /// server is twice the backend processes for no more concurrency, and it
    /// is what made a test suite of three dozen gateways ask a stock
    /// PostgreSQL for more connections than it has. It also keeps both halves
    /// on one `search_path`, which is what lets a test put the whole gateway
    /// in a schema of its own.
    #[must_use]
    pub fn from_connection(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Creates the schema if it is not there yet, and rebuilds the search
    /// projection if it is stale.
    ///
    /// Idempotent, so it is safe to run on every gateway start; the ingest
    /// calls it too, so a fresh database needs no separate migration step.
    ///
    /// The second half is the one that is easy to leave out. DDL alone brings
    /// an upgrading install an *empty* `card_search` beside a full `cards`,
    /// and nothing about that is an error — the search simply answers nothing
    /// until somebody re-ingests half a gigabyte. So a projection that is
    /// empty while `cards` is not, or one stamped at a different
    /// [`SCHEMA_VERSION`], is rebuilt here.
    ///
    /// # Errors
    /// When a statement fails — most often a missing `unaccent` extension on
    /// a server where the role may not create extensions.
    pub async fn migrate(&self) -> Result<()> {
        let tx = self
            .db
            .begin()
            .await
            .context("opening the schema transaction")?;
        take_the_schema_lock(&tx).await?;
        for sql in schema_statements() {
            tx.execute_raw(Statement::from_string(DbBackend::Postgres, sql.clone()))
                .await
                .with_context(|| format!("applying schema statement: {sql}"))?;
        }
        tx.commit().await.context("committing the schema")?;
        self.rebuild_if_stale().await
    }

    /// Rebuilds the projection if it is stale, and only ever once.
    ///
    /// The question and the answer are one transaction, which is the whole of
    /// [`PROJECT_LOCK`]'s purpose: a second gateway blocks on the lock, then
    /// asks a staleness question that the first one has already answered by
    /// stamping the version. Asking outside the lock and rebuilding inside it
    /// would let both of them through.
    async fn rebuild_if_stale(&self) -> Result<()> {
        let tx = self
            .db
            .begin()
            .await
            .context("opening the projection transaction")?;
        take_the_projection_lock(&tx).await?;
        if !projection_is_stale(&tx).await? {
            tx.rollback()
                .await
                .context("releasing the projection lock")?;
            return Ok(());
        }
        rebuild_the_projection(&tx).await?;
        tx.commit().await.context("committing the projection")?;
        self.analyze_projection().await
    }

    /// Rebuilds `card_search` from `cards` and `card_faces`.
    ///
    /// Wholesale rather than incrementally, and that is the point: one row of
    /// the projection is a `GROUP BY` over every printing of one card in
    /// every language, so a batch of four hundred printings touches rows it
    /// cannot enumerate without reading the others back anyway. An ingest is
    /// dozens of batches and one projection — twenty-one seconds on a full
    /// 542 177-printing catalog, against the three minutes the ingest itself
    /// takes — so this is called once at the end, never per batch.
    ///
    /// Unconditional, unlike [`Self::rebuild_if_stale`]: an ingest has just
    /// changed what the projection is built from, and the version it is
    /// stamped at says nothing about that.
    ///
    /// # Errors
    /// When a statement fails.
    pub async fn project(&self) -> Result<()> {
        let tx = self
            .db
            .begin()
            .await
            .context("opening the projection transaction")?;
        take_the_projection_lock(&tx).await?;
        rebuild_the_projection(&tx).await?;
        tx.commit().await.context("committing the projection")?;
        self.analyze_projection().await
    }

    /// Reads what each type and subtype is called, out of the printings.
    ///
    /// The developer's half of `data/type-names.tsv`: run it against a full
    /// catalog and commit what comes out. It answers with the file's own
    /// contents — `english\tlang\tprinted`, sorted — so the caller writes and
    /// the diff is readable.
    ///
    /// Everything it needs is already on the row. Scryfall gives every
    /// foreign face its English `type_line` beside the `printed_type_line`,
    /// so a pair is one row and no join through the English printing is
    /// needed — which is also why the eight cards with no English printing
    /// are no trouble here.
    ///
    /// Three rounds, and the second is where most languages stop being
    /// guesswork. See [`MINE_ROUNDS`] for what each one claims.
    ///
    /// # Errors
    /// When a statement fails, most often because the catalog holds only the
    /// English feed and there is nothing to read.
    pub async fn mine_type_names(&self) -> Result<String> {
        let tx = self.db.begin().await.context("opening the mining scan")?;
        for (round, sql) in MINE_ROUNDS.iter().enumerate() {
            tx.execute_raw(Statement::from_string(DbBackend::Postgres, *sql))
                .await
                .with_context(|| format!("mining round {round}"))?;
        }
        let rows = tx
            .query_all_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT english, lang, printed FROM mined ORDER BY english, lang",
            ))
            .await
            .context("reading the mined dictionary")?;
        let mut out = String::new();
        for row in &rows {
            let english: String = row.try_get("", "english")?;
            let lang: String = row.try_get("", "lang")?;
            let printed: String = row.try_get("", "printed")?;
            let _ = writeln!(out, "{english}\t{lang}\t{printed}");
        }
        tx.rollback().await.context("closing the mining scan")?;
        Ok(out)
    }

    /// Every card in the corpus, first appearance first, as a TSV.
    ///
    /// A developer's tool and not an install step, like
    /// [`Self::mine_type_names`]: it needs a catalog ingested in every
    /// language, and what it writes is the input `cargo xtask ledger` turns
    /// into the `CardIndex` ledger. Nothing at runtime reads it.
    ///
    /// Columns are `oracle_id`, `released_at`, `set_code`, `name`. The
    /// assignment itself happens in codegen, which owns the ledger's format
    /// and the slug rule — this half only says which cards there are and in
    /// which order, which is the half that needs a database.
    ///
    /// The name is the whole card's — `Fire // Ice`, not `Fire` — because
    /// that is what a decklist writes and what the ledger has always
    /// recorded. The constant is named after the front face, and codegen
    /// splits it off, because that is the face the file and the registry are
    /// keyed on.
    ///
    /// `keep` is `data/corpus-keep.tsv`'s oracle ids — cards admitted whatever
    /// the filter says, because this repo implements them. See [`CORPUS_SQL`].
    ///
    /// # Errors
    /// When the scan fails or a row is missing a column.
    pub async fn card_corpus(&self, keep: &[String]) -> Result<String> {
        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                CORPUS_SQL,
                [keep.join(",").into()],
            ))
            .await
            .context("scanning the card corpus")?;
        let mut out = String::with_capacity(rows.len() * 80);
        for row in &rows {
            let oracle_id: String = row.try_get("", "oracle_id")?;
            let released_at: String = row.try_get("", "released_at")?;
            let set_code: String = row.try_get("", "set_code")?;
            let name: String = row.try_get("", "name")?;
            let _ = writeln!(out, "{oracle_id}\t{released_at}\t{set_code}\t{name}");
        }
        Ok(out)
    }

    /// Gives the planner the row counts the rebuild just changed.
    async fn analyze_projection(&self) -> Result<()> {
        self.db
            .execute_unprepared("ANALYZE card_search")
            .await
            .context("analyzing the projection")?;
        Ok(())
    }

    /// Card text for a set of cards in a language: one entry per card the
    /// catalog knows, in `oracle_id` order.
    ///
    /// Which printing speaks for a card is [`baylee_cardtext::pick`]'s answer
    /// over every printing in `lang`, the rule a client applies to what
    /// Scryfall tells it when there is no gateway. The query only gathers:
    /// every printing in the language, plus the newest English one for the
    /// Oracle and the English names. It used to choose as well —
    /// `ORDER BY (lang = $2) DESC, released_at DESC LIMIT 1` — and served the
    /// newest printing whatever it said: a German row Scryfall never
    /// translated came back as German, a `NULL` came back as English, and a
    /// tie was settled by the order Postgres read the rows in.
    ///
    /// Every face of an entry comes from one printing, so a modal
    /// double-faced card is never drawn from two. Where no printing of the
    /// card translated anything, the newest printing in the language still
    /// names it (a German name over the Oracle's text is what that printing
    /// is), and where there is none, the newest English printing does.
    ///
    /// # Errors
    /// When the query fails.
    pub async fn text_by_card(
        &self,
        oracle_ids: &[String],
        lang: &str,
    ) -> Result<Vec<CardTextEntry>> {
        if oracle_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                TEXT_SQL,
                [
                    Value::from(oracle_ids.join(",")),
                    Value::from(lang.to_string()),
                ],
            ))
            .await
            .context("looking up card text")?;

        let mut printings: Vec<TextPrinting> = Vec::new();
        for row in rows {
            let scryfall_id: String = row.try_get("", "scryfall_id")?;
            let face = TextFace {
                name: row.try_get("", "name")?,
                printed_name: row.try_get("", "printed_name")?,
                type_line: row.try_get("", "type_line")?,
                printed_type_line: row.try_get("", "printed_type_line")?,
                oracle_text: row.try_get("", "oracle_text")?,
                printed_text: row.try_get("", "printed_text")?,
                mana_cost: row.try_get("", "mana_cost")?,
            };
            match printings.last_mut() {
                Some(printing) if printing.scryfall_id == scryfall_id => {
                    printing.faces.push(face);
                }
                _ => printings.push(TextPrinting {
                    scryfall_id,
                    oracle_id: row.try_get("", "oracle_id")?,
                    lang: row.try_get("", "lang")?,
                    released_at: row.try_get("", "released_at")?,
                    collector_number: row.try_get("", "collector_number")?,
                    layout: row.try_get("", "layout")?,
                    faces: vec![face],
                }),
            }
        }
        Ok(printings
            .chunk_by(|a, b| a.oracle_id == b.oracle_id)
            .filter_map(|card| card_entry(lang, card))
            .collect())
    }

    /// Card text for a set of printings in a language, each answered under
    /// the id that was asked for.
    ///
    /// The printing only supplies the *identity*: the answer is
    /// [`Self::text_by_card`]'s for the card it is a printing of. That is
    /// what lets a player run an English game and read German cards, and it
    /// is why a client asking by printing gets the same text as one asking
    /// by card.
    ///
    /// # Errors
    /// When a query fails.
    pub async fn text(&self, ids: &[String], lang: &str) -> Result<Vec<CardTextEntry>> {
        let asked = self.cards_of(ids).await?;
        let mut cards: Vec<String> = asked.iter().map(|(_, card)| card.clone()).collect();
        cards.sort_unstable();
        cards.dedup();
        let by_card: std::collections::BTreeMap<String, CardTextEntry> = self
            .text_by_card(&cards, lang)
            .await?
            .into_iter()
            .map(|entry| (entry.oracle_id.clone(), entry))
            .collect();
        Ok(asked
            .into_iter()
            .filter_map(|(scryfall_id, card)| {
                let entry = by_card.get(&card)?;
                Some(CardTextEntry {
                    scryfall_id,
                    ..entry.clone()
                })
            })
            .collect())
    }

    /// A number that moves whenever the catalog's cards may have: what a
    /// cache of their text is keyed on. `0` before the first ingest that
    /// stamps it.
    ///
    /// Moved by [`Self::bump_data_version`], which [`ingest::bulk`] calls
    /// when it starts and when it ends — not by each [`Self::upsert`]. An
    /// ingest writes about 1 356 batches from a process of its own, and a
    /// reader that rebuilt at every one of them would rebuild on nearly every
    /// request while the ingest runs. The start stamp is what keeps an ingest
    /// that dies halfway from leaving a cache on the old text; the end stamp
    /// is what makes the finished catalog the one that is cached.
    ///
    /// # Errors
    /// When the query fails, or the stamp is not a number.
    pub async fn data_version(&self) -> Result<i64> {
        let row = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT value FROM catalog_meta WHERE key = $1",
                [Value::from(DATA_VERSION_KEY.to_string())],
            ))
            .await
            .context("reading the data version")?;
        let Some(row) = row else {
            return Ok(0);
        };
        let value: String = row.try_get("", "value")?;
        value
            .parse()
            .with_context(|| format!("the data version {value:?} is not a number"))
    }

    /// Moves [`Self::data_version`] on, and answers the new value.
    ///
    /// # Errors
    /// When the statement fails.
    pub async fn bump_data_version(&self) -> Result<i64> {
        let row = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO catalog_meta (key, value) VALUES ($1, '1') \
                 ON CONFLICT (key) DO UPDATE \
                 SET value = (catalog_meta.value::bigint + 1)::text \
                 RETURNING value",
                [Value::from(DATA_VERSION_KEY.to_string())],
            ))
            .await
            .context("moving the data version")?
            .context("the data version statement returned no row")?;
        let value: String = row.try_get("", "value")?;
        value
            .parse()
            .with_context(|| format!("the data version {value:?} is not a number"))
    }

    /// Every language the catalog has a name for.
    ///
    /// # Errors
    /// When the query fails.
    pub async fn languages(&self) -> Result<Vec<String>> {
        let rows = self
            .db
            .query_all_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT code FROM languages ORDER BY code",
            ))
            .await
            .context("listing languages")?;
        rows.into_iter()
            .map(|row| Ok(row.try_get("", "code")?))
            .collect()
    }

    /// Who illustrated each of these printings, by Scryfall id (WG-3: the
    /// deck list credits the art it shows). A printing the catalog lacks, or
    /// one it holds no artist for, is left out; an id that is not a UUID is
    /// dropped rather than failing the batch.
    ///
    /// # Errors
    /// When the query fails.
    pub async fn artists(
        &self,
        scryfall_ids: &[String],
    ) -> Result<std::collections::BTreeMap<String, String>> {
        let ids: Vec<&str> = scryfall_ids
            .iter()
            .map(String::as_str)
            .filter(|id| is_uuid(id))
            .collect();
        if ids.is_empty() {
            return Ok(std::collections::BTreeMap::new());
        }
        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT scryfall_id::text AS scryfall_id, artist \
                 FROM cards WHERE scryfall_id = ANY(string_to_array($1, ',')::uuid[]) \
                 AND coalesce(artist, '') <> ''",
                [Value::from(ids.join(","))],
            ))
            .await
            .context("looking up artists")?;
        rows.into_iter()
            .map(|row| Ok((row.try_get("", "scryfall_id")?, row.try_get("", "artist")?)))
            .collect()
    }

    /// Which card each known printing is, as `(scryfall_id, oracle_id)`,
    /// in printing-id order. A printing the catalog lacks is left out.
    ///
    /// # Errors
    /// When the query fails.
    pub async fn cards_of(&self, ids: &[String]) -> Result<Vec<(String, String)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT scryfall_id::text AS scryfall_id, oracle_id::text AS oracle_id \
                 FROM cards WHERE scryfall_id = ANY(string_to_array($1, ',')::uuid[]) \
                 ORDER BY scryfall_id",
                [Value::from(ids.join(","))],
            ))
            .await
            .context("looking up printings")?;
        rows.into_iter()
            .map(|row| {
                Ok((
                    row.try_get("", "scryfall_id")?,
                    row.try_get("", "oracle_id")?,
                ))
            })
            .collect()
    }

    /// Searches the catalog by name and rules text.
    ///
    /// One row per **card**, not per printing. The catalog holds every
    /// printing of every card in every language, so an undeduplicated search
    /// for `Blitzschlag` answered with twelve rows that were all the same
    /// card — tle, clb, m11, clb, m10, gn3, 2x2, 4ed, fbb, 2x2, sta, 3ed —
    /// and a `LIMIT 20` that looked like twenty results was three cards. A
    /// deck builder asks *which card do you mean*, and that question has one
    /// answer per `oracle_id`.
    ///
    /// Which printing stands for the card is chosen in [`Catalog::project`]
    /// rather than here, because `DISTINCT ON` keeps whichever row its sort
    /// saw first and a sort with ties is not a sort.
    ///
    /// # Why a name and a rules text are never in one predicate
    ///
    /// They used to be, joined by `OR`, and that single `OR` is what made
    /// this the slowest query in the workspace. An expression predicate the
    /// planner can index and an `ILIKE '%…%'` it cannot are unsatisfiable
    /// together by any index, so Postgres took the only plan left — a
    /// sequential scan building a `tsvector` for each of 554 242 faces, 4.9 µs
    /// a row, **2742 ms** for `稲妻` and 2132 ms for `li`. The cost was never
    /// the tokenizer and no extension would have moved it.
    ///
    /// So the two questions are asked separately over one projection and
    /// unioned. A name match also carries a *tier* — the whole name, a name
    /// starting with the query, a word inside a name starting with it, or the
    /// query anywhere in a name — and a rules-text match ranks below all
    /// four. The tiers are asked with `position()` rather than `LIKE` so a
    /// player may type `100%` without escaping anything, and `card_search`
    /// stores its names fenced in `| ` separators so one `position()` can ask
    /// "at the start of *a* name" over all of them at once.
    ///
    /// The representative printing is resolved **after** the `LIMIT`: the
    /// ranking needs only what the projection already holds, so the join back
    /// to `cards` and `card_faces` runs over the twenty rows that survived
    /// instead of over every printing of every match.
    ///
    /// Measured against the live catalog, this query against the one it
    /// replaced: `稲妻` 2742 → 1.6 ms, `li` 2132 → 42 ms, `creature` 190 →
    /// 67 ms, `flying` 81 → 14 ms, `aether` 12 → 1.8 ms, and
    /// `Ｌｉｇｈｔｎｉｎｇ` from no answer at all to 1.2 ms.
    ///
    /// An empty query is answered here rather than in Postgres: every name
    /// contains the empty string, so tier 1 matches all 41 991 rows and the
    /// server ranks the whole projection to hand back twenty — 175 ms to say
    /// nothing. A *short* query is the client's own decision and is not
    /// second-guessed: one ASCII letter costs 202 ms and is a real search.
    ///
    /// # Errors
    /// When the query fails.
    pub async fn search(&self, query: &str, lang: &str, limit: u64) -> Result<Vec<SearchHit>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let sql = search_sql();
        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [
                    Value::from(query.to_string()),
                    Value::from(lang.to_string()),
                    Value::from(limit as i64),
                ],
            ))
            .await
            .context("searching the catalog")?;

        rows.into_iter()
            .map(|row| {
                Ok(SearchHit {
                    scryfall_id: row.try_get("", "scryfall_id")?,
                    lang: row.try_get("", "lang")?,
                    name: row.try_get("", "display_name")?,
                    english_name: row.try_get("", "english_name")?,
                    type_line: row.try_get("", "display_type")?,
                })
            })
            .collect()
    }

    /// Every printing of every named card, newest set first.
    ///
    /// Keyed on `oracle_id` rather than on a printing id, because the picker's
    /// question is "what else is this card" and the answer crosses sets *and*
    /// languages. A card the catalog has never seen simply contributes no
    /// rows; the caller keeps whatever the registry knows.
    ///
    /// # Errors
    /// When the query fails.
    pub async fn printings(&self, oracle_ids: &[String]) -> Result<Vec<Printing>> {
        if oracle_ids.is_empty() {
            return Ok(Vec::new());
        }
        // `to_char` and not `::text`: a `date` renders through the session's
        // `DateStyle`. sqlx pins that to ISO at connect (checked 2026-09-24:
        // a `-c DateStyle` option loses to it), but the wire format should
        // not rest on the driver. The empty string for a printing with no date is what the
        // column said before it was a date, and `Printing` is a wire shape.
        let sql = "\
            SELECT c.scryfall_id::text AS scryfall_id, c.oracle_id::text AS oracle_id, \
                   c.lang AS lang, c.set_code AS set_code, \
                   coalesce(c.set_name, '') AS set_name, \
                   c.collector_number AS collector_number, \
                   coalesce(c.rarity, '') AS rarity, \
                   coalesce(to_char(c.released_at, 'YYYY-MM-DD'), '') AS released_at, \
                   coalesce(c.artist, '') AS artist, \
                   coalesce(c.finishes, 'nonfoil') AS finishes, \
                   coalesce(c.frame_effects, '') AS frame_effects, \
                   coalesce(c.border_color, '') AS border_color, coalesce(c.layout, '') AS layout, \
                   c.promo AS promo, \
                   coalesce(f.printed_name, f.name) AS name \
            FROM cards c \
            JOIN card_faces f ON f.scryfall_id = c.scryfall_id AND f.face_index = 0 \
            WHERE c.oracle_id = ANY(string_to_array($1, ',')::uuid[]) \
            ORDER BY c.oracle_id, c.released_at DESC NULLS LAST, c.set_code, \
                     length(c.collector_number), c.collector_number, c.lang";

        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [Value::from(oracle_ids.join(","))],
            ))
            .await
            .context("listing printings")?;

        rows.into_iter()
            .map(|row| {
                let finishes: String = row.try_get("", "finishes")?;
                let frames: String = row.try_get("", "frame_effects")?;
                Ok(Printing {
                    scryfall_id: row.try_get("", "scryfall_id")?,
                    oracle_id: row.try_get("", "oracle_id")?,
                    lang: row.try_get("", "lang")?,
                    set: row.try_get("", "set_code")?,
                    set_name: row.try_get("", "set_name")?,
                    layout: row.try_get("", "layout")?,
                    collector_number: row.try_get("", "collector_number")?,
                    rarity: row.try_get("", "rarity")?,
                    released_at: row.try_get("", "released_at")?,
                    artist: row.try_get("", "artist")?,
                    finishes: split_list(&finishes),
                    frame_effects: split_list(&frames),
                    border_color: row.try_get("", "border_color")?,
                    promo: row.try_get("", "promo")?,
                    name: row.try_get("", "name")?,
                })
            })
            .collect()
    }

    /// Every distinct name the named cards are printed under.
    ///
    /// One row per (card, language), not per printing: forty printings of the
    /// German Forest are one name, and the deck builder wants the name.
    ///
    /// # Errors
    /// When the query fails.
    pub async fn names(&self, oracle_ids: &[String]) -> Result<Vec<LocalName>> {
        if oracle_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = "\
            SELECT DISTINCT c.oracle_id::text AS oracle_id, c.lang AS lang, \
                   coalesce(f.printed_name, f.name) AS name \
            FROM cards c \
            JOIN card_faces f ON f.scryfall_id = c.scryfall_id AND f.face_index = 0 \
            WHERE c.oracle_id = ANY(string_to_array($1, ',')::uuid[]) \
              AND coalesce(f.printed_name, f.name) <> '' \
            ORDER BY oracle_id, lang, name";

        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [Value::from(oracle_ids.join(","))],
            ))
            .await
            .context("listing localized names")?;

        rows.into_iter()
            .map(|row| {
                Ok(LocalName {
                    oracle_id: row.try_get("", "oracle_id")?,
                    lang: row.try_get("", "lang")?,
                    name: row.try_get("", "name")?,
                })
            })
            .collect()
    }

    /// Inserts or updates a batch of printings.
    ///
    /// # Errors
    /// When a statement fails.
    pub async fn upsert(&self, cards: &[scryfall::Card]) -> Result<usize> {
        let storable: Vec<&scryfall::Card> = cards.iter().filter(|c| c.is_storable()).collect();
        if storable.is_empty() {
            return Ok(0);
        }
        self.upsert_cards(&storable).await?;
        self.upsert_faces(&storable).await?;
        self.upsert_legalities(&storable).await?;
        Ok(storable.len())
    }

    /// The `card_legalities` half of a batch upsert.
    ///
    /// One row per card and not per printing, so a batch of 500 printings of
    /// forty cards writes forty rows. The batch is deduplicated here rather
    /// than left to `ON CONFLICT`, because Postgres refuses a statement that
    /// names one key twice ("cannot affect row a second time") — and every
    /// printing of a card carries the same answer, so the first is as good as
    /// any.
    async fn upsert_legalities(&self, cards: &[&scryfall::Card]) -> Result<()> {
        let mut seen: std::collections::BTreeMap<&str, &scryfall::Card> =
            std::collections::BTreeMap::new();
        for card in cards {
            if let Some(oracle_id) = card.oracle_identity()
                && !card.legalities.is_empty()
            {
                seen.entry(oracle_id).or_insert(card);
            }
        }
        let rows: Vec<(&str, &scryfall::Card)> = seen.into_iter().collect();
        for chunk in rows.chunks(rows_per_statement(LEGALITY_COLUMNS)) {
            self.execute_chunk(legalities_statement(chunk), "upserting legalities")
                .await?;
        }
        Ok(())
    }

    /// The `cards` half of a batch upsert.
    async fn upsert_cards(&self, cards: &[&scryfall::Card]) -> Result<()> {
        for chunk in cards.chunks(rows_per_statement(CARD_COLUMNS)) {
            self.execute_chunk(cards_statement(chunk), "upserting printings")
                .await?;
        }
        Ok(())
    }

    /// The `card_faces` half of a batch upsert.
    ///
    /// Split by face rows rather than by cards, because how many faces a
    /// card has is Scryfall's to say.
    async fn upsert_faces(&self, cards: &[&scryfall::Card]) -> Result<()> {
        let rows = face_rows(cards);
        for chunk in rows.chunks(rows_per_statement(FACE_COLUMNS)) {
            self.execute_chunk(faces_statement(chunk), "upserting faces")
                .await?;
        }
        Ok(())
    }

    /// Runs one statement an upsert built.
    async fn execute_chunk(
        &self,
        (sql, values): (String, Vec<Value>),
        what: &'static str,
    ) -> Result<()> {
        self.db
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                values,
            ))
            .await
            .context(what)?;
        Ok(())
    }

    /// Tell the planner what is now in the tables.
    ///
    /// PostgreSQL's autovacuum gets to this eventually, and "eventually" is
    /// the problem: an ingest writes 542k rows in about three minutes and the
    /// searches start immediately after. Until the statistics catch up the
    /// planner is choosing between a trigram index and a sequential scan on
    /// its estimate for an empty table, and it picks the scan — so the first
    /// minutes after the one operation that fills this database are the
    /// slowest searches it will ever serve.
    ///
    /// One statement, once, at the end of the ingest. It is the cheapest
    /// thing in this crate that makes the search faster.
    ///
    /// # Errors
    /// When the statement fails.
    pub async fn analyze(&self) -> Result<()> {
        self.db
            .execute_unprepared("ANALYZE cards, card_faces")
            .await
            .context("analyzing the catalog tables")?;
        Ok(())
    }

    /// How many printings are stored.
    ///
    /// # Errors
    /// When the query fails.
    pub async fn count(&self) -> Result<i64> {
        let row = self
            .db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT count(*)::bigint AS n FROM cards",
            ))
            .await?
            .context("count returned no row")?;
        Ok(row.try_get("", "n")?)
    }

    /// Whether there is anything to serve, in one round trip.
    ///
    /// Two `EXISTS` rather than [`Self::count`], and the difference is the
    /// whole reason this exists: `count(*)` over `cards` is a sequential scan,
    /// and the caller is an unauthenticated route a monitor polls. `EXISTS`
    /// stops at the first row, so the answer costs the same on an empty
    /// catalog as on a complete one. The question being asked is "is there
    /// card text here", never "how much".
    ///
    /// Measured against an `--english-only` catalog of 118 609 printings:
    /// 7.47 ms for the count, 0.61 ms for both `EXISTS` together. The gap is
    /// not the point — 7 ms would be affordable — the *slope* is: the count
    /// grows with the table and a full catalog is 542 177 rows, while the
    /// pair stays flat because neither side reads a second row.
    ///
    /// Both halves are asked because they fail apart. DDL alone leaves an
    /// upgrading install a full `cards` beside an empty `card_search`, and
    /// that state answers every search with nothing and never errors — it is
    /// exactly what [`Self::migrate`]'s second half exists to repair, so it is
    /// the one worth being able to see from outside while it is happening.
    ///
    /// # Errors
    /// When the query fails, which for this caller is itself the answer: the
    /// database is not reachable, or the schema was never applied.
    pub async fn readiness(&self) -> Result<Readiness> {
        let row = self
            .db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT EXISTS (SELECT 1 FROM cards) AS cards, \
                 EXISTS (SELECT 1 FROM card_search) AS projection",
            ))
            .await?
            .context("readiness returned no row")?;
        Ok(Readiness {
            cards: row.try_get("", "cards")?,
            projection: row.try_get("", "projection")?,
        })
    }
}

#[cfg(test)]
mod tests;

/// Whether this is a UUID in its hyphenated form, which is the only one a
/// Scryfall id comes in: a malformed one would fail a whole `::uuid[]` cast.
fn is_uuid(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(at, b)| match at {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}
