//! What an admin may set on an account from the console (beta.8,
//! `docs/protocol.md` §"Acting on an account"):
//!
//! - `must_change_password`: an admin reset the password, and the account
//!   signs in with the one it was given only to choose its own
//!   (`POST /auth/password`). `false` for every account until then.
//! - `banned_at`, `banned_until`, `ban_reason`, `banned_by`: the account is
//!   banned since `banned_at`, until `banned_until` (`NULL` for good), for
//!   `ban_reason` (the words its player is shown), by the admin named
//!   `banned_by`. All `NULL` for an account that is not banned; a ban whose
//!   end has passed is no ban, and is cleared by the next one or an unban.
//!
//! Columns on the account rather than a table of bans: only the ban in force
//! decides anything, the session lookup reads it with the account in the
//! statement it already makes, and it goes with the account when that is
//! deleted, as everything else about it does. The admin audit (the log and
//! the feedback service's) is the history.

use sea_orm_migration::prelude::*;

/// The sixteenth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Account::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Account::MustChangePassword)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .add_column_if_not_exists(
                        ColumnDef::new(Account::BannedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .add_column_if_not_exists(
                        ColumnDef::new(Account::BannedUntil)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .add_column_if_not_exists(ColumnDef::new(Account::BanReason).text().null())
                    .add_column_if_not_exists(ColumnDef::new(Account::BannedBy).text().null())
                    .to_owned(),
            )
            .await?;
        // A ban is all of its parts or none: the player is always told why.
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE account ADD CONSTRAINT account_ban_whole CHECK (\
                 (banned_at IS NULL AND ban_reason IS NULL AND banned_until IS NULL \
                  AND banned_by IS NULL) \
                 OR (banned_at IS NOT NULL AND ban_reason IS NOT NULL AND banned_by IS NOT NULL))",
            )
            .await?;
        Ok(())
    }

    /// Drops the columns, and with them every ban and every reset still
    /// waiting to be finished: those accounts sign in as before.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE account DROP CONSTRAINT IF EXISTS account_ban_whole")
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Account::Table)
                    .drop_column(Account::BannedBy)
                    .drop_column(Account::BanReason)
                    .drop_column(Account::BannedUntil)
                    .drop_column(Account::BannedAt)
                    .drop_column(Account::MustChangePassword)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Account {
    Table,
    MustChangePassword,
    BannedAt,
    BannedUntil,
    BanReason,
    BannedBy,
}
