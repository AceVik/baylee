//! Who owns an uploaded picture (#292).

use sea_orm::entity::prelude::*;

/// One owner of one stored sleeve or playmat.
///
/// A picture is stored once however many players upload it (its id is the
/// hash of its normalised bytes), so it has a row per owner. The file goes
/// when its last row does.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "upload")]
pub struct Model {
    /// The stored file's id: the SHA-256 of its bytes, hex.
    #[sea_orm(primary_key, auto_increment = false)]
    pub image_id: String,
    /// Who owns it.
    #[sea_orm(primary_key, auto_increment = false)]
    pub account_id: Uuid,
    /// `sleeve` or `playmat`.
    pub kind: String,
    /// When this owner first held it.
    pub created_at: TimeDateTimeWithTimeZone,
}

/// The owning account.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Cascades: an account's deletion takes its claim on every picture.
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
