//! A saved deck.

use sea_orm::entity::prelude::*;

/// One deck, as the player built it.
///
/// Card *names*, not registry indices: a deck outlives the pool it was built
/// against, and a name still resolves when an index has been re-issued. The
/// registry lookup happens when the deck is taken to a table.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "deck")]
pub struct Model {
    /// `UUIDv7`.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Whose deck it is.
    pub account_id: Uuid,
    /// What the player called it.
    pub name: String,
    /// Main deck, as `"N Card Name"` lines.
    ///
    /// A Postgres `text[]` rather than a `jsonb` document or a second table.
    /// A decklist is read and written whole, is never queried into, and is
    /// ordered — which is exactly the shape an array is for, and exactly the
    /// shape a `deck_card` table with a position column would be an
    /// elaborate way of getting back.
    pub cards: Vec<String>,
    /// Sideboard, in the same form. Cards outside the game a seat may reach;
    /// never shuffled into the library.
    pub sideboard: Vec<String>,
    /// The commander, if the deck has one.
    pub commander: Option<String>,
    /// Image id of the sleeve these cards show face-down. `None` means the
    /// client draws its own generated back.
    pub sleeve: Option<String>,
    /// Image id of the playmat this deck's seat plays on.
    pub playmat: Option<String>,
    /// Last saved.
    pub updated_at: TimeDateTimeWithTimeZone,
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
