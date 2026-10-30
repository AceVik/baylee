//! Source-preserving link to Central; no password, session or private chat keys in exports.
#![forbid(unsafe_code)]

use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, IsolationLevel, Statement,
    TransactionTrait, prelude::Uuid,
};
use sha2::{Digest, Sha256};

mod link;
mod sessions;
pub use link::{StoredLink, clear_link, link, set_link};
pub use sessions::{
    GatewayLease, SessionBinding, extend_session, issue_session, projection, session_binding,
    take_profile,
};

/// Snapshot size admitted by the initial encrypted account-document format.
pub const MAX_SNAPSHOT_BYTES: usize = 196_608;

/// Consistent preferences/deck export with a gateway-held digest for cutover reconciliation.
pub struct Snapshot {
    /// Account-owned preferences and complete owned-deck rows/history.
    pub document: serde_json::Value,
    /// Digest of the canonical source snapshot; not a credential.
    pub digest: [u8; 32],
}

const EXPORT: &str = "SELECT jsonb_build_object(
    'preferences', COALESCE((SELECT doc FROM client_settings WHERE account_id = $1), '{}'::jsonb),
    'decks', COALESCE((SELECT jsonb_agg(to_jsonb(d) || jsonb_build_object('history',
        COALESCE((SELECT jsonb_agg(to_jsonb(v) ORDER BY v.version) FROM deck_version v WHERE v.deck_id = d.id), '[]'::jsonb)) ORDER BY d.id)
        FROM deck d WHERE d.account_id = $1), '[]'::jsonb)
)::text AS snapshot FROM account WHERE id = $1 AND NOT guest";

// Bound work and intermediate JSON allocation before constructing the combined snapshot.
// Repeatable-read plus the gateway barrier keeps this check and the export on one source state.
const SIZE: &str = "SELECT sum(bytes)::bigint AS bytes, count(*)::bigint AS rows FROM (
    (SELECT octet_length(to_jsonb(d)::text) AS bytes FROM deck d WHERE d.account_id = $1 LIMIT 2049)
    UNION ALL
    (SELECT octet_length(to_jsonb(v)::text) AS bytes FROM deck_version v JOIN deck d ON d.id = v.deck_id WHERE d.account_id = $1 LIMIT 8193)
    UNION ALL
    (SELECT octet_length(doc::text) AS bytes FROM client_settings WHERE account_id = $1)
) sizes";

fn id(value: &str) -> Result<Uuid, DbErr> {
    Uuid::parse_str(value).map_err(|_| DbErr::Custom("invalid account".into()))
}

async fn read(db: &impl ConnectionTrait, account: Uuid) -> Result<Snapshot, DbErr> {
    let sizes = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            SIZE,
            [account.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("source unavailable".into()))?;
    let bytes: Option<i64> = sizes.try_get("", "bytes")?;
    let rows: i64 = sizes.try_get("", "rows")?;
    if bytes.unwrap_or_default() > 196_608 || rows > 2048 {
        return Err(DbErr::Custom(
            "migration snapshot exceeds supported size".into(),
        ));
    }
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            EXPORT,
            [account.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("account unavailable".into()))?;
    let text: String = row.try_get("", "snapshot")?;
    if text.len() > MAX_SNAPSHOT_BYTES {
        return Err(DbErr::Custom(
            "migration snapshot exceeds supported size".into(),
        ));
    }
    let digest = Sha256::digest(text.as_bytes()).into();
    let document = serde_json::from_str(&text)
        .map_err(|_| DbErr::Custom("invalid migration snapshot".into()))?;
    Ok(Snapshot { document, digest })
}

/// Export after fresh legacy password verification. The source is never removed or modified.
/// The gateway's migration barrier must exclude in-flight account mutations for the call.
///
/// # Errors
/// Refuses linked accounts, oversized exports or unavailable storage without partial linking.
pub async fn prepare(db: &DatabaseConnection, account: &str) -> Result<Snapshot, DbErr> {
    snapshot(db, account, true).await
}

/// Refresh a previously proven migration without authorizing a new password generation.
///
/// # Errors
/// Refuses missing, linked or credential-changed migrations and oversized snapshots.
pub async fn refresh(db: &DatabaseConnection, account: &str) -> Result<Snapshot, DbErr> {
    snapshot(db, account, false).await
}

async fn snapshot(
    db: &DatabaseConnection,
    account: &str,
    fresh_proof: bool,
) -> Result<Snapshot, DbErr> {
    let account = id(account)?;
    let transaction = db
        .begin_with_config(Some(IsolationLevel::RepeatableRead), None)
        .await?;
    transaction
        .execute_unprepared("SET LOCAL statement_timeout = '3s'; SET LOCAL lock_timeout = '2s'")
        .await?;
    let snapshot = read(&transaction, account).await?;
    let statement = if fresh_proof {
        "INSERT INTO central_account_link (account_id, snapshot_digest, source_credential_hash)
         SELECT id, $2, password_hash FROM account WHERE id = $1 AND NOT central_only AND password_hash IS NOT NULL
         ON CONFLICT (account_id) DO UPDATE SET snapshot_digest = EXCLUDED.snapshot_digest,
             prepared_at = clock_timestamp(), source_credential_hash = EXCLUDED.source_credential_hash
         WHERE central_account_link.central_id IS NULL"
    } else {
        "UPDATE central_account_link c SET snapshot_digest = $2, prepared_at = clock_timestamp()
         FROM account a WHERE c.account_id = $1 AND a.id = c.account_id AND c.central_id IS NULL
         AND a.password_hash IS NOT NULL AND c.source_credential_hash = a.password_hash"
    };
    let saved = transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            statement,
            [account.into(), snapshot.digest.to_vec().into()],
        ))
        .await?;
    if saved.rows_affected() != 1 {
        return Err(DbErr::Custom("account already linked".into()));
    }
    transaction.commit().await?;
    Ok(snapshot)
}

