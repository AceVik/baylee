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
        let mut sql = String::from(
            "INSERT INTO cards \
             (scryfall_id, oracle_id, lang, set_code, collector_number, rarity, layout, released_at) \
             VALUES ",
        );
        let mut values: Vec<Value> = Vec::with_capacity(cards.len() * 8);
        for (i, card) in cards.iter().enumerate() {
            let base = i * 8;
            if i > 0 {
                sql.push(',');
            }
            let _ = write!(
                sql,
                "(${}::uuid,${}::uuid,${},${},${},${},${},${})",
                base + 1,
                base + 2,
                base + 3,
                base + 4,
                base + 5,
                base + 6,
                base + 7,
                base + 8
            );
            values.push(Value::from(card.id.clone()));
            values.push(Value::from(card.oracle_id.clone()));
            values.push(Value::from(card.lang.clone()));
            values.push(Value::from(card.set.clone()));
            values.push(Value::from(card.collector_number.clone()));
            values.push(Value::from(card.rarity.clone()));
            values.push(Value::from(card.layout.clone()));
            values.push(Value::from(card.released_at.clone()));
        }
        sql.push_str(
            " ON CONFLICT (scryfall_id) DO UPDATE SET \
             oracle_id = EXCLUDED.oracle_id, lang = EXCLUDED.lang, \
             set_code = EXCLUDED.set_code, collector_number = EXCLUDED.collector_number, \
             rarity = EXCLUDED.rarity, layout = EXCLUDED.layout, \
             released_at = EXCLUDED.released_at, updated_at = now()",
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
            updated_at       timestamptz NOT NULL DEFAULT now()
        )"
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
}
