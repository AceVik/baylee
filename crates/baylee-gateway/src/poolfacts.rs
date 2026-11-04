//! `GET /pool/facts`: what a client filters a pool card by, beyond its text.
//!
//! The table is `baylee_cardtext::PoolFactsBody`, made once per catalog stamp
//! from what `Catalog::pool_records` reads and kept as the bytes it was
//! serialized to, as [`crate::texts`] keeps `/pool`'s: it is the same for
//! every reader until an ingest changes it, and it does not depend on the
//! language.
//!
//! Its `version` is a hash of the rest and not the catalog's stamp, because
//! the stamp is this database's own count: a restored catalog goes back to an
//! older one, and two gateways on two catalogs may hold the same number. A
//! client names the version it keeps (`have`) and is told `304` while the
//! table is the same one.
//!
//! This is the gateway's own derived status for the game's own pool, for a
//! signed-in session as `/pool` is, and not a feed of Scryfall's fields: the
//! terms say "You may not simply repackage, republish, or proxy Scryfall data"
//! (`docs/legal.md` §3), so what no client feature reads is not in it.

use crate::{ErrorBody, Shared};
use anyhow::Context;
use axum::extract::{Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use baylee_catalog::{CardLine, Catalog, PoolFacts, PoolFactsBody, PoolRecords};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// The table at one catalog stamp.
pub struct Held {
    /// The catalog's data version this was read at.
    stamp: i64,
    /// The hash a client keeps it by; empty when it says nothing.
    version: String,
    /// The whole answer, serialized once.
    body: bytes::Bytes,
    /// The same table as a deck check reads it. `None` when it says nothing,
    /// which a check takes as no table at all.
    pub(crate) table: Option<Arc<PoolFacts>>,
}

/// The table this process holds, read again when the catalog's stamp moves.
#[derive(Default)]
pub struct PoolFactsCache {
    /// A burst of requests after a stamp waits here and reads it once.
    held: tokio::sync::Mutex<Option<Arc<Held>>>,
    /// How many times the table was read from the catalog: what the tests
    /// hold "once per stamp" against.
    reads: AtomicU64,
}

impl PoolFactsCache {
    /// The table, read again when the catalog moved since it was last read.
    ///
    /// The stamp is read before the table, so a write landing mid-read leaves
    /// it stamped older than what it may already contain and the next request
    /// reads it again. Held at a different stamp and not an older one, for
    /// the reason [`crate::texts::TextCache::get`] gives.
    ///
    /// # Errors
    /// When the catalog cannot be read. Nothing is kept then.
    pub async fn get(&self, catalog: &Catalog) -> anyhow::Result<Arc<Held>> {
        let mut stamp = catalog.data_version().await?;
        let mut held = self.held.lock().await;
        if let Some(table) = held.as_ref() {
            // Newer than this request read: another request built it after
            // an ingest moved the stamp, or the catalog went back. Asking
            // again tells them apart.
            if table.stamp > stamp {
                stamp = catalog.data_version().await?;
            }
            if table.stamp == stamp {
                return Ok(Arc::clone(table));
            }
        }
        self.reads.fetch_add(1, Ordering::Relaxed);
        let records = catalog.pool_records(crate::texts::pool_cards()).await?;
        let table = Arc::new(Held::of(stamp, &records)?);
        *held = Some(Arc::clone(&table));
        Ok(table)
    }
}

impl Held {
    fn of(stamp: i64, records: &PoolRecords) -> anyhow::Result<Self> {
        let mut body = assemble(records);
        if !body.cards.is_empty() {
            let unstamped = serde_json::to_vec(&body).context("serializing the pool's table")?;
            body.version = hash(&unstamped);
        }
        let bytes = serde_json::to_vec(&body).context("serializing the pool's table")?;
        let version = body.version.clone();
        let table = (!body.cards.is_empty()).then(|| Arc::new(PoolFacts::from(body)));
        Ok(Self {
            stamp,
            version,
            body: bytes.into(),
            table,
        })
    }
}

/// The table for the pool's cards, by registry index. A card the catalog
/// knows nothing about has no row, which is what a client takes as "not
/// known" rather than as a card with no printings.
fn assemble(records: &PoolRecords) -> PoolFactsBody {
    let lines = baylee_cards::pool::rows()
        .iter()
        .filter_map(|card| {
            let legalities = records.verdicts.get(card.oracle_id);
            let printed = records.printed.get(card.oracle_id);
            if legalities.is_none() && printed.is_none() {
                return None;
            }
            Some(CardLine {
                index: card.index,
                legalities: legalities.cloned().unwrap_or_default(),
                printed: printed.cloned().unwrap_or_default(),
            })
        })
        .collect();
    PoolFactsBody::assemble(records.sets.clone(), lines)
}

/// The first 16 hex digits of the SHA-256 of `bytes`.
fn hash(bytes: &[u8]) -> String {
    crate::auth::hex_lower(&Sha256::digest(bytes)[..8])
}

/// Query for `GET /pool/facts`.
/// @openapi
#[derive(Deserialize)]
pub struct PoolFactsQuery {
    /// @description The `version` of the table the client already keeps.
    /// @internal
    /// The version the client keeps, to be told it is still the one.
    have: Option<String>,
}

/// `GET /pool/facts`: the pool's table.
///
/// A gateway with no catalog, or one that is down, answers an empty table
/// rather than an error, for the reason `/pool` serves the registry alone:
/// a client then says these filters are not available here, and nothing else
/// about it changes.
///
/// @route GET /pool/facts
/// @scope public
/// @summary What a client filters a pool card by
/// @description Every format's verdict on each pool card the catalog knows,
/// and the sets and rarities it was printed in, in one compact table
/// (`docs/protocol.md` §"Pool facts"). Answers `304` with no body when `have`
/// names the current `version`. Empty without a filled catalog. For a
/// signed-in session, a guest's included.
/// @tag Catalog
/// @security bearer
/// @insert `SessionRefusals`
/// @query-params `$PoolFactsQuery`
/// @return 200: `$PoolFactsBody`
/// @return 304: "`have` names the table this gateway holds"
pub async fn pool_facts(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(params): Query<PoolFactsQuery>,
) -> Result<Response, (StatusCode, Json<ErrorBody>)> {
    crate::authed(&state, &headers).await?;
    let Some(catalog) = state.catalog.as_ref() else {
        return Ok(Json(PoolFactsBody::default()).into_response());
    };
    match state.pool_facts.get(catalog).await {
        Ok(held) if !held.version.is_empty() && params.have.as_ref() == Some(&held.version) => {
            Ok(StatusCode::NOT_MODIFIED.into_response())
        }
        Ok(held) => Ok(([(CONTENT_TYPE, "application/json")], held.body.clone()).into_response()),
        Err(e) => {
            tracing::warn!(%e, "reading the pool's table failed; answering an empty one");
            Ok(Json(PoolFactsBody::default()).into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    //! The cache, against a real catalog in a schema of its own: when it reads
    //! is a question about a stamp another connection moves. The route, with
    //! its session and its `304`, is `tests/e2e_pool_facts.rs`.

    use super::*;
    use baylee_catalog::scryfall;
    use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
    use std::sync::atomic::AtomicU32;

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
                "t_poolfacts_{what}_{}_{}",
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

    /// Mind Stone, a pool card, as a catalog would hold one printing of it.
    fn stone(n: u32, word: &str) -> scryfall::Card {
        let card = baylee_cards::pool::rows()
            .iter()
            .find(|c| c.english_name == "Mind Stone")
            .expect("Mind Stone is in the pool");
        scryfall::Card {
            id: format!("00000000-0000-4000-8000-{n:012}"),
            oracle_id: Some(card.oracle_id.to_owned()),
            lang: "en".to_owned(),
            set: "fic".to_owned(),
            set_name: Some("Final Fantasy Commander".to_owned()),
            collector_number: n.to_string(),
            rarity: Some("uncommon".to_owned()),
            name: "Mind Stone".to_owned(),
            type_line: Some("Artifact".to_owned()),
            legalities: [("modern".to_owned(), word.to_owned())].into(),
            ..scryfall::Card::default()
        }
    }

    fn reads(cache: &PoolFactsCache) -> u64 {
        cache.reads.load(Ordering::Relaxed)
    }

    /// Read once per stamp, and the version moves with what the table says
    /// and not with the stamp.
    #[tokio::test]
    async fn the_table_is_read_once_until_the_catalog_moves() {
        let sandbox = Sandbox::open("once").await;
        let catalog = &sandbox.catalog;
        catalog.upsert(&[stone(1, "legal")]).await.expect("seeding");
        let cache = PoolFactsCache::default();

        let first = cache.get(catalog).await.expect("reading");
        assert!(!first.version.is_empty());
        let again = cache.get(catalog).await.expect("reading");
        assert!(Arc::ptr_eq(&first, &again));
        assert_eq!(reads(&cache), 1);

        catalog.bump_data_version().await.expect("the stamp");
        let same = cache.get(catalog).await.expect("reading");
        assert_eq!(reads(&cache), 2, "the stamp moved, so it was read");
        assert_eq!(same.version, first.version, "nothing in it changed");

        catalog.forget_verdicts().await.expect("forgetting");
        catalog
            .upsert(&[stone(1, "banned")])
            .await
            .expect("an ingest's batch");
        catalog.bump_data_version().await.expect("the stamp");
        let changed = cache.get(catalog).await.expect("reading");
        assert_ne!(changed.version, first.version);
        sandbox.close().await;
    }

    /// An empty catalog sends no version, so no client keeps it as a table.
    #[tokio::test]
    async fn a_catalog_with_nothing_in_it_has_no_version() {
        let sandbox = Sandbox::open("empty").await;
        let cache = PoolFactsCache::default();
        let held = cache.get(&sandbox.catalog).await.expect("reading");
        assert_eq!(held.version, "");
        let body: PoolFactsBody = serde_json::from_slice(&held.body).expect("reading it back");
        assert!(body.cards.is_empty());
        sandbox.close().await;
    }

    #[test]
    fn a_hash_is_sixteen_hex_digits() {
        assert_eq!(hash(b"table"), hash(b"table"));
        assert_eq!(hash(b"table").len(), 16);
        assert_ne!(hash(b"table"), hash(b"tables"));
    }
}
