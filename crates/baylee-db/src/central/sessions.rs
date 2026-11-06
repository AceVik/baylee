//! Gateway-owned projections of Central accounts. Their sessions live in memory
//! (`baylee-gateway` `sessions.rs`), never here.
#![forbid(unsafe_code)]

use super::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait, Uuid,
};

pub(super) fn central_id(value: [u8; 16]) -> Result<Uuid, DbErr> {
    let id = Uuid::from_bytes(value);
    if id.get_version_num() != 7 || value[8] & 0xc0 != 0x80 {
        return Err(DbErr::Custom("invalid central identity".into()));
    }
    Ok(id)
}

/// The account `central` is linked to here, if it is linked yet.
///
/// # Errors
/// Refuses non-v7 identities, and reports unavailable storage.
pub async fn linked_account(
    db: &DatabaseConnection,
    central: [u8; 16],
) -> Result<Option<String>, DbErr> {
    let central = central_id(central)?;
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT account_id FROM central_account_link WHERE central_id = $1",
            [central.into()],
        ))
        .await?;
    row.map(|row| {
        row.try_get::<Uuid>("", "account_id")
            .map(|id| id.to_string())
    })
    .transpose()
}

/// Resolve/create a local projection exclusively by immutable Central ID.
/// Private login names and e-mail addresses are never used for account linking.
///
/// # Errors
/// Refuses non-v7 identities, mapping conflicts and unavailable storage.
pub async fn projection(
    db: &DatabaseConnection,
    central: [u8; 16],
    allow_create: bool,
) -> Result<String, DbErr> {
    let central = central_id(central)?;
    let transaction = db.begin().await?;
    transaction
        .execute_unprepared("SET LOCAL statement_timeout = '3s'; SET LOCAL lock_timeout = '2s'")
        .await?;
    // Concurrent first admission serializes on the same Central identifier. The lock
    // is only a creation aid; the unique index remains the identity authority.
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 673298421))",
            [central.to_string().into()],
        ))
        .await?;
    if let Some(row) = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT account_id FROM central_account_link WHERE central_id = $1",
            [central.into()],
        ))
        .await?
    {
        let account: Uuid = row.try_get("", "account_id")?;
        transaction.commit().await?;
        return Ok(account.to_string());
    }
    if !allow_create {
        return Err(DbErr::Custom("gateway admission closed".into()));
    }
    let row = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "INSERT INTO account (display_name, created_at, lang, guest, central_only)
         VALUES ('Player', clock_timestamp(), '', false, true) RETURNING id",
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("projection unavailable".into()))?;
    let account: Uuid = row.try_get("", "id")?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_account_link (account_id, central_id, snapshot_digest, linked_at)
         VALUES ($1, $2, $3, clock_timestamp())",
            [account.into(), central.into(), vec![0_u8; 32].into()],
        ))
        .await?;
    transaction.commit().await?;
    Ok(account.to_string())
}

/// Copy the display name and tag other players see at Central onto a
/// projection, at each of its sign-ins, so that a player is the same
/// `name#tag` here as on the website. A missing name or tag keeps the local
/// one; so does a tag another account here already holds, which the unique
/// index would refuse. Answers whether the row was written.
///
/// # Errors
/// Refuses a malformed account id and unavailable storage.
pub async fn take_profile(
    db: &impl ConnectionTrait,
    account: &str,
    display_name: Option<&str>,
    tag: Option<i32>,
) -> Result<bool, DbErr> {
    let account = super::id(account)?;
    let updated = db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE account SET
                 display_name = COALESCE($2::text, display_name),
                 tag = CASE WHEN $3::integer IS NULL OR EXISTS
                     (SELECT 1 FROM account other WHERE other.tag = $3::integer AND other.id <> $1)
                     THEN tag ELSE $3::integer END
             WHERE id = $1 AND central_only
               AND (display_name IS DISTINCT FROM COALESCE($2::text, display_name)
                    OR tag IS DISTINCT FROM COALESCE($3::integer, tag))",
            [
                account.into(),
                display_name.map(str::to_owned).into(),
                tag.into(),
            ],
        ))
        .await?;
    Ok(updated.rows_affected() == 1)
}
