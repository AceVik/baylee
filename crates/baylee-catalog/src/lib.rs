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
pub mod scryfall;

use anyhow::{Context, Result};
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, TransactionTrait, Value,
};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// What `/catalog/text` answers: the one wire shape both ends link, from
/// the crate that also holds the rule a printing is picked by.
pub use baylee_cardtext::{CardTextEntry, FaceText};

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

/// The advisory lock a rebuild holds.
///
/// Two gateways starting against one database both find the version stale,
/// and the lock is where the second one stops — but only because the
/// staleness question is asked *inside* the transaction that holds it
/// ([`Catalog::rebuild_if_stale`]). The loser waits, asks again, reads the
/// version the winner stamped, and does nothing. A lock taken after the
/// question would serialise two rebuilds rather than prevent one, which is
/// what `TRUNCATE` does for free and why the lock would then be decoration.
const PROJECT_LOCK: i64 = 0x000b_a11e_eca7_a106;

/// Serialises the DDL in [`Catalog::migrate`].
///
/// **`IF NOT EXISTS` is a check and an insert, and nothing holds them
/// together.** Two sessions migrating at once both find `unaccent` missing,
/// both insert, and the loser gets `duplicate key value violates unique
/// constraint "pg_extension_name_index"` — not a wrong schema, but a failed
/// migration, which is worse than either outcome it was protecting against.
/// It is invisible wherever the extension already exists, so it does not
/// happen on a development machine and does happen on a CI server that
/// starts an empty PostgreSQL and then runs the e2e tests in parallel
/// schemas: three tests died on it there while every local run was green.
/// `CREATE TABLE`/`CREATE INDEX IF NOT EXISTS` race the same way, so the
/// lock is around the whole loop and not around the one statement that was
/// caught.
const SCHEMA_LOCK: i64 = 0x000b_a11e_5c4e_3a01;

/// The body of `catalog_norm`, which a query and a stored name are both
/// folded through.
///
/// Three foldings, and each one is a language this catalog serves. `NFKC`
/// turns the full-width Latin a Japanese keyboard produces (`Ｌｉｇｈｔｎｉｎｇ`)
/// into the ASCII a deck builder stores; `unaccent` makes `Æther` reachable
/// by typing `aether`; `lower` is the rest. `unaccent`'s two-argument form is
/// used rather than the one-argument one because only the two-argument form
/// is `IMMUTABLE` — naming the dictionary is what makes this function honest
/// about being one.
const NORM_BODY: &str = "SELECT lower(unaccent('unaccent', normalize(s, NFKC)))";

/// The body of `catalog_bigrams`, which the name index is built from.
///
/// Bigrams rather than trigrams, and the reason is a two-character card name.
/// `pg_trgm` pads a whole *word*, so `show_trgm('稲妻')` does return three
/// trigrams — but a `%稲妻%` pattern is not padded, so the index it built
/// could not serve the query that needed it, and a search for `稲妻` was a
/// sequential scan over 554 242 faces at 2742 ms. Every non-ASCII character
/// is also indexed on its own, because `島` is a card name and a whole word.
const BIGRAMS_BODY: &str = "SELECT coalesce(array_agg(DISTINCT g), '{}'::text[]) FROM ( \
       SELECT substr(s, i, 2) AS g FROM generate_series(1, greatest(length(s) - 1, 0)) i \
       UNION ALL \
       SELECT substr(s, i, 1) FROM generate_series(1, length(s)) i \
        WHERE substr(s, i, 1) !~ '[[:ascii:]]' \
     ) t";

