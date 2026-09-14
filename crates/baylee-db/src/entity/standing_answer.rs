//! A remembered answer to one optional ability.

use sea_orm::entity::prelude::*;

/// "Always gain the life from this trigger", stored once and replayed into
/// every game the account sits down to.
///
/// The engine addresses an ability by `AbilityRef { card, index }`, a handle
/// that names a *card* and says nothing about a particular game — which is
/// exactly what makes the answer storable at all, and why it belongs to the
/// account rather than to the table.
///
/// The key is the question: `(account, card, ability)`. A surrogate id would
/// be a way to store two different answers to one question.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "standing_answer")]
pub struct Model {
    /// Whose answer it is.
    #[sea_orm(primary_key, auto_increment = false)]
    pub account_id: Uuid,
    /// Registry index of the card the ability is printed on.
    ///
    /// `i64` for a `u32`, and `card` takes it for the same reason `ability`
    /// must: Postgres has no unsigned integer, and the reserved ability
    /// indices count *down* from `u32::MAX`, so an `int4` column cannot hold
    /// them at all. Storing the pair as one type keeps the conversion at the
    /// two edges rather than in the middle of a row.
    #[sea_orm(primary_key, auto_increment = false)]
    pub card: i64,
    /// Index into that card's ability list; the reserved values above
    /// `AbilityRef::FIRST_RESERVED` name the abilities no card prints.
    #[sea_orm(primary_key, auto_increment = false)]
    pub ability: i64,
    /// What to answer without asking.
    pub yes: bool,
}

/// The owning account.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Cascades.
    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::AccountId",
        to = "super::account::Column::Id",
        on_delete = "Cascade"
    )]
    Account,
}

impl Related<super::account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