/// Check the cutover prerequisites before irrevocably closing legacy enrollment at Central.
/// The caller keeps the exclusive gateway barrier until `finish` completes.
///
/// # Errors
/// Refuses changed credentials/content, missing preparation or conflicting identity.
pub async fn validate_cutover(
    db: &DatabaseConnection,
    account: &str,
    central: [u8; 16],
    digest: [u8; 32],
) -> Result<(), DbErr> {
    let transaction = db
        .begin_with_config(Some(IsolationLevel::RepeatableRead), None)
        .await?;
    transaction
        .execute_unprepared("SET LOCAL statement_timeout = '3s'; SET LOCAL lock_timeout = '2s'")
        .await?;
    validate(
        &transaction,
        id(account)?,
        Uuid::from_bytes(central),
        digest,
    )
    .await?;
    transaction.commit().await
}

async fn validate(
    db: &impl ConnectionTrait,
    account: Uuid,
    central: Uuid,
    restored_digest: [u8; 32],
) -> Result<bool, DbErr> {
    let row = db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT c.central_id, c.snapshot_digest, c.source_credential_hash = a.password_hash AS credential_matches
         FROM central_account_link c JOIN account a ON a.id = c.account_id
         WHERE c.account_id = $1 FOR UPDATE OF c, a", [account.into()])).await?
        .ok_or_else(|| DbErr::Custom("migration not prepared".into()))?;
    let previous: Option<Uuid> = row.try_get("", "central_id")?;
    if let Some(previous) = previous {
        if previous != central {
            return Err(DbErr::Custom("conflicting central identity".into()));
        }
        return Ok(true);
    }
    if row.try_get::<Option<bool>>("", "credential_matches")? != Some(true) {
        return Err(DbErr::Custom("source credentials changed".into()));
    }
    let expected: Vec<u8> = row.try_get("", "snapshot_digest")?;
    let current = read(db, account).await?;
    if expected != current.digest || expected != restored_digest {
        return Err(DbErr::Custom("source changed during migration".into()));
    }
    Ok(false)
}

/// Whether a legacy account has cut over. A linked account can never use the old password route.
///
/// # Errors
/// Reports database or invalid-identifier errors.
pub async fn linked(db: &impl ConnectionTrait, account: &str) -> Result<bool, DbErr> {
    let row = db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT central_id IS NOT NULL AS linked FROM central_account_link WHERE account_id = $1", [id(account)?.into()])).await?;
    row.map_or(Ok(false), |row| row.try_get("", "linked"))
}

/// Whether local legacy proof has prepared an enrollment that must be invalidated
/// centrally before changing its source credential.
///
/// # Errors
/// Reports unavailable storage or malformed local account identifiers.
pub async fn pending(db: &impl ConnectionTrait, account: &str) -> Result<bool, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT central_id IS NULL AS pending FROM central_account_link WHERE account_id = $1",
            [id(account)?.into()],
        ))
        .await?;
    row.map_or(Ok(false), |row| row.try_get("", "pending"))
}

/// Whether an unfinished migration has a source-generation proof from this schema version.
///
/// # Errors
/// Reports unavailable storage or malformed local account identifiers.
pub async fn prepared(db: &impl ConnectionTrait, account: &str) -> Result<bool, DbErr> {
    let row = db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT central_id IS NULL AND source_credential_hash IS NOT NULL AS prepared FROM central_account_link WHERE account_id = $1",
        [id(account)?.into()])).await?;
    row.map_or(Ok(false), |row| row.try_get("", "prepared"))
}

/// Whether this immutable legacy account is linked to the authenticated central identity.
///
/// # Errors
/// Reports unavailable storage or invalid account identifiers.
pub async fn matches(
    db: &impl ConnectionTrait,
    account: &str,
    central: [u8; 16],
) -> Result<bool, DbErr> {
    let row = db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT central_id = $2 AS matches FROM central_account_link WHERE account_id = $1 AND central_id IS NOT NULL",
        [id(account)?.into(), Uuid::from_bytes(central).into()])).await?;
    row.map_or(Ok(false), |row| row.try_get("", "matches"))
}

/// Reconcile a fresh central admission with its original account and cut over once.
/// Caller must hold the exclusive migration barrier until the new gateway session is issued.
/// Original decks/settings remain available for gameplay and recovery.
///
/// # Errors
/// Refuses a changed source snapshot, a conflicting central link or storage failure.
pub async fn finish(
    db: &DatabaseConnection,
    account: &str,
    central: [u8; 16],
    restored_digest: [u8; 32],
) -> Result<(), DbErr> {
    let account = id(account)?;
    let transaction = db
        .begin_with_config(Some(IsolationLevel::RepeatableRead), None)
        .await?;
    transaction
        .execute_unprepared("SET LOCAL statement_timeout = '3s'; SET LOCAL lock_timeout = '2s'")
        .await?;
    let central = Uuid::from_bytes(central);
    if validate(&transaction, account, central, restored_digest).await? {
        transaction.commit().await?;
        return Ok(());
    }
    transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_account_link SET central_id = $2, linked_at = clock_timestamp(), source_credential_hash = NULL WHERE account_id = $1",
        [account.into(), central.into()])).await?;
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
    transaction.commit().await
}