/// Fills `card_search` from `cards` and `card_faces`.
///
/// The inner query is one row per *printed text* — `DISTINCT ON (oracle_id,
/// lang, face_index)` over 542 177 printings leaves 296 313 — and the outer
/// one collapses those to 41 991 oracle faces, each carrying every language
/// it is printed in. Which printing speaks for a language is decided here
/// rather than in the search, and the order is the one the search used to
/// carry: a printing that actually has a translated type line first, because
/// 6489 of 59 465 German faces have a `printed_name` and no
/// `printed_type_line`, and taking one of those heads a German card
/// `Instant`.
///
/// The `LEFT JOIN` is the half that reaches cards nobody printed abroad.
/// Merging every printed type line already makes `同盟者`, `Ally` and
/// `Kleriker` reach the right cards — but only cards somebody printed that
/// way, and **3634 cards have no German printing at all**. So `type_names`
/// says what `Ally` is called in ten languages and the words go in whether or
/// not the printing exists. Measured here: 89.5% of the subtype occurrences
/// on those 3634 cards can be named in German, against none before.
///
/// It is a join inside this `INSERT` and not an `UPDATE` afterwards, which is
/// the shape it was written in first. Both produce the same `tsvector` —
/// `tsv || tsv` deduplicates the tokens — but the update writes every row a
/// second time, and the whole cost of the translation turned out to be that
/// second write. Measured on the live catalog, `card_search` in total:
/// **215 MB** before any of this, **413 MB** written twice, **230 MB** as it
/// stands. The stored text grew 76 MB to 85 MB in both, so the translation
/// itself costs 15 MB and the other 183 MB was the rewrite. A `VACUUM` finds
/// nothing to report about it either, because the dead half is reusable space
/// inside the files rather than dead tuples, and only the next `TRUNCATE`
/// gives it back. The join is also the faster of the two: 23.8 s against
/// 32.8 s for a full rebuild.
///
/// The dictionary is joined on **whole words**, not with a `LIKE` over the
/// type line. Padding the line and matching `'% ' || english || ' %'` would
/// also find a *multi-word* subtype, and that shape was measured too: 22.8 s
/// against 1.3 s, a nested loop rejecting 178 million pairs. Magic prints
/// exactly one multi-word subtype, `Time Lord`, which no printing anywhere
/// translates — so the general shape costs seventeen times the time to reach
/// a card that is unsearchable in German either way.
///
/// The left side of the type line is looked up twice: as the whole phrase
/// (`Legendary Creature` is `Legendäre Kreatur`, and a supertype is never
/// decomposed — `Basic Land` is one German word for two) and as its last word
/// alone, the card type. The second is what catches the 28 faces whose exact
/// combination of supertypes was never printed abroad, which would otherwise
/// get no word at all.
///
/// The second `LEFT JOIN` carries **flavor names** — the just-for-fun name a
/// Secret Lair prints instead of the card's own, with the real one in small
/// type beside it. It is joined separately, and not folded into the inner
/// query, because that one keeps one printing per language (`DISTINCT ON`)
/// and a flavor name belongs to the *printing*: Command Tower has six of
/// them, and picking one printing would throw five away. 476 cards in 661
/// printings carry one. They reach `names_norm`, so typing `Cybertron` finds
/// Command Tower at the same tier as typing its own name, and `bg` follows
/// because it is `GENERATED` from that column. They reach neither `names` —
/// which answers "what is this card called in your language", and a flavor
/// name is not a language — nor anything the rules read: `oracle_id` and the
/// Oracle name are unchanged, which is the whole reason this is a search
/// concern and not a card one.
///
/// **A printed English name is not a language either**, and that is the same
/// argument one step in. `names` took `coalesce(printed_name, name)` from the
/// newest printing of each language, which is exactly right abroad — the
/// printed name *is* the German name — and wrong at home, where `name` is the
/// Oracle name and `printed_name` is how one Secret Lair chose to set the
/// type. 32 of 41 991 faces answered `names->>'en'` with a styling:
/// `BIRDS OF PARADISE`, `IMP'S MSCHF`, `GIGANTO-SAURUS` (#35). So English
/// takes `name` and every other language keeps the printed one.
///
/// The styling is not lost, it moves. `pn` is the printed spelling whatever
/// the language, and it joins `names_norm` and `tsv` where it differs from
/// the display name — the same place a flavor name lives, for the same
/// reason: somebody holding the card and typing what is on it has to find it.
const PROJECT_SQL: &str = "\
    WITH face AS ( \
      SELECT DISTINCT ON (c.oracle_id, cf.face_index) \
             c.oracle_id, cf.face_index, cf.type_line \
      FROM cards c JOIN card_faces cf USING (scryfall_id) \
      WHERE cf.type_line IS NOT NULL \
      ORDER BY c.oracle_id, cf.face_index, (c.lang = 'en') DESC, c.scryfall_id \
    ) \
    INSERT INTO card_search (oracle_id, face_index, names, names_norm, tsv) \
    SELECT p.oracle_id, p.face_index, p.names, \
           p.names_norm || coalesce(v.extra_norm, ''), \
           p.tsv || coalesce(t.extra, ''::tsvector) \
                 || coalesce(v.extra_tsv, ''::tsvector) \
    FROM ( \
      SELECT oracle_id, face_index, \
             jsonb_object_agg(lang, nm) AS names, \
             '| ' || catalog_norm(string_agg(DISTINCT nm, ' | ') \
               || coalesce(' | ' || string_agg(DISTINCT pn, ' | ') \
                             FILTER (WHERE pn <> nm), '')) || ' |' AS names_norm, \
             to_tsvector('simple', string_agg(nm || ' ' || tl || ' ' || tx, ' ') \
               || coalesce(' ' || string_agg(pn, ' ') FILTER (WHERE pn <> nm), '')) AS tsv \
      FROM ( \
        SELECT DISTINCT ON (c.oracle_id, c.lang, f.face_index) \
               c.oracle_id, c.lang AS lang, f.face_index, \
               CASE WHEN c.lang = 'en' THEN f.name \
                    ELSE coalesce(f.printed_name, f.name) END AS nm, \
               coalesce(f.printed_name, f.name) AS pn, \
               coalesce(f.printed_type_line, f.type_line, '') AS tl, \
               coalesce(f.printed_text, f.oracle_text, '') AS tx \
        FROM cards c JOIN card_faces f USING (scryfall_id) \
        ORDER BY c.oracle_id, c.lang, f.face_index, \
                 (f.printed_type_line IS NOT NULL) DESC, \
                 c.released_at DESC NULLS LAST, c.scryfall_id \
      ) one_per_language \
      GROUP BY oracle_id, face_index \
    ) p LEFT JOIN ( \
      SELECT w.oracle_id, w.face_index, \
             to_tsvector('simple', string_agg(DISTINCT d.printed, ' ')) AS extra \
      FROM ( \
        SELECT oracle_id, face_index, split_part(type_line, ' — ', 1) AS english FROM face \
        UNION ALL \
        SELECT oracle_id, face_index, regexp_replace(split_part(type_line, ' — ', 1), '^.* ', '') \
        FROM face \
        UNION ALL \
        SELECT oracle_id, face_index, unnest(string_to_array(split_part(type_line, ' — ', 2), ' ')) \
        FROM face WHERE type_line LIKE '% — %' \
      ) w JOIN type_names d USING (english) \
      GROUP BY w.oracle_id, w.face_index \
    ) t USING (oracle_id, face_index) \
    LEFT JOIN ( \
      SELECT c.oracle_id, f.face_index, \
             ' ' || catalog_norm(string_agg(DISTINCT f.flavor_name, ' | ')) || ' |' \
               AS extra_norm, \
             to_tsvector('simple', string_agg(DISTINCT f.flavor_name, ' ')) AS extra_tsv \
      FROM cards c JOIN card_faces f USING (scryfall_id) \
      WHERE f.flavor_name IS NOT NULL AND f.flavor_name <> '' \
      GROUP BY c.oracle_id, f.face_index \
    ) v USING (oracle_id, face_index)";

/// The columns one printing binds in `upsert_cards`, in bind order.
///
/// The placeholder run, the value pushes and the table definition are written
/// three places apart; a mismatch makes Postgres reject the whole batch, so
/// the list lives here and the tests hold the other two against it.
const CARD_INSERT_COLUMNS: &str = "scryfall_id, oracle_id, lang, set_code, collector_number, \
     rarity, layout, released_at, set_name, artist, finishes, frame_effects, border_color, promo, set_type, \
     digital, games";

