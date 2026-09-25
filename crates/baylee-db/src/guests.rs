//! Guests (#269): accounts handed out on request, with no username, no
//! address and no password (migration 000007 holds a guest to that).
//!
//! Nothing signs in as a guest, so its session is the only way into it, and
//! a guest no live session leads to is an account nobody can reach again.
//! This is where such a guest goes, with its decks, their history and its
//! preferences (the cascades of 000001).

use crate::accounts::{Gone, delete_where};
use sea_orm::{ConnectionTrait, DbErr};
use time::OffsetDateTime;
use uuid::Uuid;

/// Deletes every guest that no session live at `now` leads to, or only
/// `only` when it is such a guest, and answers what went
/// ([`crate::accounts`]).
///
/// An account that is not a guest is never touched, whatever its sessions.
///
/// # Errors
///
/// When the delete fails.
pub async fn purge(
    db: &impl ConnectionTrait,
    now: OffsetDateTime,
    only: Option<Uuid>,
) -> Result<Gone, DbErr> {
    const UNREACHABLE: &str = "a.guest AND NOT EXISTS \
         (SELECT 1 FROM session_token s WHERE s.account_id = a.id AND s.expires_at > $1)";
    match only {
        None => delete_where(db, UNREACHABLE, vec![now.into()]).await,
        Some(account) => {
            delete_where(
                db,
                &format!("{UNREACHABLE} AND a.id = $2"),
                vec![now.into(), account.into()],
            )
            .await
        }
    }
}
