//! The accounts from before Baylee Central (`docs/central-login.md`
//! §"Legacy migration"): the link fixed here when one that moved to Central
//! is first admitted. Central signs these accounts in and moves them; this
//! store keeps their game data.
#![forbid(unsafe_code)]

use super::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait, Uuid,
    sessions::central_id, source,
};
use sha2::{Digest, Sha256};

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
