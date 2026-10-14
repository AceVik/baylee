//! Deploy-time upgrades of authoritative catalog facts.
//!
//! Schema and search projection versions cannot prove that a download read
//! newly required fields. This separate stamp is written only after ingest
//! finishes and every stored printing has its rules metadata. Gateway startup
//! never downloads a feed; deployment calls `baylee-catalog upgrade` explicitly.

use crate::{Catalog, ingest};
use anyhow::{Context, Result, ensure};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, SqlxPostgresConnector, Statement,
    TransactionTrait,
};
use std::{future::Future, io::BufRead};

#[derive(PartialEq, Eq)]
enum State {
    Empty,
    Current,
    Stale,
}

/// Bump when another authoritative field needs to be read from the provider.
const INGEST_VERSION: &str = "1";
const INGEST_KEY: &str = "authoritative_ingest";

/// What a deploy-time upgrade did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// No catalog was installed; populating it remains an operator choice.
    Empty,
    /// A completed ingest already supplies this build's required facts.
    Current,
    /// Downloaded and stored this many printings.
    Refreshed(usize),
}

/// Refreshes an older populated catalog once, using every language.
///
/// Intended for deployment preparation, not gateway startup. No feed is
/// requested on a current or empty catalog. An interrupted ingest invalidates its
/// completion stamp; the next explicit invocation retries it. A concurrent upgrade
/// fails promptly rather than downloading a second copy or reporting success.
///
/// # Errors
/// When another upgrade holds the lock, download or ingest fails, or the
/// provider still cannot supply all stored printings' rules facts.
pub async fn bulk(catalog: &Catalog) -> Result<Outcome> {
    run(catalog, || {
        ingest::bulk(catalog, ingest::Feed::AllLanguages)
    })
    .await
}

/// Applies the same upgrade policy to a supplied authoritative JSONL feed.
///
/// Like [`ingest::from_feed`], this makes the download boundary replaceable
/// without replacing the real database, upsert, projection or version checks.
/// `open` is never called for a current or empty catalog.
///
/// # Errors
/// The same failures as [`bulk`], with reader errors in place of HTTP errors.
pub async fn from_feed<R: BufRead>(
    catalog: &Catalog,
    open: impl FnOnce() -> Result<R> + Send + 'static,
) -> Result<Outcome> {
    run(catalog, || ingest::from_feed(catalog, open)).await
}

async fn run<F: Future<Output = Result<usize>>>(
    catalog: &Catalog,
    refresh: impl FnOnce() -> F,
) -> Result<Outcome> {
    // The lock must not borrow the only connection from a one-slot caller
    // pool. Clone its connection options and callbacks (including search_path)
    // into a temporary pool dedicated to this one advisory-lock transaction.
    let pool = catalog.db.get_postgres_connection_pool();
    let lock_pool = pool
        .options()
        .clone()
        .max_connections(1)
        .min_connections(0)
        .connect_with((*pool.connect_options()).clone())
        .await
        .context("opening the dedicated upgrade connection")?;
    let lock_db = SqlxPostgresConnector::from_sqlx_postgres_pool(lock_pool);
    let result = run_locked(catalog, refresh, &lock_db).await;
    lock_db
        .close()
        .await
        .context("closing the upgrade connection")?;
    result
}

async fn run_locked<F: Future<Output = Result<usize>>>(
    catalog: &Catalog,
    refresh: impl FnOnce() -> F,
    lock_db: &DatabaseConnection,
) -> Result<Outcome> {
    // Transaction-scoped so errors/cancellation release it. Namespace by
    // schema: installations and integration-test sandboxes are independent.
    let lock = lock_db.begin().await.context("opening the upgrade lock")?;
    let row = lock
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_try_advisory_xact_lock(\
             hashtextextended(current_schema() || ':baylee-catalog-upgrade', 0)) AS acquired",
        ))
        .await?
        .context("upgrade lock returned no row")?;
    ensure!(
        row.try_get::<bool>("", "acquired")?,
        "catalog upgrade already running; retry later"
    );

    let result = refresh_if_stale(catalog, refresh).await;
    // Await rollback even on a download error so an immediate retry can
    // acquire the lock; do not rely on an asynchronous transaction drop.
    lock.rollback()
        .await
        .context("releasing the upgrade lock")?;
    result
}

async fn refresh_if_stale<F: Future<Output = Result<usize>>>(
    catalog: &Catalog,
    refresh: impl FnOnce() -> F,
) -> Result<Outcome> {
    // Recheck under the lock, so a second deploy observes the first's stamp.
    let outcome = match state(catalog).await? {
        State::Empty => Outcome::Empty,
        State::Current => Outcome::Current,
        State::Stale => {
            tracing::info!("catalog authoritative facts need a full refresh");
            let stored = refresh().await?;
            ensure!(
                stored > 0,
                "catalog upgrade feed contained no usable printings"
            );
            ensure!(
                state(catalog).await? == State::Current,
                "catalog upgrade left missing rules metadata; no completion stamp was written"
            );
            Outcome::Refreshed(stored)
        }
    };
    Ok(outcome)
}

async fn state(catalog: &Catalog) -> Result<State> {
    let row = catalog
        .db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT EXISTS (SELECT 1 FROM cards) AS populated, \
         EXISTS (SELECT 1 FROM cards WHERE rules_metadata IS NULL) AS missing, \
         (SELECT value FROM catalog_meta WHERE key = $1) AS version",
            [INGEST_KEY.into()],
        ))
        .await?
        .context("catalog ingest version returned no row")?;
    if !row.try_get::<bool>("", "populated")? {
        return Ok(State::Empty);
    }
    let version: Option<String> = row.try_get("", "version")?;
    if version.as_deref() == Some(INGEST_VERSION) && !row.try_get::<bool>("", "missing")? {
        Ok(State::Current)
    } else {
        Ok(State::Stale)
    }
}

/// A completed feed may still leave older rows it did not contain. Never
/// declare those facts known, including after an English-only refresh of a
/// multilingual database. Only fully covered ingests earn this stamp.
pub(crate) async fn stamp_if_complete(catalog: &Catalog) -> Result<()> {
    catalog
        .db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO catalog_meta (key, value) \
         SELECT $1, $2 WHERE EXISTS (SELECT 1 FROM cards) \
         AND NOT EXISTS (SELECT 1 FROM cards WHERE rules_metadata IS NULL) \
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
            [INGEST_KEY.into(), INGEST_VERSION.into()],
        ))
        .await
        .context("stamping the authoritative ingest version")?;
    Ok(())
}

/// Invalidate before a feed can change even one printing. A failed feed must
/// not inherit the successful completion stamp of the preceding ingest.
pub(crate) async fn invalidate(catalog: &Catalog) -> Result<()> {
    catalog
        .db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM catalog_meta WHERE key = $1",
            [INGEST_KEY.into()],
        ))
        .await
        .context("invalidating the authoritative ingest version")?;
    Ok(())
}
