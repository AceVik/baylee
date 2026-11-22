//! Keyset selection of one original private deck and one historical save.
use super::{Deck, DeckVersion, DeckVersions, Decks, deck, deck_version};
use sea_orm::{
    AccessMode, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IsolationLevel, QueryFilter,
    QueryOrder, TransactionTrait,
};
use uuid::Uuid;

pub(crate) async fn read(
    db: &DatabaseConnection,
    owner: Uuid,
    after: Option<Uuid>,
    wanted: Option<Uuid>,
    version: i32,
) -> Result<Option<(Deck, Option<DeckVersion>)>, DbErr> {
    let txn = db
        .begin_with_config(
            Some(IsolationLevel::RepeatableRead),
            Some(AccessMode::ReadOnly),
        )
        .await?;
    let mut rows = Decks::find()
        .filter(deck::Column::AccountId.eq(owner))
        .filter(deck::Column::Kind.eq("account"));
    if let Some(id) = wanted {
        rows = rows.filter(deck::Column::Id.eq(id));
    }
    if let Some(id) = after {
        rows = rows.filter(deck::Column::Id.gt(id));
    }
    let result = match rows.order_by_asc(deck::Column::Id).one(&txn).await? {
        Some(head) => {
            let old = DeckVersions::find()
                .filter(deck_version::Column::DeckId.eq(head.id))
                .filter(deck_version::Column::Version.gt(version))
                .order_by_asc(deck_version::Column::Version)
                .one(&txn)
                .await?;
            Some((head.into(), old.map(Into::into)))
        }
        None => None,
    };
    txn.commit().await?;
    Ok(result)
}
