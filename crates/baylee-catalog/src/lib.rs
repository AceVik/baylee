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
//! # Why hand-written SQL under an ORM
//!
//! The whole value of this crate is in three index definitions and two
//! queries: a lateral join that resolves "the same card in my language", and a
//! trigram/full-text search that has to stay on its index. Both are shaped by
//! the query planner rather than by the entity model, so they are written as
//! SQL and `SeaORM` supplies the pool, the parameter binding and the backend
//! abstraction.
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
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, Value};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// Text for one face of a card, already resolved to a language.
///
/// The localized and English forms are both carried: a client needs the
/// English name to recognise whether the object on the table is still the card
/// this text describes (a clone is not), and it cannot do that from a
/// translated name.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct FaceText {
    /// Name in the served language.
    pub name: String,
    /// English name of the same face.
    pub english_name: String,
    /// Type line in the served language.
    pub type_line: String,
    /// Rules text in the served language.
    pub oracle_text: String,
    /// Mana cost in Scryfall notation (language-independent).
    pub mana_cost: String,
}

/// Every face of one requested printing.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CardTextEntry {
    /// The printing id that was asked for.
    pub scryfall_id: String,
    /// The language actually served, after the fallback.
    pub lang: String,
    /// Faces in printed order.
    pub faces: Vec<FaceText>,
}

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

/// The searchable text of a face, as one expression.
///
/// Written once because the GIN index and the `WHERE` clause must be
/// character-for-character identical or Postgres silently falls back to a
/// sequential scan over every printing — which is exactly the failure this
/// crate exists to avoid, and it is invisible until the table is large.
const SEARCH_EXPR: &str = "to_tsvector('simple', \
     coalesce(f.printed_name, f.name) || ' ' || \
     coalesce(f.printed_type_line, f.type_line, '') || ' ' || \
     coalesce(f.printed_text, f.oracle_text, ''))";

/// The columns one printing binds in `upsert_cards`, in bind order.
///
/// The placeholder run, the value pushes and the table definition are written
/// three places apart; a mismatch makes Postgres reject the whole batch, so
/// the list lives here and the tests hold the other two against it.
const CARD_INSERT_COLUMNS: &str = "scryfall_id, oracle_id, lang, set_code, collector_number, \
     rarity, layout, released_at, set_name, artist, finishes, frame_effects, border_color, promo";

