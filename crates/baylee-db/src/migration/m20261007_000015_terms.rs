//! Which terms of use an account accepted, and when (WG-1).
//!
//! - `terms_version`: the version of the gateway's terms the account last
//!   accepted (`POST /account/terms`), as `GET /terms` names it. `NULL` for
//!   an account that never accepted any, which is every account from before
//!   this migration and every account on a gateway without terms.
//! - `terms_accepted_at`: when it did.
//!
//! Two columns on the account rather than a table of acceptances: only the
//! last one decides anything (`terms_stale` at sign-in), and they go with the
//! account when it is deleted, as everything else about it does.

use sea_orm_migration::prelude::*;

/// The fifteenth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Account::Table)
                    .add_column_if_not_exists(ColumnDef::new(Account::TermsVersion).text().null())
                    .add_column_if_not_exists(
                        ColumnDef::new(Account::TermsAcceptedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    /// Drops both columns, and with them every acceptance: the next sign-in
    /// on a gateway with terms asks again.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Account::Table)
                    .drop_column(Account::TermsAcceptedAt)
                    .drop_column(Account::TermsVersion)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Account {
    Table,
    TermsVersion,
    TermsAcceptedAt,
}
