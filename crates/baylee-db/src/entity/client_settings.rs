//! One account's client preferences.

use sea_orm::entity::prelude::*;

/// The keymap, the phase rail and what the client may answer without asking.
///
/// Deliberately one opaque `jsonb` document and not a table of typed columns.
/// The gateway would have to link `baylee-client-core` to know what is in
/// here, and it links neither the client's brain nor the engine on purpose;
/// and a client that adds a preference should not need a gateway deploy and a
/// migration before it can store it. What the gateway enforces is that the
/// value is an object and that it is small.
///
/// `jsonb` rather than `json`: the document is read far more often than it is
/// written, and `jsonb` parses once on write instead of on every read.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "client_settings")]
pub struct Model {
    /// Whose preferences these are. One row per account, so the account *is*
    /// the key.
    #[sea_orm(primary_key, auto_increment = false)]
    pub account_id: Uuid,
    /// The document.
    #[sea_orm(column_type = "JsonBinary")]
    pub doc: Json,
    /// Last written.
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
