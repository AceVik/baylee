//! A signed-in session.

use sea_orm::entity::prelude::*;

/// One live session, addressed by the hash of the bearer token.
///
/// The token itself is never stored. A database file, a backup or a `pg_dump`
/// would otherwise be a stack of working logins, and the gateway has no need
/// for the plaintext: it hashes what a request presents and looks that up.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "session_token")]
pub struct Model {
    /// SHA-256 of the bearer token, hex. The key, because it is the only
    /// thing this row is ever found by.
    #[sea_orm(primary_key, auto_increment = false)]
    pub token_hash: Vec<u8>,
    /// Whose session it is.
    pub account_id: Uuid,
    /// When it lapses. Slid forward on use — see
    /// `baylee_gateway::auth` for why not on *every* use.
    pub expires_at: TimeDateTimeWithTimeZone,
}

/// The owning account.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Cascades: closing an account signs it out everywhere.
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
