//! Confirmation links (#293): a link that expired confirms nothing, so the
//! gateway's sweeper deletes it ([`sweep`]) rather than keeping a row that
//! names an account for as long as the table lasts.

use crate::entity::confirmation::{Column, Entity as Confirmation};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use time::OffsetDateTime;

/// Deletes every confirmation link that has expired by `now`, answering how
/// many went.
///
/// `expires_at <= now`, on the index 000001 made for it. No link is made since
/// the cutover (`docs/central-login.md`); the ones left go within a day.
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
