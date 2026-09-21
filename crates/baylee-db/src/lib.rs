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
//! way `UUIDv4` does. The column is a `uuid`, which is 16 bytes instead of
//! 36 and a comparison instead of a `strcmp`.
//!
//! **Postgres mints them**, not the gateway: the columns are `DEFAULT
//! uuidv7()`, which PostgreSQL 18 has natively. The reason is the property
//! above. A `UUIDv7` is time-ordered by the clock of whatever made it, so
//! several gateways behind one database each write at their own right-hand
//! edge and between them scatter the index exactly the way v4 would — and
//! the clock they disagree about is the one thing none of them can fix. One
//! database has one clock. A default is not a prohibition, so the importer
//! still writes the ids an older store already had; what it removes is the
//! gateway *choosing* one when nothing had asked it to.
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
///
/// A password is written in **two** places and this takes out both.
///
/// The first is the userinfo, and its end is the **last** `@` before the
/// query string rather than the first, because a password may hold an
/// unencoded `@` and a host may not. That is not a hypothetical shape:
/// sqlx parses `postgres://username:p@ssw0rd@hostname:5432/database` and
/// asserts the password is `p@ssw0rd`, and it parses
/// `postgres://user@hostname:password@hostname:5432/database` as the
/// username `user@hostname`. Read from the front, the first of those leaves
/// `ssw0rd` in the log line and the second leaves the password whole.
///
/// The second is the query string, where `password` is a parameter sqlx
/// reads — `postgres:///?password=some_pass` is a working connection string
/// with no userinfo at all. Its value is replaced rather than the parameter
/// dropped, so the line still says that a password was supplied, and the
/// key is compared whole: a parameter whose name merely ends in the word is
/// a different parameter.
///
/// What it cannot find is a password holding an unencoded `?`, because that
/// is where the query string begins for every reader including the one this
/// URL is going to. Percent-encoding is the answer to that, and is what the
/// syntax asks for in the first place.
#[must_use]
pub fn redacted(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let (head, query) = match rest.split_once('?') {
        Some((head, query)) => (head, Some(query)),
        None => (rest, None),
    };
    let head = match head.rsplit_once('@') {
        Some((creds, host)) => {
            let user = creds.split_once(':').map_or(creds, |(u, _)| u);
            format!("{user}@{host}")
        }
        None => head.to_owned(),
    };
    match query {
        Some(query) => format!("{scheme}://{head}?{}", without_password(query)),
        None => format!("{scheme}://{head}"),
    }
}

/// The `password` parameter's value out of a query string, with every other
/// parameter and the order they were written in left alone.
fn without_password(query: &str) -> String {
    query
        .split('&')
        .map(|param| {
            if param
                .split_once('=')
                .is_some_and(|(key, _)| key == "password")
            {
                "password=…"
            } else {
                param
            }
        })
        .collect::<Vec<_>>()
        .join("&")
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

    /// **One password proves nothing about the next one.** The test above
    /// passes on a reader that takes the *first* `@`, and these two URLs are
    /// what that reader does with a password that holds one: half of
    /// `p@ssw0rd` survives as `ssw0rd`, and a username that itself holds an
    /// `@` leaves the password whole and untouched.
    ///
    /// Neither shape is invented. They are sqlx's own
    /// `it_parses_password_with_non_ascii_chars_correctly` and
    /// `it_parses_username_with_at_sign_correctly`, which assert exactly
    /// which half is which — so this is what the database on the other end
    /// reads too, not a spelling nobody would write.
    #[test]
    fn a_password_that_holds_an_at_sign_goes_with_the_rest_of_it() {
        assert_eq!(
            redacted("postgres://username:p@ssw0rd@hostname:5432/database"),
            "postgres://username@hostname:5432/database"
        );
        assert_eq!(
            redacted("postgres://user@hostname:password@hostname:5432/database"),
            "postgres://user@hostname@hostname:5432/database",
            "the username keeps the `@` that is its own; only the password goes"
        );
        for (url, secret) in [
            (
                "postgres://username:p@ssw0rd@hostname:5432/database",
                "ssw0rd",
            ),
            (
                "postgres://user@hostname:password@hostname:5432/database",
                "password",
            ),
        ] {
            assert!(
                !redacted(url).contains(secret),
                "no part of the password is left in {}",
                redacted(url)
            );
        }
    }

    /// **The other place a password is written.** `password` is a query
    /// parameter sqlx reads, so a URL with no userinfo at all can still
    /// carry the secret — `postgres:///?password=some_pass` is a test of
    /// theirs and connects — and this function used to hand that back
    /// verbatim.
    #[test]
    fn a_password_handed_over_as_a_parameter_is_still_a_password() {
        assert_eq!(
            redacted("postgres://127.0.0.1:5432/baylee?password=hunter2"),
            "postgres://127.0.0.1:5432/baylee?password=…"
        );
        assert_eq!(
            redacted("postgres:///?password=some_pass"),
            "postgres:///?password=…",
            "no host and no user, and still something to hide"
        );
        assert_eq!(
            redacted(
                "postgres://baylee:hunter2@127.0.0.1/baylee\
                 ?sslmode=require&password=hunter2&application_name=gateway"
            ),
            "postgres://baylee@127.0.0.1/baylee\
             ?sslmode=require&password=…&application_name=gateway",
            "both places at once, and every other parameter in its own order"
        );
        assert_eq!(
            redacted("postgres://127.0.0.1/baylee?options[password]=x"),
            "postgres://127.0.0.1/baylee?options[password]=x",
            "a key that merely ends in the word is a different parameter"
        );
    }

    /// The four shapes that are not "user:password@host", each of which an
    /// eager splitter would mangle into something that no longer names the
    /// same server.
    ///
    /// The last of them is why the search stops at the query string: an `@`
    /// in a parameter's value is not a credential, and a reader taking the
    /// first one cut that URL down to `postgres://127.0.0.1@b`.
    #[test]
    fn a_url_with_nothing_to_hide_is_left_alone() {
        for url in [
            "postgres://127.0.0.1/baylee",
            "postgres://baylee@127.0.0.1/baylee",
            "not a url at all",
            "postgres://127.0.0.1:5432/baylee?application_name=a@b",
        ] {
            assert_eq!(redacted(url), url);
        }
    }
}
