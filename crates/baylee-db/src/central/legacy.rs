//! The accounts from before Baylee Central, at the cutover that hands their
//! sign-in and their move to Central (`docs/central-login.md` §"Legacy
//! migration"): what Central's legacy store takes from here, and the link
//! fixed here when a moved account is first admitted.
#![forbid(unsafe_code)]

use super::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, IsolationLevel, Statement,
    TransactionTrait, Uuid, sessions::central_id, source,
};
use sea_orm::AccessMode;
use sha2::{Digest, Sha256};

/// The most accounts one export carries; the official gateway has six.
pub const MAX_LEGACY_ACCOUNTS: usize = 1000;

/// A ban in force when the export was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ban {
    /// Until when, Unix seconds; `None` for good.
    pub until: Option<i64>,
}

/// One account that signs in here with a password and has not moved to
/// Central: everything its move needs, and nothing else (no address, no
/// session, no chat key).
pub struct LegacyAccount {
    /// Its immutable id here, which Central keeps as the legacy id.
    pub id: String,
    /// The key its username is unique under; `None` for an account that
    /// signs in only with an address, which Central cannot take.
    pub username_key: Option<String>,
    /// The name other players see, as it is stored.
    pub display_name: String,
    /// Its tag.
    pub tag: i32,
    /// Its Argon2id PHC string.
    pub password_hash: String,
    /// The version of this gateway's terms it last accepted.
    pub terms_version: Option<String>,
    /// An admin reset its password and it has not chosen its own yet.
    pub must_change_password: bool,
    /// Its ban, when one is in force.
    pub ban: Option<Ban>,
    /// Its source snapshot as the text the digest is taken over; `None` when
    /// it is larger than the first snapshot format admits.
    pub snapshot: Option<String>,
}

/// Every account that still signs in here with a password, ordered by id, as
/// one consistent read. The caller stops the gateway first: a password
/// changed after this read would leave Central a stale hash.
///
/// # Errors
/// Refuses more than [`MAX_LEGACY_ACCOUNTS`] and reports unavailable storage.
pub async fn legacy_accounts(db: &DatabaseConnection) -> Result<Vec<LegacyAccount>, DbErr> {
    let transaction = db
        .begin_with_config(
            Some(IsolationLevel::RepeatableRead),
            Some(AccessMode::ReadOnly),
        )
        .await?;
    let rows = transaction
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            format!(
                "SELECT a.id, a.username_key, a.display_name, a.tag, a.password_hash,
                     a.terms_version, a.must_change_password,
                     a.banned_at IS NOT NULL
                         AND (a.banned_until IS NULL OR a.banned_until > clock_timestamp()) AS banned,
                     floor(extract(epoch FROM a.banned_until))::bigint AS banned_until
                 FROM account a
                 WHERE NOT a.guest AND NOT a.central_only AND a.password_hash IS NOT NULL
                     AND NOT EXISTS (SELECT 1 FROM central_account_link c
                         WHERE c.account_id = a.id AND c.central_id IS NOT NULL)
                 ORDER BY a.id LIMIT {}",
                MAX_LEGACY_ACCOUNTS + 1
            ),
        ))
        .await?;
    if rows.len() > MAX_LEGACY_ACCOUNTS {
        return Err(DbErr::Custom(
            "more legacy accounts than one export takes".into(),
        ));
    }
    let mut accounts = Vec::with_capacity(rows.len());
    for row in rows {
        let id: Uuid = row.try_get("", "id")?;
        let banned: bool = row.try_get("", "banned")?;
        accounts.push(LegacyAccount {
            id: id.to_string(),
            username_key: row.try_get("", "username_key")?,
            display_name: row.try_get("", "display_name")?,
            tag: row.try_get("", "tag")?,
            password_hash: row.try_get("", "password_hash")?,
            terms_version: row.try_get("", "terms_version")?,
            must_change_password: row.try_get("", "must_change_password")?,
            ban: banned
                .then(|| row.try_get("", "banned_until"))
                .transpose()?
                .map(|until| Ban { until }),
            snapshot: source(&transaction, id).await?,
        });
    }
    transaction.commit().await?;
    Ok(accounts)
}

/// Fix the link of a legacy account to the Central identity its move closed
/// under, at its first admission here: Central names the legacy account to
/// the bridge only once the move is closed. The account's password and its
/// password sessions end with it, as at the cutover the gateway itself ran
/// before. The same link again changes nothing; answers whether it was made
/// now.
///
/// # Errors
/// Refuses a guest, an unknown account, a link to another identity, an
/// identity linked to another account, and unavailable storage.
pub async fn adopt(
    db: &DatabaseConnection,
    account: &str,
    central: [u8; 16],
) -> Result<bool, DbErr> {
    let account = super::id(account)?;
    let central = central_id(central)?;
    let transaction = db.begin().await?;
    transaction
        .execute_unprepared("SET LOCAL statement_timeout = '3s'; SET LOCAL lock_timeout = '2s'")
        .await?;
    // The same lock a projection's creation takes for this identity.
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 673298421))",
            [central.to_string().into()],
        ))
        .await?;
    let guest: bool = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT guest FROM account WHERE id = $1 FOR UPDATE",
            [account.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("no such account".into()))?
        .try_get("", "guest")?;
    if guest {
        return Err(DbErr::Custom("a guest has no legacy link".into()));
    }
    let linked: Option<Option<Uuid>> = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT central_id FROM central_account_link WHERE account_id = $1 FOR UPDATE",
            [account.into()],
        ))
        .await?
        .map(|row| row.try_get("", "central_id"))
        .transpose()?;
    match linked.flatten() {
        Some(previous) if previous == central => {
            transaction.commit().await?;
            return Ok(false);
        }
        Some(_) => return Err(DbErr::Custom("conflicting central identity".into())),
        None => {}
    }
    if transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT 1 AS taken FROM central_account_link WHERE central_id = $1",
            [central.into()],
        ))
        .await?
        .is_some()
    {
        return Err(DbErr::Custom("central identity linked elsewhere".into()));
    }
    // What the gateway holds at the link: the source Central's store was
    // given, since nothing here changes it while the account cannot sign in.
    let digest = Sha256::digest(
        source(&transaction, account)
            .await?
            .ok_or_else(|| DbErr::Custom("migration snapshot exceeds supported size".into()))?
            .as_bytes(),
    )
    .to_vec();
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_account_link (account_id, central_id, snapshot_digest, linked_at)
             VALUES ($1, $2, $3, clock_timestamp())
             ON CONFLICT (account_id) DO UPDATE SET central_id = EXCLUDED.central_id,
                 snapshot_digest = EXCLUDED.snapshot_digest, linked_at = EXCLUDED.linked_at,
                 source_credential_hash = NULL
             WHERE central_account_link.central_id IS NULL",
            [account.into(), central.into(), digest.into()],
        ))
        .await?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE account SET central_only = true, password_hash = NULL WHERE id = $1",
            [account.into()],
        ))
        .await?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM session_token WHERE account_id = $1",
            [account.into()],
        ))
        .await?;
    transaction.commit().await?;
    Ok(true)
}
