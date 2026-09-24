//! Guests (#269): accounts handed out on request, with no username, no
//! address and no password (migration 000007 holds a guest to that).
//!
//! Nothing signs in as a guest, so its session is the only way into it, and
//! a guest no live session leads to is an account nobody can reach again.
//! This is where such a guest goes, with its decks, their history and its
//! preferences (the cascades of 000001).

use sea_orm::{ConnectionTrait, DbErr, Statement};
use time::OffsetDateTime;
use uuid::Uuid;

/// Deletes every guest that no session live at `now` leads to, or only
/// `only` when it is such a guest, and says how many went.
///
/// An account that is not a guest is never touched, whatever its sessions.
/// A copy another player made of one of a guest's decks stays, pointing at
/// nothing (`deck_copied_from` is `SET NULL`).
///
/// # Errors
///
/// When the delete fails.
pub async fn purge(
    db: &impl ConnectionTrait,
    now: OffsetDateTime,
    only: Option<Uuid>,
) -> Result<u64, DbErr> {
    const UNREACHABLE: &str = "DELETE FROM account a WHERE a.guest AND NOT EXISTS \
         (SELECT 1 FROM session_token s WHERE s.account_id = a.id AND s.expires_at > $1)";
    let backend = db.get_database_backend();
    let statement = match only {
        None => Statement::from_sql_and_values(backend, UNREACHABLE, [now.into()]),
        Some(account) => Statement::from_sql_and_values(
            backend,
            format!("{UNREACHABLE} AND a.id = $2"),
            [now.into(), account.into()],
        ),
    };
    Ok(db.execute_raw(statement).await?.rows_affected())
}
