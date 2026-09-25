//! An outstanding "confirm your address" link.

use sea_orm::entity::prelude::*;

/// One unspent confirmation link.
///
/// Keyed by the hash of the token in the link and not by an id, for the
/// reason a session token is: a link that reached a mailbox is a credential,
/// and a stored one would be a working login sitting in a table.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "confirmation")]
pub struct Model {
    /// SHA-256 of the token in the link, hex.
    #[sea_orm(primary_key, auto_increment = false)]
    pub token_hash: Vec<u8>,
    /// The account it confirms.
    pub account_id: Uuid,
    /// When the link stops working.
    ///
    /// A link that never expired would be a password that never expired,
    /// sitting in a mailbox. A link goes when it is used, when its account
    /// asks for a new one or is deleted, and otherwise once it has expired,
    /// in the gateway's sweep ([`crate::confirmations::sweep`]), which is
    /// what keeps the table from growing without bound.
    pub expires_at: TimeDateTimeWithTimeZone,
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
