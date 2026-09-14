//! The six account-side tables, and the indexes the gateway's own queries ask
//! for.
//!
//! # Indexes are cut to the questions, not to the columns
//!
//! Every index here exists because a route runs a query that would otherwise
//! be a sequential scan:
//!
//! - Signing in looks an account up by e-mail, case-insensitively, and
//!   registering refuses a display name that is taken, case-insensitively.
//!   Both are `lower(...)` comparisons, so both indexes are on the
//!   expression — an index on the plain column is not usable by either and
//!   would be a scan that looked like it was covered.
//! - A session token and a confirmation link are found by their hash, which
//!   is the primary key, so neither needs an index of its own. What both need
//!   is one on `expires_at`, because the sweep that removes lapsed rows is
//!   the only query that does not name a key.
//! - A deck list is "this account's decks, newest first", which is one index
//!   on `(account_id, updated_at DESC)` and no sort.
//!
//! The foreign keys are all `ON DELETE CASCADE`, which is the whole of
//! account deletion: one `DELETE` and the decks, sessions, links, answers and
//! settings go with it.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{
    array, big_integer, boolean, json_binary, pk_uuid, text, text_null, timestamp_with_time_zone,
    timestamp_with_time_zone_null, uuid,
};

/// The first migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        accounts(manager).await?;
        credentials(manager).await?;
        decks(manager).await?;
        preferences(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Children before the parent: the cascades would do it, but a
        // rollback that relied on them would drop `account` and take five
        // tables with it whether or not this migration made them.
        for table in [
            ClientSettings::Table.into_iden(),
            StandingAnswer::Table.into_iden(),
            Deck::Table.into_iden(),
            Confirmation::Table.into_iden(),
            SessionToken::Table.into_iden(),
            Account::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).if_exists().to_owned())
                .await?;
        }
        Ok(())
    }
}

/// The account table and the two uniquenesses that mean what a person means
/// by them.
async fn accounts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Account::Table)
                .if_not_exists()
                .col(pk_uuid(Account::Id))
                .col(text(Account::Email))
                .col(text(Account::DisplayName))
                .col(text(Account::PasswordHash))
                .col(timestamp_with_time_zone(Account::CreatedAt))
                .col(timestamp_with_time_zone_null(Account::ConfirmedAt))
                .col(text(Account::Lang).default(""))
                .to_owned(),
        )
        .await?;

    // Uniqueness that means what a person means by it. Two accounts whose
    // addresses differ only in case are one address, and two display
    // names that differ only in case are one name to everyone reading the
    // lobby.
    for (name, table, column) in [
        ("account_email_lower", Account::Table, Account::Email),
        (
            "account_display_name_lower",
            Account::Table,
            Account::DisplayName,
        ),
    ] {
        manager
            .create_index(
                Index::create()
                    .name(name)
                    .table(table)
                    .unique()
                    .col(Func::lower(Expr::col(column)))
                    .to_owned(),
            )
            .await?;
    }
    Ok(())
}

/// Sessions and confirmation links: the two tables keyed by the hash of a
/// secret, and the expiry indexes their sweeps stand on.
async fn credentials(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(SessionToken::Table)
                .if_not_exists()
                .col(text(SessionToken::TokenHash).primary_key())
                .col(uuid(SessionToken::AccountId))
                .col(timestamp_with_time_zone(SessionToken::ExpiresAt))
                .foreign_key(&mut cascade_to_account(
                    "session_token_account",
                    SessionToken::Table,
                    SessionToken::AccountId,
                ))
                .to_owned(),
        )
        .await?;

    manager
        .create_table(
            Table::create()
                .table(Confirmation::Table)
                .if_not_exists()
                .col(text(Confirmation::TokenHash).primary_key())
                .col(uuid(Confirmation::AccountId))
                .col(timestamp_with_time_zone(Confirmation::ExpiresAt))
                .foreign_key(&mut cascade_to_account(
                    "confirmation_account",
                    Confirmation::Table,
                    Confirmation::AccountId,
                ))
                .to_owned(),
        )
        .await?;

    // The sweeps. Both tables are read by key on every other path, so
    // expiry is the one query with nothing to stand on.
    manager
        .create_index(
            Index::create()
                .name("session_token_expiry")
                .table(SessionToken::Table)
                .col(SessionToken::ExpiresAt)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("confirmation_expiry")
                .table(Confirmation::Table)
                .col(Confirmation::ExpiresAt)
                .to_owned(),
        )
        .await?;
    // Resending a link invalidates the last one, which is a delete by
    // account rather than by hash.
    manager
        .create_index(
            Index::create()
                .name("confirmation_account_id")
                .table(Confirmation::Table)
                .col(Confirmation::AccountId)
                .to_owned(),
        )
        .await?;
    Ok(())
}

