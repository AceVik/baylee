//! baylee-db — the account side of the gateway, in PostgreSQL.
//!
//! Accounts, session tokens, decks, confirmation links, standing answers and
//! client preferences. Everything that used to live in one JSON file that was
//! held whole in memory, re-serialized on a debounce and written back over
//! itself.
//!
//! # Why a file stopped being enough
//!
//! The file was not slow and it was not fragile — it was written
//! temp-then-rename and it never lost anything. What it could not do is
//! answer a *question*. "The account with this e-mail" was a linear scan of
//! every account; "this account's decks" was a scan of every deck. That is
//! free at ten accounts and is the whole cost at ten thousand, and there is
//! no version of the file that fixes it, because the fix is an index and a
//! file has none.
//!
//! The second reason is smaller and sharper: saving one changed row meant
//! serializing every row. A gateway that wrote a deck wrote the accounts
//! too.
//!
//! # Why not the catalog's approach
//!
//! [`baylee_catalog`] talks to the same database and deliberately writes its
//! SQL by hand, because its value is in three index definitions and two
//! queries the planner shapes. This crate is the opposite case: two dozen
//! small, ordinary reads and writes on six tables, where the thing worth
//! having is that a column's Rust type and its Postgres type cannot drift
//! apart. So the catalog keeps its statements and this keeps its entities,
//! and they are two crates rather than one for exactly that reason.
//!
//! They also own their schemas separately, which is deliberate: the
//! catalog's tables are rebuilt wholesale by an ingest and are `CREATE TABLE
//! IF NOT EXISTS`, while these are migrated in place and have to survive the
//! data in them. One migrator over both would have to pretend those are the
//! same lifecycle.
//!
//! # `UUIDv7`, where it is the right key
//!
//! An account and a deck are named by a `UUIDv7`: unique without asking the
//! database, and *time-ordered*, so a b-tree index on the primary key gets
//! inserts at the right-hand edge instead of scattered through the tree the
//! way `UUIDv4` does. They were already `UUIDv7` as strings; what changes is
//! that Postgres now knows it, which is 16 bytes instead of 36 and a
//! comparison instead of a `strcmp`.
//!
//! Three tables are keyed by something else, and that is not an oversight. A
//! session token and a confirmation link are looked up by the SHA-256 of the
//! secret in them and by nothing else, so the hash *is* the key and a second
//! id would only be a column nobody reads. A standing answer is identified by
//! the question it answers — `(account, card, ability)` — and settings by the
//! account they belong to. A surrogate key on any of those would be a way to
//! store the same answer twice.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod import;
pub mod migration;

use anyhow::{Context, Result};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait as _;
use std::time::Duration;

pub use sea_orm;

/// How many connections one gateway keeps open.
///
/// Small on purpose. A gateway is not a batch job — its queries are
/// single-row reads and writes that return in well under a millisecond, so
/// concurrency buys almost nothing past a handful of connections, and each
/// one costs a backend process on the server. The number that actually
/// matters is the other end: Postgres ships with `max_connections = 100`,
/// and a test suite that spawns three dozen gateways at a default pool of
/// ten would ask for 360 of them and fail in whichever test happened to be
/// last.
pub const DEFAULT_POOL: u32 = 8;

/// Open the database and bring it up to date.
///
/// Migrating on connect rather than from a separate command is a decision
/// about operations, not about convenience: a gateway that started against a
/// schema older than its code would answer requests wrongly rather than
/// refuse to start, and there is no supervision arrangement in which that is
/// the better failure.
///
/// # Errors
///
/// If the URL is unusable, the server unreachable, or a migration fails.
pub async fn connect(url: &str, pool: u32) -> Result<DatabaseConnection> {
    let mut options = ConnectOptions::new(url.to_owned());
    options
        .max_connections(pool)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        // A connection that has been idle for ten minutes is one the gateway
        // did not need; holding it open only keeps a backend process alive
        // on the other side.
        .idle_timeout(Duration::from_secs(600))
        .sqlx_logging(false);

    let db = Database::connect(options)
        .await
        .with_context(|| format!("connecting to {}", redacted(url)))?;

    migration::Migrator::up(&db, None)
        .await
        .context("applying migrations")?;

    Ok(db)
}

/// A database URL with its password removed, for a log line or an error.
///
/// Connection strings carry credentials and errors get pasted into bug
/// reports. `postgres://baylee:hunter2@host/db` becomes
/// `postgres://baylee@host/db`.
#[must_use]
pub fn redacted(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let Some((creds, host)) = rest.split_once('@') else {
        return url.to_owned();
    };
    let user = creds.split_once(':').map_or(creds, |(u, _)| u);
    format!("{scheme}://{user}@{host}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_password_never_reaches_a_log_line() {
        assert_eq!(
            redacted("postgres://baylee:hunter2@127.0.0.1:5432/baylee"),
            "postgres://baylee@127.0.0.1:5432/baylee"
        );
    }

    /// The three shapes that are not "user:password@host", each of which an
    /// eager splitter would mangle into something that no longer names the
    /// same server.
    #[test]
    fn a_url_with_nothing_to_hide_is_left_alone() {
        for url in [
            "postgres://127.0.0.1/baylee",
            "postgres://baylee@127.0.0.1/baylee",
            "not a url at all",
        ] {
            assert_eq!(redacted(url), url);
        }
    }
}
