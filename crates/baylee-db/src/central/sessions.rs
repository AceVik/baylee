//! Gateway-owned projections and opaque Central authorization leases.
#![forbid(unsafe_code)]

use super::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait, Uuid,
};

/// A server-only lease, never included in public gateway responses or diagnostics.
pub struct GatewayLease {
    /// Opaque Central session capability, usable only with this gateway's credential.
    pub lease: [u8; 32],
    /// Immutable Central identity bound to the local projection.
    pub central: [u8; 16],
    /// Absolute Unix-second expiration; local activity cannot renew it.
    pub expires_at: i64,
    /// Closed authority: either player admission or gateway administration.
    pub purpose: String,
}

/// Whether a local session needs a live Central authorization check.
pub enum SessionBinding {
    /// A traditional local account or guest.
    Local,
    /// A Central projection, with its server-only lease.
    Central(GatewayLease),
    /// Missing session, mapping, lease or expired Central authorization.
    Rejected,
}

pub(super) fn central_id(value: [u8; 16]) -> Result<Uuid, DbErr> {
    let id = Uuid::from_bytes(value);
    if id.get_version_num() != 7 || value[8] & 0xc0 != 0x80 {
        return Err(DbErr::Custom("invalid central identity".into()));
    }
    Ok(id)
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

/// Issue a local session atomically with its non-renewable Central lease.
///
/// # Errors
/// Refuses malformed tokens, expired leases, missing accounts and storage errors.
pub async fn issue_session(
    db: &DatabaseConnection,
    account: &str,
    token_hash: Vec<u8>,
    lease: [u8; 32],
    expires_at: i64,
    purpose: &str,
) -> Result<(), DbErr> {
    if token_hash.len() != 32 || !matches!(purpose, "admit" | "admin") {
        return Err(DbErr::Custom("invalid session".into()));
    }
    let account = super::id(account)?;
    let transaction = db.begin().await?;
    transaction
        .execute_unprepared("SET LOCAL statement_timeout = '3s'; SET LOCAL lock_timeout = '2s'")
        .await?;
    let inserted = transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO session_token (token_hash, account_id, expires_at)
         SELECT $1, id, to_timestamp($3::bigint) FROM account
         WHERE id = $2 AND central_only AND to_timestamp($3::bigint) > clock_timestamp()",
            [token_hash.clone().into(), account.into(), expires_at.into()],
        ))
        .await?;
    if inserted.rows_affected() != 1 {
        return Err(DbErr::Custom("central session unavailable".into()));
    }
    transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO central_gateway_session (token_hash, lease, expires_at, purpose) VALUES ($1, $2, to_timestamp($3::bigint), $4)",
        [token_hash.into(), lease.to_vec().into(), expires_at.into(), purpose.into()])).await?;
    transaction.commit().await
}

/// Move a Central-bound session's lease expiry forward to `expires_at`, once
/// Central renewed it (baylee-central `docs/operations.md` §"Sessions"); never
/// backwards. The lease bounds the session (`session_binding`); the bearer's
/// own expiry renews itself on use like any session's.
///
/// # Errors
/// Reports storage failures; nothing is moved then.
pub async fn extend_session(
    db: &DatabaseConnection,
    lease: [u8; 32],
    expires_at: i64,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE central_gateway_session SET expires_at = to_timestamp($2::bigint)
         WHERE lease = $1 AND expires_at < to_timestamp($2::bigint)",
        [lease.to_vec().into(), expires_at.into()],
    ))
    .await?;
    Ok(())
}

/// Inspect whether a bearer session must still pass Central introspection.
/// A Central-only account without a lease fails closed, including after restart.
///
/// # Errors
/// Reports storage failures without authorizing the request.
pub async fn session_binding(
    db: &DatabaseConnection,
    token_hash: Vec<u8>,
) -> Result<SessionBinding, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT a.central_only, l.lease, l.purpose, c.central_id,
                floor(extract(epoch FROM l.expires_at))::bigint AS expires_at,
                l.expires_at > clock_timestamp() AS live
         FROM session_token s JOIN account a ON a.id = s.account_id
         LEFT JOIN central_gateway_session l ON l.token_hash = s.token_hash
         LEFT JOIN central_account_link c ON c.account_id = a.id
         WHERE s.token_hash = $1",
            [token_hash.into()],
        ))
        .await?;
    let Some(row) = row else {
        return Ok(SessionBinding::Rejected);
    };
    if !row.try_get::<bool>("", "central_only")? {
        return Ok(SessionBinding::Local);
    }
    let lease: Option<Vec<u8>> = row.try_get("", "lease")?;
    let central: Option<Uuid> = row.try_get("", "central_id")?;
    let expires_at: Option<i64> = row.try_get("", "expires_at")?;
    let live: Option<bool> = row.try_get("", "live")?;
    let purpose: Option<String> = row.try_get("", "purpose")?;
    let (Some(lease), Some(central), Some(expires_at), Some(true)) =
        (lease, central, expires_at, live)
    else {
        return Ok(SessionBinding::Rejected);
    };
    let lease = lease
        .try_into()
        .map_err(|_| DbErr::Custom("invalid lease".into()))?;
    Ok(SessionBinding::Central(GatewayLease {
        lease,
        central: *central.as_bytes(),
        expires_at,
        purpose: purpose.ok_or_else(|| DbErr::Custom("invalid lease purpose".into()))?,
    }))
}
