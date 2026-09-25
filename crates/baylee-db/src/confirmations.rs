//! Confirmation links (#293): a link that expired confirms nothing, so the
//! gateway's sweeper deletes it ([`sweep`]) rather than keeping a row that
//! names an account for as long as the table lasts.

use crate::entity::confirmation::{Column, Entity as Confirmation};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use time::OffsetDateTime;

/// Deletes every confirmation link that has expired by `now`, answering how
/// many went.
///
/// `expires_at <= now`, the same moment `GET /auth/confirm` starts refusing
/// the link, on the index 000001 made for it.
///
/// # Errors
///
/// When the delete fails.
pub async fn sweep(db: &impl ConnectionTrait, now: OffsetDateTime) -> Result<u64, DbErr> {
    Ok(Confirmation::delete_many()
        .filter(Column::ExpiresAt.lte(now))
        .exec(db)
        .await?
        .rows_affected)
}