/// Decks, and the one index that answers "mine, newest first" in order.
async fn decks(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Deck::Table)
                .if_not_exists()
                .col(pk_uuid(Deck::Id))
                .col(uuid(Deck::AccountId))
                .col(text(Deck::Name))
                .col(array(Deck::Cards, ColumnType::Text))
                .col(array(Deck::Sideboard, ColumnType::Text).default(Expr::cust("'{}'")))
                .col(text_null(Deck::Commander))
                .col(text_null(Deck::Sleeve))
                .col(text_null(Deck::Playmat))
                .col(timestamp_with_time_zone(Deck::UpdatedAt))
                .foreign_key(&mut cascade_to_account(
                    "deck_account",
                    Deck::Table,
                    Deck::AccountId,
                ))
                .to_owned(),
        )
        .await?;

    // "My decks, newest first" in index order, with no sort on top.
    manager
        .create_index(
            Index::create()
                .name("deck_account_recent")
                .table(Deck::Table)
                .col(Deck::AccountId)
                .col((Deck::UpdatedAt, IndexOrder::Desc))
                .to_owned(),
        )
        .await?;

    Ok(())
}

/// What an account remembers: standing answers, and the client's own
/// preferences document.
async fn preferences(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(StandingAnswer::Table)
                .if_not_exists()
                .col(uuid(StandingAnswer::AccountId))
                .col(big_integer(StandingAnswer::Card))
                .col(big_integer(StandingAnswer::Ability))
                .col(boolean(StandingAnswer::Yes))
                .primary_key(
                    Index::create()
                        .col(StandingAnswer::AccountId)
                        .col(StandingAnswer::Card)
                        .col(StandingAnswer::Ability),
                )
                .foreign_key(&mut cascade_to_account(
                    "standing_answer_account",
                    StandingAnswer::Table,
                    StandingAnswer::AccountId,
                ))
                .to_owned(),
        )
        .await?;

    manager
        .create_table(
            Table::create()
                .table(ClientSettings::Table)
                .if_not_exists()
                .col(uuid(ClientSettings::AccountId).primary_key())
                .col(json_binary(ClientSettings::Doc))
                .col(timestamp_with_time_zone(ClientSettings::UpdatedAt))
                .foreign_key(&mut cascade_to_account(
                    "client_settings_account",
                    ClientSettings::Table,
                    ClientSettings::AccountId,
                ))
                .to_owned(),
        )
        .await?;
    Ok(())
}

/// One child-to-account reference, cascading.
fn cascade_to_account<T, C>(name: &str, table: T, column: C) -> ForeignKeyCreateStatement
where
    T: IntoIden + 'static,
    C: IntoIden + 'static,
{
    ForeignKey::create()
        .name(name)
        .from(table, column)
        .to(Account::Table, Account::Id)
        .on_delete(ForeignKeyAction::Cascade)
        .to_owned()
}

#[derive(DeriveIden)]
enum Account {
    Table,
    Id,
    Email,
    DisplayName,
    PasswordHash,
    CreatedAt,
    ConfirmedAt,
    Lang,
}

#[derive(DeriveIden)]
enum SessionToken {
    Table,
    TokenHash,
    AccountId,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum Confirmation {
    Table,
    TokenHash,
    AccountId,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum Deck {
    Table,
    Id,
    AccountId,
    Name,
    Cards,
    Sideboard,
    Commander,
    Sleeve,
    Playmat,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum StandingAnswer {
    Table,
    AccountId,
    Card,
    Ability,
    Yes,
}

#[derive(DeriveIden)]
enum ClientSettings {
    Table,
    AccountId,
    Doc,
    UpdatedAt,
}
