//! Closed-beta keys (#317): made by the operator, spent by registrations.
//!
//! Hand-written statements, as in [`crate::records`]: the one thing worth
//! having here is that spending a key is a single `UPDATE` which either takes
//! a use or finds nothing to take, so two registrations racing for a key's
//! last use cannot both have it. The caller runs [`redeem`] inside the
//! transaction that writes the account, so a registration that fails after
//! it (a taken username) gives the use back by rolling back.
//!
//! No function here sees a key: only its SHA-256 (`key_hash`), which is what
//! the table holds. And nothing reads the hash back out: [`Invite`] has no
//! field for it.

use sea_orm::{ConnectionTrait, DbErr, Statement};
use time::OffsetDateTime;
use uuid::Uuid;

/// A key about to be stored.
#[derive(Clone, Debug)]
pub struct NewInvite {
    /// SHA-256 of the key's canonical form.
    pub key_hash: Vec<u8>,
    /// Who it is for, in the operator's words; at most 100 characters (the
    /// table refuses more).
    pub note: Option<String>,
    /// How many accounts it may admit.
    pub uses: i32,
    /// When it stops admitting anybody, if ever.
    pub expires_at: Option<OffsetDateTime>,
}

/// A stored key, as `invite list` shows it. Never the key, nor its hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invite {
    /// Its id, which `invite revoke` takes.
    pub id: Uuid,
    /// When it was made.
    pub created_at: OffsetDateTime,
    /// Who it is for.
    pub note: Option<String>,
    /// How many more accounts it may admit.
    pub uses_left: i32,
    /// When it stops admitting anybody, if ever.
    pub expires_at: Option<OffsetDateTime>,
    /// When it was revoked, if it was.
    pub revoked_at: Option<OffsetDateTime>,
    /// How many accounts that still exist it admitted.
    pub admitted: i64,
}

/// Stores a key, answering its id.
///
/// # Errors
///
/// When the insert fails: the same hash twice (which a random key never
/// is), a note over 100 characters, or a negative count.
pub async fn create(db: &impl ConnectionTrait, new: &NewInvite) -> Result<Uuid, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO invite (key_hash, note, uses_left, expires_at) \
             VALUES ($1, $2, $3, $4) RETURNING id",
            [
                new.key_hash.clone().into(),
                new.note.clone().into(),
                new.uses.into(),
                new.expires_at.into(),
            ],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("the insert answered no id".into()))?;
    row.try_get("", "id")
}

/// Every stored key, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn list(db: &impl ConnectionTrait) -> Result<Vec<Invite>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            "SELECT i.id, i.created_at, i.note, i.uses_left, i.expires_at, i.revoked_at, \
                    (SELECT count(*) FROM account a WHERE a.invite_id = i.id) AS admitted \
             FROM invite i ORDER BY i.created_at, i.id",
        ))
        .await?;
    rows.into_iter()
        .map(|row| {
            Ok(Invite {
                id: row.try_get("", "id")?,
                created_at: row.try_get("", "created_at")?,
                note: row.try_get("", "note")?,
                uses_left: row.try_get("", "uses_left")?,
                expires_at: row.try_get("", "expires_at")?,
                revoked_at: row.try_get("", "revoked_at")?,
                admitted: row.try_get("", "admitted")?,
            })
        })
        .collect()
}

/// Revokes the key `id` as of `now`. Answers whether it did: `false` for no
/// such key, or one already revoked, whose first revocation stands.
///
/// # Errors
///
/// When the update fails.
pub async fn revoke(
    db: &impl ConnectionTrait,
    id: Uuid,
    now: OffsetDateTime,
) -> Result<bool, DbErr> {
    let done = db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE invite SET revoked_at = $2 WHERE id = $1 AND revoked_at IS NULL",
            [id.into(), now.into()],
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Spends one use of the key whose hash is `key_hash`, if it has one left,
/// has not expired by `now` and is not revoked, and answers its id; `None`
/// otherwise, whichever of those it was.
///
/// One statement: a second caller waiting on the same row reads it again
/// once the first commits, so a key with one use left admits one of them.
/// Run it in the transaction that writes the account.
///
/// # Errors
///
/// When the update fails.
pub async fn redeem(
    db: &impl ConnectionTrait,
    key_hash: &[u8],
    now: OffsetDateTime,
) -> Result<Option<Uuid>, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE invite SET uses_left = uses_left - 1 \
             WHERE key_hash = $1 AND uses_left > 0 AND revoked_at IS NULL \
               AND (expires_at IS NULL OR expires_at > $2) \
             RETURNING id",
            [key_hash.to_vec().into(), now.into()],
        ))
        .await?;
    row.map(|row| row.try_get("", "id")).transpose()
}
