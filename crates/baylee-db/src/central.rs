//! The gateway's side of Baylee Central: its projections of Central's
//! accounts, their leases, and the legacy accounts' cutover (`legacy`). No
//! password, session or private chat key is ever in an export.
#![forbid(unsafe_code)]

use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, IsolationLevel, Statement,
    TransactionTrait, prelude::Uuid,
};

mod legacy;
mod sessions;
pub use legacy::{Ban, LegacyAccount, adopt, legacy_accounts};
pub use sessions::{
    GatewayLease, SessionBinding, extend_session, issue_session, projection, session_binding,
    take_profile,
};

/// Snapshot size admitted by the initial encrypted account-document format.
pub const MAX_SNAPSHOT_BYTES: usize = 196_608;

const EXPORT: &str = "SELECT jsonb_build_object(
    'preferences', COALESCE((SELECT doc FROM client_settings WHERE account_id = $1), '{}'::jsonb),
    'decks', COALESCE((SELECT jsonb_agg(to_jsonb(d) || jsonb_build_object('history',
        COALESCE((SELECT jsonb_agg(to_jsonb(v) ORDER BY v.version) FROM deck_version v WHERE v.deck_id = d.id), '[]'::jsonb)) ORDER BY d.id)
        FROM deck d WHERE d.account_id = $1), '[]'::jsonb)
)::text AS snapshot FROM account WHERE id = $1 AND NOT guest";

// Bound work and intermediate JSON allocation before constructing the combined snapshot.
// The caller's repeatable-read transaction keeps this check and the export on one source state.
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

/// The account's source snapshot as the text its digest is taken over, or
/// `None` when it is larger than the first snapshot format admits.
async fn source(db: &impl ConnectionTrait, account: Uuid) -> Result<Option<String>, DbErr> {
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
        return Ok(None);
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
    Ok((text.len() <= MAX_SNAPSHOT_BYTES).then_some(text))
}

/// Whether a legacy account has moved to Central and been linked here.
///
/// # Errors
/// Reports database or invalid-identifier errors.
pub async fn linked(db: &impl ConnectionTrait, account: &str) -> Result<bool, DbErr> {
    let row = db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT central_id IS NOT NULL AS linked FROM central_account_link WHERE account_id = $1", [id(account)?.into()])).await?;
    row.map_or(Ok(false), |row| row.try_get("", "linked"))
}
