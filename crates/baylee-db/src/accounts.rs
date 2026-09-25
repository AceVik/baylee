//! Deleting accounts (#292): a player's own, on request, and the guests no
//! session leads to any more ([`crate::guests::purge`]).
//!
//! One statement per deletion, and the rows an account owns go with it by
//! the cascades of 000001 and 000008: its sessions, confirmation links,
//! decks with their history, preferences and its claims on uploaded
//! pictures.
//!
//! The pictures themselves are files, which this crate never sees. So a
//! deletion answers which pictures the accounts claimed, and the caller
//! removes those that nobody claims any more.

use sea_orm::{ConnectionTrait, DbErr, Statement, Value};
use uuid::Uuid;

/// What a deletion took.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gone {
    /// The accounts deleted.
    pub accounts: Vec<Uuid>,
    /// Every picture one of them claimed, once each. Another account may
    /// still claim one of these.
    pub pictures: Vec<String>,
}

/// Deletes the one account `account`, answering what went. No such account
/// is an empty [`Gone`], not an error.
///
/// # Errors
///
/// When the delete fails.
pub async fn delete(db: &impl ConnectionTrait, account: Uuid) -> Result<Gone, DbErr> {
    delete_where(db, "a.id = $1", vec![account.into()]).await
}

/// Deletes every account `condition` holds for, `a` being the account, and
/// answers what went.
///
/// The pictures are read in the same statement as the delete, from the
/// snapshot it started with, so the claims its cascade removes are still
/// there to be read.
pub(crate) async fn delete_where(
    db: &impl ConnectionTrait,
    condition: &str,
    values: Vec<Value>,
) -> Result<Gone, DbErr> {
    let sql = format!(
        "WITH gone AS (DELETE FROM account a WHERE {condition} RETURNING a.id) \
         SELECT g.id, u.image_id FROM gone g LEFT JOIN upload u ON u.account_id = g.id"
    );
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            sql,
            values,
        ))
        .await?;
    let mut gone = Gone::default();
    for row in rows {
        gone.accounts.push(row.try_get("", "id")?);
        if let Some(picture) = row.try_get::<Option<String>>("", "image_id")? {
            gone.pictures.push(picture);
        }
    }
    gone.accounts.sort_unstable();
    gone.accounts.dedup();
    gone.pictures.sort_unstable();
    gone.pictures.dedup();
    Ok(gone)
}