/// How many columns that is.
const CARD_COLUMNS: usize = 17;

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

    /// Looks up text for a set of printings in a language.
    ///
    /// The requested printing only supplies the *identity*: the row that comes
    /// back is the same card in the requested language when one exists, and
    /// the English printing otherwise. That is what lets a player run an
    /// English game and read German cards.
    ///
    /// # Errors
    /// When the query fails.
    pub async fn text(&self, ids: &[String], lang: &str) -> Result<Vec<CardTextEntry>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = "\
            WITH wanted AS (SELECT unnest(string_to_array($1, ','))::uuid AS id) \
            SELECT w.id::text AS requested, c.lang AS lang, f.face_index AS face_index, \
                   f.name AS name, f.printed_name AS printed_name, \
                   f.type_line AS type_line, f.printed_type_line AS printed_type_line, \
                   f.oracle_text AS oracle_text, f.printed_text AS printed_text, \
                   f.mana_cost AS mana_cost \
            FROM wanted w \
            JOIN cards src ON src.scryfall_id = w.id \
            JOIN LATERAL ( \
                SELECT c2.scryfall_id, c2.lang FROM cards c2 \
                WHERE c2.oracle_id = src.oracle_id AND c2.lang IN ($2, 'en') \
                ORDER BY (c2.lang = $2) DESC, c2.released_at DESC NULLS LAST \
                LIMIT 1 \
            ) c ON TRUE \
            JOIN card_faces f ON f.scryfall_id = c.scryfall_id \
            ORDER BY requested, f.face_index";

        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [Value::from(ids.join(",")), Value::from(lang.to_string())],
            ))
            .await
            .context("looking up card text")?;

        let mut out: Vec<CardTextEntry> = Vec::new();
        for row in rows {
            let requested: String = row.try_get("", "requested")?;
            let lang: String = row.try_get("", "lang")?;
            let name: String = row.try_get("", "name")?;
            let printed_name: Option<String> = row.try_get("", "printed_name")?;
            let type_line: Option<String> = row.try_get("", "type_line")?;
            let printed_type_line: Option<String> = row.try_get("", "printed_type_line")?;
            let oracle_text: Option<String> = row.try_get("", "oracle_text")?;
            let printed_text: Option<String> = row.try_get("", "printed_text")?;
            let mana_cost: Option<String> = row.try_get("", "mana_cost")?;

            let face = FaceText {
                // Field-by-field fallback: a printing may be translated but
                // have no translated rules text, and half a card is better
                // than none.
                name: printed_name.unwrap_or_else(|| name.clone()),
                english_name: name,
                type_line: printed_type_line.or(type_line).unwrap_or_default(),
                oracle_text: printed_text.or(oracle_text).unwrap_or_default(),
                mana_cost: mana_cost.unwrap_or_default(),
            };
            match out.last_mut() {
                Some(entry) if entry.scryfall_id == requested => entry.faces.push(face),
                _ => out.push(CardTextEntry {
                    scryfall_id: requested,
                    lang,
                    faces: vec![face],
                }),
            }
        }
        Ok(out)
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
        if seen.is_empty() {
            return Ok(());
        }
        let mut sql = String::from("INSERT INTO card_legalities (oracle_id, legalities) VALUES ");
        let mut values: Vec<Value> = Vec::with_capacity(seen.len() * 2);
        for (n, (oracle_id, card)) in seen.iter().enumerate() {
            if n > 0 {
                sql.push(',');
            }
            let _ = write!(sql, "(${}::uuid,${}::jsonb)", n * 2 + 1, n * 2 + 2);
            values.push(Value::from((*oracle_id).to_string()));
            values.push(Value::from(
                serde_json::to_string(&card.legalities).unwrap_or_else(|_| "{}".to_string()),
            ));
        }
        sql.push_str(" ON CONFLICT (oracle_id) DO UPDATE SET legalities = EXCLUDED.legalities");
        self.db
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                values,
            ))
            .await
            .context("upserting legalities")?;
        Ok(())
    }

    /// The `cards` half of a batch upsert.
    async fn upsert_cards(&self, cards: &[&scryfall::Card]) -> Result<()> {
        let mut sql = format!("INSERT INTO cards ({CARD_INSERT_COLUMNS}) VALUES ");
        let mut values: Vec<Value> = Vec::with_capacity(cards.len() * CARD_COLUMNS);
        for (i, card) in cards.iter().enumerate() {
            let base = i * CARD_COLUMNS;
            if i > 0 {
                sql.push(',');
            }
            sql.push('(');
            for (k, column) in CARD_INSERT_COLUMNS.split(", ").enumerate() {
                if k > 0 {
                    sql.push(',');
                }
                let at = base + k + 1;
                // Everything a `scryfall::Card` carries is a `String` or a
                // `bool` and Postgres infers most of them from the column.
                // The three it cannot are named here rather than counted to,
                // so adding a column ahead of one of them cannot move it.
                let _ = match column {
                    "scryfall_id" | "oracle_id" => write!(sql, "${at}::uuid"),
                    // An omitted `released_at` binds as null and the cast is
                    // what tells Postgres which kind of null; an *empty* one
                    // would be `''::date`, which is an error and not a null.
                    "released_at" => write!(sql, "nullif(${at}, '')::date"),
                    _ => write!(sql, "${at}"),
                };
            }
            sql.push(')');
            values.push(Value::from(card.id.clone()));
            values.push(Value::from(card.oracle_identity().map(str::to_string)));
            values.push(Value::from(card.lang.clone()));
            values.push(Value::from(card.set.clone()));
            values.push(Value::from(card.collector_number.clone()));
            values.push(Value::from(card.rarity.clone()));
            values.push(Value::from(card.layout.clone()));
            values.push(Value::from(card.released_at.clone()));
            values.push(Value::from(card.set_name.clone()));
            values.push(Value::from(card.artist.clone()));
            values.push(Value::from(card.finish_list().join(",")));
            values.push(Value::from(card.frame_effects.join(",")));
            values.push(Value::from(card.border_color.clone()));
            values.push(Value::from(card.promo));
            values.push(Value::from(card.set_type.clone()));
            values.push(Value::from(card.digital));
            values.push(Value::from(card.games.join(",")));
        }
        sql.push_str(
            " ON CONFLICT (scryfall_id) DO UPDATE SET \
             oracle_id = EXCLUDED.oracle_id, lang = EXCLUDED.lang, \
             set_code = EXCLUDED.set_code, collector_number = EXCLUDED.collector_number, \
             rarity = EXCLUDED.rarity, layout = EXCLUDED.layout, \
             released_at = EXCLUDED.released_at, set_name = EXCLUDED.set_name, \
             artist = EXCLUDED.artist, finishes = EXCLUDED.finishes, \
             frame_effects = EXCLUDED.frame_effects, border_color = EXCLUDED.border_color, \
             promo = EXCLUDED.promo, set_type = EXCLUDED.set_type, \
             digital = EXCLUDED.digital, games = EXCLUDED.games, \
             updated_at = now()",
        );
        self.db
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                values,
            ))
            .await
            .context("upserting printings")?;
        Ok(())
    }

    /// The `card_faces` half of a batch upsert.
    async fn upsert_faces(&self, cards: &[&scryfall::Card]) -> Result<()> {
        let mut sql = String::from(
            "INSERT INTO card_faces \
             (scryfall_id, face_index, name, printed_name, flavor_name, \
              type_line, printed_type_line, \
              oracle_text, printed_text, mana_cost, power, toughness, loyalty) VALUES ",
        );
        let mut values: Vec<Value> = Vec::new();
        let mut n = 0usize;
        for card in cards {
            for (index, face) in card.faces().into_iter().enumerate() {
                let base = n * 13;
                if n > 0 {
                    sql.push(',');
                }
                let _ = write!(
                    sql,
                    "(${}::uuid,${},${},${},${},${},${},${},${},${},${},${},${})",
                    base + 1,
                    base + 2,
                    base + 3,
                    base + 4,
                    base + 5,
                    base + 6,
                    base + 7,
                    base + 8,
                    base + 9,
                    base + 10,
                    base + 11,
                    base + 12,
                    base + 13
                );
                values.push(Value::from(card.id.clone()));
                values.push(Value::from(index as i16));
                values.push(Value::from(face.name));
                values.push(Value::from(face.printed_name));
                values.push(Value::from(face.flavor_name));
                values.push(Value::from(face.type_line));
                values.push(Value::from(face.printed_type_line));
                values.push(Value::from(face.oracle_text));
                values.push(Value::from(face.printed_text));
                values.push(Value::from(face.mana_cost));
                values.push(Value::from(face.power));
                values.push(Value::from(face.toughness));
                values.push(Value::from(face.loyalty));
                n += 1;
            }
        }
        if n == 0 {
            return Ok(());
        }
        sql.push_str(
            " ON CONFLICT (scryfall_id, face_index) DO UPDATE SET \
             name = EXCLUDED.name, printed_name = EXCLUDED.printed_name, \
             flavor_name = EXCLUDED.flavor_name, \
             type_line = EXCLUDED.type_line, printed_type_line = EXCLUDED.printed_type_line, \
             oracle_text = EXCLUDED.oracle_text, printed_text = EXCLUDED.printed_text, \
             mana_cost = EXCLUDED.mana_cost, power = EXCLUDED.power, \
             toughness = EXCLUDED.toughness, loyalty = EXCLUDED.loyalty",
        );
        self.db
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                values,
            ))
            .await
            .context("upserting faces")?;
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
async fn take_the_projection_lock(conn: &impl ConnectionTrait) -> Result<()> {
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
async fn take_the_schema_lock(conn: &impl ConnectionTrait) -> Result<()> {
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
async fn projection_is_stale(conn: &impl ConnectionTrait) -> Result<bool> {
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
async fn rebuild_the_projection(conn: &impl ConnectionTrait) -> Result<()> {
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

/// The card corpus, in the order the cards first appeared.
///
/// This is where a `CardIndex` comes from. An index is assigned by first
/// appearance and never moves again, so the order below is the only thing
/// that decides one, and it has to be **total**: release date, then the
/// English name, then the oracle id, which no two cards share. Two rows that
/// tie on all three are the same card.
///
/// "First appearance" is the earliest printing in *any* language, because
/// that is when the card appeared; English only breaks a tie on the same day,
/// so the set recorded beside it is the one a person would name.
///
/// Three clauses decide what counts as a card at all, and all three are
/// Scryfall's own words rather than a list of set codes this repo would have
/// to keep correct. `set_type` drops the sets that print souvenirs:
/// `memorabilia` is art cards and the challenge decks, `token` is what it
/// says. `layout` drops the things shaped like cards that turn up inside
/// ordinary sets — a Commander Collection's Snake token is `arsenal`, not
/// `token`. What is left keeps every layout the rules have a type for, planes
/// and schemes and Vanguard avatars included, because [`baylee_core`]'s type
/// bits already do.
///
/// The third clause is the interesting one, and it is why a joke set is not
/// dropped wholesale. Unfinity printed 266 tournament-legal cards beside its
/// acorn ones — same set, same black border — so neither `set_type` nor the
/// border colour separates them. Legality does, in its weakest sense:
/// `not_legal` means no format has ever heard of the card, while `banned` is
/// a real card that a format has an opinion about. It is asked **only**
/// inside a joke set, because planes, schemes and Vanguard avatars are
/// `not_legal` too and are perfectly real.
///
/// All three sit inside one `OR`, because a rule this crate owns cannot see
/// the other half of the question. `data/corpus-keep.tsv` names the cards
/// *this repo implements* and the rule drops — eight acorn lands written
/// before it existed — and codegen cannot build a card with no row in the
/// ledger, so dropping one is not a tidier corpus but a card that stops
/// compiling. A kept card is admitted whole rather than appended: its set and
/// its place in the order come out of this same query, which is the only
/// reason the `set` column beside it is worth freezing.
///
/// [`baylee_core`]: https://docs.rs/baylee-core
const CORPUS_SQL: &str = "\
    WITH first_printing AS ( \
      SELECT DISTINCT ON (c.oracle_id) \
             c.oracle_id, c.scryfall_id, c.released_at, c.set_code \
      FROM cards c \
      WHERE c.released_at IS NOT NULL \
        AND (c.oracle_id::text = ANY(string_to_array($1, ',')) OR ( \
          coalesce(c.set_type, '') NOT IN ('memorabilia', 'token') \
          AND (coalesce(c.set_type, '') <> 'funny' OR EXISTS ( \
                SELECT 1 FROM card_legalities g \
                WHERE g.oracle_id = c.oracle_id \
                  AND coalesce(g.legalities ->> 'vintage', 'not_legal') \
                        <> 'not_legal')) \
          AND coalesce(c.layout, '') \
                NOT IN ('token', 'double_faced_token', 'art_series', 'emblem'))) \
      ORDER BY c.oracle_id, c.released_at, (c.lang = 'en') DESC, \
               c.set_code, c.collector_number \
    ) \
    SELECT p.oracle_id::text AS oracle_id, to_char(p.released_at, 'YYYY-MM-DD') AS released_at, \
           p.set_code AS set_code, \
           string_agg(f.name, ' // ' ORDER BY f.face_index) AS name \
    FROM first_printing p JOIN card_faces f USING (scryfall_id) \
    WHERE f.name <> '' \
    GROUP BY p.oracle_id, p.released_at, p.set_code \
    ORDER BY released_at, name, oracle_id";

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
fn split_list(joined: &str) -> Vec<String> {
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
fn schema_statements() -> Vec<String> {
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
fn projection_schema() -> Vec<String> {
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
fn reference_tables() -> Vec<String> {
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
fn language_seed() -> String {
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

/// The three readings [`Catalog::mine_type_names`] makes, in order.
///
/// Each one may only write a pair it understood in full, which is the same
/// rule the card transcoder obeys and for the same reason: a wrong row here
/// is a German word attached to the wrong cards in everybody's search, and
/// nothing downstream can tell it from a right one.
///
/// **Round 0, the type line's left side**, as one phrase. Supertypes are not
/// decomposed — German prints `Basic Land` as `Standardland`, one word for
/// two, and Spanish lowercases and reorders its adjectives.
///
/// **Round 1, cards with exactly one subtype.** The clean signal: the whole
/// printed segment is the whole translation, with nothing to split. It
/// reaches 316 of the 506 subtypes in German.
///
/// **Round 2, subtraction.** A card with several subtypes whose printed
/// segment tokenises into forms round 0 and 1 already know, plus exactly one
/// leftover, and whose English segment has exactly one unknown — then the two
/// leftovers are each other. This is what `Druid`, `Warlock`, `Advisor`,
/// `Ninja` and `Ally` come from: 62 more subtypes in German, and 79.7% to
/// 89.4% of the subtype occurrences on cards with no German printing. There
/// is no round 3; the remainder does not fall to more passes, it falls to
/// Wizards printing those cards in German.
///
/// Where the rounds disagree, the **most frequent** form wins and the
/// **newest** printing breaks a tie. That is not a style choice: 48 of 317
/// German cells hold more than one form, and reading them showed the extra
/// forms are old type lines and errata — `Löwe` and `Tiger` on cards Scryfall
/// now calls `Cat`, `Engellegende` from when Legend was a card type. Both
/// rules agree everywhere but one cell (`Orgg`, a 2–2 tie the newest printing
/// settles), so the tiebreak is doing exactly the work it claims to.
const MINE_ROUNDS: [&str; 4] = [
    "CREATE TEMP TABLE mined (english text, lang text, printed text, round int) ON COMMIT DROP",
    // Round 0: the left side.
    "INSERT INTO mined \
     SELECT DISTINCT ON (english, lang) english, lang, printed, 0 FROM ( \
       SELECT c.lang, c.released_at, \
              CASE WHEN f.type_line LIKE '% — %' \
                   THEN split_part(f.type_line, ' — ', 1) ELSE f.type_line END AS english, \
              btrim(CASE WHEN f.printed_type_line ~ '( — | : |～| - )' \
                         THEN regexp_replace(f.printed_type_line, '( — | : |～| - ).*$', '') \
                         ELSE f.printed_type_line END) AS printed \
       FROM cards c JOIN card_faces f USING (scryfall_id) \
       WHERE c.lang <> 'en' AND f.printed_type_line IS NOT NULL AND f.type_line IS NOT NULL \
     ) p WHERE printed <> '' \
     GROUP BY english, lang, printed \
     ORDER BY english, lang, count(*) DESC, max(released_at) DESC NULLS LAST, printed",
    // Round 1: one subtype, one translation.
    "INSERT INTO mined \
     SELECT DISTINCT ON (english, lang) english, lang, printed, 1 FROM ( \
       SELECT lang, released_at, subs[1] AS english, printed FROM ( \
         SELECT c.lang, c.released_at, \
                string_to_array(split_part(f.type_line, ' — ', 2), ' ') AS subs, \
                btrim(regexp_replace(f.printed_type_line, '^.*?( — | : |～| - )', '')) AS printed \
         FROM cards c JOIN card_faces f USING (scryfall_id) \
         WHERE c.lang <> 'en' AND f.printed_type_line IS NOT NULL \
           AND f.type_line LIKE '% — %' AND f.printed_type_line ~ '( — | : |～| - )' \
       ) seg WHERE array_length(subs, 1) = 1 AND subs[1] <> '' \
     ) s WHERE printed <> '' \
     GROUP BY english, lang, printed \
     ORDER BY english, lang, count(*) DESC, max(released_at) DESC NULLS LAST, printed",
    // Round 2: strike out what is already known and see what is left.
    //
    // The tokeniser has to serve every language at once: German separates
    // subtypes with a comma, Japanese with `・`, Simplified Chinese with `／`,
    // and the Romance languages with a space and sometimes a conjunction
    // (`humain et clerc`), which is not a subtype and would otherwise be
    // learned as one the first time both its neighbours were known.
    "INSERT INTO mined \
     SELECT DISTINCT ON (english, lang) english, lang, printed, 2 FROM ( \
       SELECT lang, released_at, unknown[1] AS english, leftover[1] AS printed FROM ( \
         SELECT s.lang, s.released_at, \
                (SELECT array_agg(x) FROM unnest(s.subs) x WHERE x <> '' \
                  AND NOT EXISTS (SELECT 1 FROM mined d \
                                   WHERE d.lang = s.lang AND d.english = x)) AS unknown, \
                (SELECT array_agg(t) FROM unnest( \
                   regexp_split_to_array(s.printed, '[,、，／/・]|\\s+')) t \
                  WHERE btrim(t) <> '' \
                    AND lower(t) NOT IN ('et', 'y', 'e', 'and', 'und', 'i', 'ed', '和') \
                    AND NOT EXISTS (SELECT 1 FROM mined d \
                                     WHERE d.lang = s.lang \
                                       AND lower(d.printed) = lower(t))) AS leftover \
         FROM ( \
           SELECT c.lang, c.released_at, \
                  string_to_array(split_part(f.type_line, ' — ', 2), ' ') AS subs, \
                  btrim(regexp_replace(f.printed_type_line, '^.*?( — | : |～| - )', '')) AS printed \
           FROM cards c JOIN card_faces f USING (scryfall_id) \
           WHERE c.lang <> 'en' AND f.printed_type_line IS NOT NULL \
             AND f.type_line LIKE '% — %' AND f.printed_type_line ~ '( — | : |～| - )' \
         ) s WHERE array_length(s.subs, 1) > 1 \
       ) c WHERE array_length(unknown, 1) = 1 AND array_length(leftover, 1) = 1 \
     ) s \
     GROUP BY english, lang, printed \
     ORDER BY english, lang, count(*) DESC, max(released_at) DESC NULLS LAST, printed",
];

/// The mined dictionary, committed rather than derived at install time.
///
/// Mining it needs a *full* catalog — every language, 542 177 printings — and
/// CI has an empty Postgres, so the file is the artifact and
/// `baylee-catalog mine-types` is the developer's tool that rewrites it. Same
/// bargain as the `CardIndex` ledger.
const TYPE_NAMES_TSV: &str = include_str!("../../../data/type-names.tsv");

/// What each type and subtype is called, as one `INSERT`.
///
/// `ON CONFLICT DO NOTHING` so a self-hoster may correct a row and keep it,
/// exactly as [`language_seed`] allows.
fn type_name_seed() -> String {
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

/// The search, as one statement, so a test can read the shape of it.
///
/// The representative printing is picked per language, and **English asks the
/// Oracle fields**. `printed_name` and `printed_type_line` are a translation
/// abroad and a styling or an obsolete wording at home, so the preference for
/// a printing that carries a printed type line — written for the 6489 German
/// faces that have a name and no type — was heading English searches with the
/// oldest wording it could find: `birds of paradise` answered
/// `BIRDS OF PARADISE — Summon Bird` against the live catalog, a styling from
/// one Secret Lair over a type line Magic stopped printing in 1994 (#35).
///
/// It is a free function rather than a `const` because the tests below assert
/// about its *structure* — that the two tiers are unioned rather than `OR`ed,
/// and that the fence the tiers read is the one the projection writes — and a
/// constant would say nothing about where either half is used.
///
/// The `LIMIT` sits at the end of `ranked`, so the rows it counts have to be
/// rows `picked` can still resolve. `card_search` holds one row per card in
/// *every* language, and `picked` keeps only a printing in the asked-for
/// language or in English — so a card with neither would be counted against
/// the limit and then vanish, and a caller asking for twenty would be handed
/// nineteen. Eight cards in this catalog have no English printing at all
/// (the Japanese Dreamcast promos, `psdg`), which is few enough that the
/// symptom would have been read as a search that simply found less.
fn search_sql() -> &'static str {
    "\
        WITH q AS (SELECT catalog_norm($1) AS n, $1 AS raw, $2 AS lang), \
        named AS ( \
          SELECT s.oracle_id, s.face_index, \
                 CASE WHEN position('| ' || q.n || ' |' in s.names_norm) > 0 THEN 0 \
                      WHEN position('| ' || q.n in s.names_norm) > 0 THEN 1 \
                      WHEN position(' ' || q.n in s.names_norm) > 0 THEN 2 \
                      ELSE 3 END AS tier \
          FROM card_search s CROSS JOIN q \
          WHERE s.bg @> catalog_bigrams(q.n) \
            AND position(q.n in s.names_norm) > 0 \
        ), \
        texted AS ( \
          SELECT s.oracle_id, s.face_index, 4 AS tier \
          FROM card_search s CROSS JOIN q \
          WHERE s.tsv @@ plainto_tsquery('simple', q.raw) \
        ), \
        hit AS ( \
          SELECT DISTINCT ON (oracle_id) oracle_id, face_index, tier \
          FROM (SELECT * FROM named UNION ALL SELECT * FROM texted) tiers \
          ORDER BY oracle_id, tier, face_index \
        ), \
        ranked AS ( \
          SELECT h.oracle_id, h.face_index, \
                 row_number() OVER ( \
                   ORDER BY h.tier, jsonb_exists(s.names, q.lang) DESC, \
                            length(coalesce(s.names ->> q.lang, s.names ->> 'en', '')), \
                            coalesce(s.names ->> q.lang, s.names ->> 'en') \
                 ) AS nth \
          FROM hit h JOIN card_search s USING (oracle_id, face_index) CROSS JOIN q \
          WHERE jsonb_exists(s.names, q.lang) OR jsonb_exists(s.names, 'en') \
          ORDER BY nth \
          LIMIT $3 \
        ), \
        picked AS ( \
          SELECT DISTINCT ON (r.oracle_id) r.nth, \
                 c.scryfall_id::text AS scryfall_id, c.lang AS lang, \
                 CASE WHEN c.lang = 'en' THEN f.name \
                      ELSE coalesce(f.printed_name, f.name) END AS display_name, \
                 f.name AS english_name, \
                 CASE WHEN c.lang = 'en' THEN coalesce(f.type_line, '') \
                      ELSE coalesce(f.printed_type_line, f.type_line, '') \
                      END AS display_type \
          FROM ranked r \
          JOIN cards c ON c.oracle_id = r.oracle_id \
          JOIN card_faces f ON f.scryfall_id = c.scryfall_id \
                           AND f.face_index = r.face_index \
          CROSS JOIN q \
          WHERE c.lang IN (q.lang, 'en') \
          ORDER BY r.oracle_id, (c.lang = q.lang) DESC, \
                   (c.lang <> 'en' AND f.printed_type_line IS NOT NULL) DESC, \
                   c.released_at DESC NULLS LAST, c.scryfall_id \
        ) \
        SELECT scryfall_id, lang, display_name, english_name, display_type \
        FROM picked ORDER BY nth"
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name predicate and a rules-text predicate in one `OR` is the defect
    /// this projection exists to undo: no index can satisfy both at once, so
    /// Postgres builds a `tsvector` for every row instead — 2742 ms for
    /// `稲妻` against the live catalog. It would come back as an
    /// *optimisation* ("one pass over one table"), and it would still be
    /// correct, which is why nothing else would catch it.
    #[test]
    fn the_name_and_the_rules_text_are_never_asked_in_one_predicate() {
        let sql = search_sql();
        let named = sql.find("named AS").expect("the name tier");
        let texted = sql.find("texted AS").expect("the text tier");
        let between = &sql[named..texted];
        assert!(
            !between.contains(" OR "),
            "the name tier's WHERE grew an OR:\n{between}"
        );
        assert!(
            sql.contains("UNION ALL"),
            "the two tiers have to be unioned, not joined"
        );
    }

    /// The names are fenced in separators so one `position()` can ask "at the
    /// start of *a* name" across every language a card prints in. If the fill
    /// query stops writing the fence, every tier test silently answers 3 —
    /// the search still works, and ranks a substring match as highly as an
    /// exact one.
    #[test]
    fn the_fence_the_tiers_read_is_the_fence_the_projection_writes() {
        assert!(
            PROJECT_SQL.contains("'| ' || catalog_norm(") && PROJECT_SQL.contains("|| ' |'"),
            "the projection stopped fencing names_norm"
        );
        let sql = search_sql();
        assert!(
            sql.contains("position('| ' || q.n || ' |' in s.names_norm)"),
            "the exact tier stopped reading the fence"
        );
    }

    /// A flavor name belongs to a *printing*, and the inner query keeps one
    /// printing per language. Command Tower carries six of them, so folding
    /// the lookup in there would keep whichever printing `DISTINCT ON` chose
    /// and throw the other five away — a search that finds `Cybertron` and
    /// not `Croft Manor`, with nothing to show it happened. It is therefore
    /// its own join over every printing, and it has to keep writing the
    /// fence, or the names it adds rank as substrings instead of as names.
    #[test]
    fn every_flavor_name_a_card_was_ever_printed_under_is_searchable() {
        let start = PROJECT_SQL
            .find("f.flavor_name")
            .expect("the projection stopped reading flavor names");
        let join = PROJECT_SQL[..start]
            .rfind("LEFT JOIN (")
            .expect("flavor names are not inside a join of their own");
        let branch = &PROJECT_SQL[join..];
        assert!(
            !branch.contains("DISTINCT ON"),
            "the flavor-name join picks one printing per card:\n{branch}"
        );
        assert!(
            branch.contains("string_agg(DISTINCT f.flavor_name"),
            "the flavor-name join stopped collecting every printing's name"
        );
        assert!(
            branch.contains("' ' || catalog_norm(") && branch.contains("|| ' |'"),
            "the flavor names stopped being fenced like every other name"
        );
        assert!(
            PROJECT_SQL.contains("p.names_norm || coalesce(v.extra_norm, '')"),
            "the flavor names are computed and then not appended"
        );
    }

    /// A kept card has to escape **every** clause, not the last one written.
    ///
    /// The three filters were `AND`ed in a row and the keep-list was wrapped
    /// round them afterwards, which is exactly the edit an added fourth clause
    /// undoes by accident: append it after the closing bracket and an acorn
    /// land is dropped again, silently, by a rule that never mentions it.
    /// So this reads the bracket back rather than trusting the shape.
    #[test]
    fn a_card_the_repo_implements_escapes_the_whole_filter_and_not_part_of_it() {
        let start = CORPUS_SQL
            .find("AND (c.oracle_id::text = ANY(string_to_array($1")
            .map(|at| at + "AND ".len())
            .expect("the corpus stopped reading the keep-list");

        // Walk to the bracket the keep clause opened. Every filter has to be
        // inside it; what follows may only be the ORDER BY that closes the CTE.
        let body = &CORPUS_SQL[start..];
        let mut depth = 0i32;
        let mut end = body.len();
        for (at, ch) in body.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = at;
                        break;
                    }
                }
                _ => {}
            }
        }
        let inside = &body[..end];
        assert!(
            inside.contains(") OR ("),
            "the keep-list is not the left branch of an OR:\n{inside}"
        );
        for clause in ["set_type", "layout", "vintage"] {
            assert!(
                inside.contains(clause),
                "the {clause} filter is outside the keep-list's bracket, so a \
                 kept card is still dropped by it"
            );
        }
        let after = &body[end..];
        assert!(
            !after.contains("AND"),
            "a filter was added after the keep-list stopped applying:\n{after}"
        );
    }

    /// A `DROP` is the one statement here that can reach out of the schema it
    /// is run in: `CREATE … IF NOT EXISTS` builds in `current_schema()`, but
    /// a bare `DROP … IF EXISTS` resolves along the whole search path. A test
    /// puts the catalog in a sandbox schema with `public` behind it, so an
    /// unqualified drop there deletes the *developer's* index — which is
    /// exactly what happened the first time this ran.
    #[test]
    fn nothing_is_dropped_outside_the_schema_it_was_created_in() {
        for sql in schema_statements() {
            if !sql.contains("DROP ") {
                continue;
            }
            assert!(
                sql.contains("current_schema()"),
                "a drop that can reach past its own schema:\n{sql}"
            );
        }
    }

    /// Every statement has to be safe to run against an existing database,
    /// because the gateway applies them on every start.
    ///
    /// Four spellings say that, not one. `IF NOT EXISTS` creates what is
    /// missing, `IF EXISTS` drops what is left over, `CREATE OR REPLACE`
    /// writes a function body over whatever is there, and `ON CONFLICT` seeds
    /// a row without minding that it is already seeded.
    #[test]
    fn every_schema_statement_is_idempotent() {
        for sql in schema_statements() {
            assert!(
                [
                    "IF NOT EXISTS",
                    "IF EXISTS",
                    "CREATE OR REPLACE",
                    "ON CONFLICT"
                ]
                .iter()
                .any(|guard| sql.contains(guard)),
                "not idempotent: {sql}"
            );
        }
    }

    /// The projection's plain columns are written by `project()` and its one
    /// generated column by the server. Making `names_norm` generated too
    /// would be the obvious tidying-up and would be wrong: it is folded
    /// through an extension's dictionary that the search path resolves, and a
    /// `STORED` column computed from that is recomputed only when the row is
    /// written — so a rebuild could not correct it.
    #[test]
    fn only_the_bigrams_are_generated() {
        let table = schema_statements()
            .into_iter()
            .find(|s| s.contains("CREATE TABLE IF NOT EXISTS card_search"))
            .expect("the projection is part of the schema");
        assert_eq!(
            table.matches("GENERATED").count(),
            1,
            "exactly one column of the projection is the server's to compute:\n{table}"
        );
        assert!(
            table.contains("bg         text[] GENERATED"),
            "and it is the bigram array:\n{table}"
        );
    }

    /// Every language a card in the catalog is printed in has a name a person
    /// can read, and the codes are Scryfall's rather than ISO's — `zhs` and
    /// `zht` are language codes nowhere else.
    #[test]
    fn every_language_the_catalog_stores_is_named_in_itself() {
        let seed = language_seed();
        // Measured against the live catalog: `SELECT DISTINCT lang FROM cards`.
        for code in [
            "en", "ja", "fr", "de", "es", "it", "zhs", "pt", "zht", "ru", "ko", "ph", "qya", "dw",
            "grc", "ar", "la", "sa", "he",
        ] {
            assert!(seed.contains(&format!("('{code}', ")), "no name for {code}");
        }
    }

    /// The insert binds one placeholder per column, and the count is written
    /// separately from the list. Postgres would reject the batch, but only
    /// against a live database — and nothing in CI has one.
    #[test]
    fn the_insert_binds_exactly_as_many_columns_as_it_names() {
        assert_eq!(CARD_INSERT_COLUMNS.split(',').count(), CARD_COLUMNS);
    }

    /// A column added to the insert but not to the table, or added to the
    /// table but never backfilled onto an existing one, both fail only at
    /// ingest time against a real server.
    #[test]
    fn every_inserted_column_exists_and_can_be_added_to_an_older_catalog() {
        let statements = schema_statements();
        let create = statements
            .iter()
            .find(|s| s.contains("CREATE TABLE IF NOT EXISTS cards"))
            .expect("the cards table is part of the schema");
        let alter = statements
            .iter()
            .find(|s| s.starts_with("ALTER TABLE cards"))
            .expect("the upgrade path is part of the schema");
        // `scryfall_id` through `released_at` predate the printing columns and
        // are in every catalog that ever existed; the rest arrived later and
        // have to be reachable by an ALTER too.
        let original = [
            "scryfall_id",
            "oracle_id",
            "lang",
            "set_code",
            "collector_number",
            "rarity",
            "layout",
            "released_at",
        ];
        for column in CARD_INSERT_COLUMNS.split(',').map(str::trim) {
            assert!(
                create.contains(&format!("{column} ")),
                "{column} is inserted but not declared"
            );
            if !original.contains(&column) {
                assert!(
                    alter.contains(&format!("IF NOT EXISTS {column} ")),
                    "{column} is new, so an existing catalog cannot gain it"
                );
            }
        }
    }

    /// Scryfall omits `finishes` on some records rather than writing the
    /// obvious value, and an empty list reaches the picker as a card that
    /// cannot be added at all.
    #[test]
    fn a_printing_that_names_no_finish_is_still_available_plain() {
        let quiet = scryfall::Card::default();
        assert_eq!(quiet.finish_list(), vec!["nonfoil".to_string()]);

        let shiny = scryfall::Card {
            finishes: vec!["nonfoil".to_string(), "foil".to_string()],
            ..scryfall::Card::default()
        };
        assert_eq!(shiny.finish_list(), vec!["nonfoil", "foil"]);
    }

    /// The tag columns are stored joined and read back split. A trailing
    /// comma, an empty column and a single tag all have to survive that.
    #[test]
    fn a_joined_tag_column_round_trips() {
        assert_eq!(split_list(""), Vec::<String>::new());
        assert_eq!(split_list("foil"), vec!["foil"]);
        assert_eq!(
            split_list("nonfoil,foil,etched"),
            vec!["nonfoil", "foil", "etched"]
        );
        assert_eq!(
            split_list("showcase, extendedart,"),
            vec!["showcase", "extendedart"]
        );
    }

    /// A printing's wire shape is what the deck builder's picker renders, and
    /// the client defines its own struct for it — same reason as the text
    /// entry above, same protection.
    #[test]
    fn the_printing_wire_shape_is_pinned() {
        let printing = Printing {
            scryfall_id: "id".to_string(),
            oracle_id: "oid".to_string(),
            lang: "ja".to_string(),
            set: "neo".to_string(),
            set_name: "Kamigawa: Neon Dynasty".to_string(),
            collector_number: "123".to_string(),
            rarity: "rare".to_string(),
            released_at: "2022-02-18".to_string(),
            artist: "Someone".to_string(),
            finishes: vec!["nonfoil".to_string(), "foil".to_string()],
            frame_effects: vec!["showcase".to_string()],
            border_color: "black".to_string(),
            promo: false,
            name: "御守り".to_string(),
            layout: "transform".to_string(),
        };
        assert_eq!(
            serde_json::to_string(&printing).expect("serializes"),
            r#"{"scryfall_id":"id","oracle_id":"oid","lang":"ja","set":"neo","set_name":"Kamigawa: Neon Dynasty","collector_number":"123","rarity":"rare","released_at":"2022-02-18","artist":"Someone","finishes":["nonfoil","foil"],"frame_effects":["showcase"],"border_color":"black","promo":false,"name":"御守り","layout":"transform"}"#
        );
    }
}
