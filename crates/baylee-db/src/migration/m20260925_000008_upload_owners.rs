//! Who uploaded which picture (#292).
//!
//! An uploaded sleeve or playmat is stored once, named by the hash of its
//! normalised bytes, so two players uploading the same picture share one
//! file. Who owns it is therefore not a column on the file but a row per
//! owner: `upload(image_id, account_id)`, cascading from the account like
//! every other table here. A file goes when its last owner does.
//!
//! The files uploaded before this migration were never recorded. They are
//! attributed to every account with a deck that names them. That is the
//! uploader: a copied deck starts without the original's pictures, so a
//! deck names somebody else's picture only when its player typed the id in,
//! and that player is then taken for an owner as well. A file no deck names
//! is left without an owner, and `docs/privacy.md` lists that case.
//!
//! An index on `account_id`, because an account's deletion asks for its rows
//! and the key leads with the image.

use sea_orm_migration::prelude::*;

/// The eighth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Upload::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Upload::ImageId).text().not_null())
                    .col(ColumnDef::new(Upload::AccountId).uuid().not_null())
                    .col(ColumnDef::new(Upload::Kind).text().not_null())
                    .col(
                        ColumnDef::new(Upload::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(Index::create().col(Upload::ImageId).col(Upload::AccountId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("upload_account")
                            .from(Upload::Table, Upload::AccountId)
                            .to(Account::Table, Account::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .check(Expr::col(Upload::Kind).is_in(["sleeve", "playmat"]))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("upload_by_account")
                    .table(Upload::Table)
                    .col(Upload::AccountId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        let db = manager.get_connection();
        for (column, kind) in [("sleeve", "sleeve"), ("playmat", "playmat")] {
            db.execute_unprepared(&format!(
                "INSERT INTO upload (image_id, account_id, kind) \
                 SELECT DISTINCT {column}, account_id, '{kind}' FROM deck \
                 WHERE {column} IS NOT NULL AND account_id IS NOT NULL \
                 ON CONFLICT DO NOTHING"
            ))
            .await?;
        }
        Ok(())
    }

    /// Forgets who owns what. The files stay where they are.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Upload::Table).if_exists().to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Upload {
    Table,
    ImageId,
    AccountId,
    Kind,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Account {
    Table,
    Id,
}
