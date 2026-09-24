//! A player signs in with a username, and an address becomes optional (#269).
//!
//! - `username`: the name as its owner typed it, after the rule's folding
//!   (`baylee_protocol::names`); private, shown to its owner and asked for at
//!   sign-in. The name other players see stays the display name and its tag.
//! - `username_key`: what the name is unique under and looked up by, its
//!   lower case. A unique index, so two sign-ins with the same name cannot
//!   both be written; `NULL`s are distinct, so an account without a name (a
//!   guest, once there are guests) takes no name from anybody.
//! - `email` may be empty from here on: registration asks for a username
//!   instead. The index on `lower(email)` stays as it is, since its `NULL`s
//!   are distinct too.
//!
//! Every account that exists is named from its address
//! ([`crate::usernames::name_the_unnamed`]) before the index is made, so the
//! index is made over names that are already unique.

use sea_orm_migration::prelude::*;

/// The sixth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Account::Table)
                    .add_column_if_not_exists(ColumnDef::new(Account::Username).text().null())
                    .add_column_if_not_exists(ColumnDef::new(Account::UsernameKey).text().null())
                    .modify_column(ColumnDef::new(Account::Email).text().null())
                    .to_owned(),
            )
            .await?;
        crate::usernames::name_the_unnamed(manager.get_connection()).await?;
        manager
            .create_index(
                Index::create()
                    .name("account_username_key")
                    .table(Account::Table)
                    .unique()
                    .col(Account::UsernameKey)
                    .if_not_exists()
                    .to_owned(),
            )
            .await
    }

    /// The columns go, and with them every name. The address becomes
    /// required again, which fails on a database holding an account without
    /// one: there is no address to give it back.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DO $drop$ BEGIN \
                   EXECUTE format('DROP INDEX IF EXISTS %I.account_username_key', current_schema()); \
                 END $drop$",
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Account::Table)
                    .drop_column(Account::Username)
                    .drop_column(Account::UsernameKey)
                    .modify_column(ColumnDef::new(Account::Email).text().not_null())
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Account {
    Table,
    Email,
    Username,
    UsernameKey,
}