/// How many columns that is.
const CARD_COLUMNS: usize = 14;

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

    /// Creates the schema if it is not there yet.
    ///
    /// Idempotent, so it is safe to run on every gateway start; the ingest
    /// calls it too, so a fresh database needs no separate migration step.
    ///
    /// # Errors
    /// When a statement fails — most often a missing `pg_trgm` extension on a
    /// server where the role may not create extensions.
    pub async fn migrate(&self) -> Result<()> {
        for sql in schema_statements() {
            self.db
                .execute_raw(Statement::from_string(DbBackend::Postgres, sql.clone()))
                .await
                .with_context(|| format!("applying schema statement: {sql}"))?;
        }
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
    /// # Errors
    /// When the query fails.
    pub async fn search(&self, query: &str, lang: &str, limit: u64) -> Result<Vec<SearchHit>> {
        let sql = format!(
            "SELECT c.scryfall_id::text AS scryfall_id, c.lang AS lang, \
                    coalesce(f.printed_name, f.name) AS display_name, \
                    f.name AS english_name, \
                    coalesce(f.printed_type_line, f.type_line, '') AS display_type \
             FROM card_faces f \
             JOIN cards c ON c.scryfall_id = f.scryfall_id \
             WHERE c.lang IN ($2, 'en') \
               AND ({SEARCH_EXPR} @@ plainto_tsquery('simple', $1) \
                    OR coalesce(f.printed_name, f.name) ILIKE '%' || $1 || '%') \
             ORDER BY (c.lang = $2) DESC, \
                      similarity(coalesce(f.printed_name, f.name), $1) DESC \
             LIMIT $3"
        );
        let rows = self
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
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
        let sql = "\
            SELECT c.scryfall_id::text AS scryfall_id, c.oracle_id::text AS oracle_id, \
                   c.lang AS lang, c.set_code AS set_code, \
                   coalesce(c.set_name, '') AS set_name, \
                   c.collector_number AS collector_number, \
                   coalesce(c.rarity, '') AS rarity, \
                   coalesce(c.released_at, '') AS released_at, \
                   coalesce(c.artist, '') AS artist, \
                   coalesce(c.finishes, 'nonfoil') AS finishes, \
                   coalesce(c.frame_effects, '') AS frame_effects, \
                   coalesce(c.border_color, '') AS border_color, \
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
        Ok(storable.len())
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
            for k in 0..CARD_COLUMNS {
                if k > 0 {
                    sql.push(',');
                }
                // The two uuid columns come first; everything after them is
                // text or boolean, which Postgres infers from the column.
                let cast = if k < 2 { "::uuid" } else { "" };
                let _ = write!(sql, "${}{cast}", base + k + 1);
            }
            sql.push(')');
            values.push(Value::from(card.id.clone()));
            values.push(Value::from(card.oracle_id.clone()));
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
        }
        sql.push_str(
            " ON CONFLICT (scryfall_id) DO UPDATE SET \
             oracle_id = EXCLUDED.oracle_id, lang = EXCLUDED.lang, \
             set_code = EXCLUDED.set_code, collector_number = EXCLUDED.collector_number, \
             rarity = EXCLUDED.rarity, layout = EXCLUDED.layout, \
             released_at = EXCLUDED.released_at, set_name = EXCLUDED.set_name, \
             artist = EXCLUDED.artist, finishes = EXCLUDED.finishes, \
             frame_effects = EXCLUDED.frame_effects, border_color = EXCLUDED.border_color, \
             promo = EXCLUDED.promo, updated_at = now()",
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
             (scryfall_id, face_index, name, printed_name, type_line, printed_type_line, \
              oracle_text, printed_text, mana_cost, power, toughness, loyalty) VALUES ",
        );
        let mut values: Vec<Value> = Vec::new();
        let mut n = 0usize;
        for card in cards {
            for (index, face) in card.faces().into_iter().enumerate() {
                let base = n * 12;
                if n > 0 {
                    sql.push(',');
                }
                let _ = write!(
                    sql,
                    "(${}::uuid,${},${},${},${},${},${},${},${},${},${},${})",
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
                    base + 12
                );
                values.push(Value::from(card.id.clone()));
                values.push(Value::from(index as i16));
                values.push(Value::from(face.name));
                values.push(Value::from(face.printed_name));
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
}

/// A comma-joined column back into a list, without empty entries.
///
/// `finishes` and `frame_effects` are short, closed sets of tags; storing them
/// as text keeps the whole crate on one `Value` type and one parameter
/// binding, and neither is ever queried *into* — only read back with the row.
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
fn schema_statements() -> Vec<String> {
    // The index is declared on the table itself, so it carries no alias; the
    // query joins the table as `f`. Same expression, one prefix apart.
    let indexed_expr = SEARCH_EXPR.replace("f.", "");
    vec![
        // Trigram matching for "name starts with / contains", which a deck
        // builder's type-ahead needs and full-text search cannot do.
        "CREATE EXTENSION IF NOT EXISTS pg_trgm".to_string(),
        "CREATE TABLE IF NOT EXISTS cards (
            scryfall_id      uuid PRIMARY KEY,
            oracle_id        uuid NOT NULL,
            lang             text NOT NULL,
            set_code         text NOT NULL DEFAULT '',
            collector_number text NOT NULL DEFAULT '',
            rarity           text,
            layout           text,
            released_at      text,
            set_name         text,
            artist           text,
            finishes         text NOT NULL DEFAULT 'nonfoil',
            frame_effects    text NOT NULL DEFAULT '',
            border_color     text,
            promo            boolean NOT NULL DEFAULT false,
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
             ADD COLUMN IF NOT EXISTS promo boolean NOT NULL DEFAULT false"
            .to_string(),
        "CREATE TABLE IF NOT EXISTS card_faces (
            scryfall_id       uuid NOT NULL REFERENCES cards(scryfall_id) ON DELETE CASCADE,
            face_index        smallint NOT NULL,
            name              text NOT NULL DEFAULT '',
            printed_name      text,
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
        // The lookup that serves every game: identity, then language.
        "CREATE INDEX IF NOT EXISTS cards_oracle_lang ON cards (oracle_id, lang)".to_string(),
        format!(
            "CREATE INDEX IF NOT EXISTS card_faces_search ON card_faces USING gin ({indexed_expr})"
        ),
        "CREATE INDEX IF NOT EXISTS card_faces_name_trgm ON card_faces \
         USING gin (coalesce(printed_name, name) gin_trgm_ops)"
            .to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The index and the `WHERE` clause are built from the same constant, but
    /// the index drops the table alias. If that ever diverges the search still
    /// returns correct rows — by scanning the whole table — so no test would
    /// notice without pinning the shape here.
    #[test]
    fn the_search_index_matches_the_search_expression() {
        let index = schema_statements()
            .into_iter()
            .find(|s| s.contains("card_faces_search"))
            .expect("the search index is part of the schema");
        let aliasless = SEARCH_EXPR.replace("f.", "");
        assert!(
            index.contains(&aliasless),
            "index expression drifted from SEARCH_EXPR:\n{index}"
        );
    }

    /// The mirror of `baylee_client_core::card_face`'s wire test. The client
    /// deliberately defines its own structs — it cannot depend on this crate
    /// without pulling an ORM and a Postgres driver into a wasm build — so the
    /// only thing keeping the two ends together is that both pin this JSON.
    #[test]
    fn the_client_wire_shape_is_pinned() {
        let entry = CardTextEntry {
            scryfall_id: "id".to_string(),
            lang: "de".to_string(),
            faces: vec![FaceText {
                name: "Wald".to_string(),
                english_name: "Forest".to_string(),
                type_line: "Basisland — Wald".to_string(),
                oracle_text: "({T}: Erzeuge {G}.)".to_string(),
                mana_cost: String::new(),
            }],
        };
        assert_eq!(
            serde_json::to_string(&entry).expect("serializes"),
            r#"{"scryfall_id":"id","lang":"de","faces":[{"name":"Wald","english_name":"Forest","type_line":"Basisland — Wald","oracle_text":"({T}: Erzeuge {G}.)","mana_cost":""}]}"#
        );
    }

    /// Every statement has to be safe to run against an existing database,
    /// because the gateway applies them on every start.
    #[test]
    fn every_schema_statement_is_idempotent() {
        for sql in schema_statements() {
            assert!(sql.contains("IF NOT EXISTS"), "not idempotent: {sql}");
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
        };
        assert_eq!(
            serde_json::to_string(&printing).expect("serializes"),
            r#"{"scryfall_id":"id","oracle_id":"oid","lang":"ja","set":"neo","set_name":"Kamigawa: Neon Dynasty","collector_number":"123","rarity":"rare","released_at":"2022-02-18","artist":"Someone","finishes":["nonfoil","foil"],"frame_effects":["showcase"],"border_color":"black","promo":false,"name":"御守り"}"#
        );
    }
}
