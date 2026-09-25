//! Stored pictures (#301): which ones anything still claims, so that the
//! gateway can remove the files of the rest.
//!
//! A picture is claimed by an owner (an `upload` row, 000008) or by a deck
//! that names it as its sleeve or playmat, owner or not. The second half is
//! the one that matters to a player: a deck showing a picture keeps it,
//! whatever became of the rows that say who uploaded it.

use std::collections::BTreeSet;

use crate::entity::deck::{Column as DeckColumn, Entity as Decks};
use crate::entity::upload::{Column as UploadColumn, Entity as Uploads};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};

/// Every picture something still claims, by image id.
///
/// # Errors
///
/// When a query fails.
pub async fn claimed(db: &impl ConnectionTrait) -> Result<BTreeSet<String>, DbErr> {
    let mut claimed: BTreeSet<String> = Uploads::find()
        .select_only()
        .column(UploadColumn::ImageId)
        .distinct()
        .into_tuple::<String>()
        .all(db)
        .await?
        .into_iter()
        .collect();
    for column in [DeckColumn::Sleeve, DeckColumn::Playmat] {
        let named = Decks::find()
            .select_only()
            .column(column)
            .filter(column.is_not_null())
            .distinct()
            .into_tuple::<Option<String>>()
            .all(db)
            .await?;
        claimed.extend(named.into_iter().flatten());
    }
    Ok(claimed)
}
