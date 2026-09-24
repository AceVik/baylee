//! Guests: an account a player gets by asking for it, with no name to sign
//! in with and no password (#269).
//!
//! - `guest`: the account was handed out by `POST /auth/guest`. Its session
//!   is the only way into it, so it lives as long as a session does and is
//!   deleted once none is left (`baylee-gateway`'s sweeper), its decks and
//!   preferences with it by the cascades 000001 made.
//! - `password_hash` may be empty from here on, for a guest and nobody else.
//!
//! Two checks say who holds what, in both directions:
//!
//! - `account_can_sign_in`: an account that is not a guest has a password
//!   and something to name it by — its username, or, until the sign-in by
//!   address ends (#280), its address. The address is allowed because the
//!   importer writes an account before it names it; #280 narrows this to the
//!   username.
//! - `account_guest_holds_no_credentials`: a guest has no username, no
//!   address and no password. Nothing can sign in as one, so nothing can
//!   keep one alive but its session.
//!
//! And a partial index over the guests alone: the sweeper walks them on every
//! tick and the gateway counts them before it hands another one out, and
//! neither should read the whole table to do it.

use sea_orm_migration::prelude::*;

/// The seventh migration.
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
                        ColumnDef::new(Account::Guest)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .modify_column(ColumnDef::new(Account::PasswordHash).text().null())
                    .to_owned(),
            )
            .await?;
        let db = manager.get_connection();
        db.execute_unprepared(
            "ALTER TABLE account ADD CONSTRAINT account_can_sign_in \
             CHECK (guest OR (password_hash IS NOT NULL \
                              AND (username_key IS NOT NULL OR email IS NOT NULL)))",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE account ADD CONSTRAINT account_guest_holds_no_credentials \
             CHECK (NOT guest OR (password_hash IS NULL AND username_key IS NULL \
                                  AND username IS NULL AND email IS NULL))",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS account_guest ON account (created_at) WHERE guest",
        )
        .await?;
        Ok(())
    }

    /// The guests go with the column that says they are guests, which fails
    /// on a database holding one: an account without a password cannot be
    /// given one back. Delete the guests first to go below this.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "DO $drop$ BEGIN \
               EXECUTE format('DROP INDEX IF EXISTS %I.account_guest', current_schema()); \
             END $drop$",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE account DROP CONSTRAINT IF EXISTS account_guest_holds_no_credentials",
        )
        .await?;
        db.execute_unprepared("ALTER TABLE account DROP CONSTRAINT IF EXISTS account_can_sign_in")
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Account::Table)
                    .drop_column(Account::Guest)
                    .modify_column(ColumnDef::new(Account::PasswordHash).text().not_null())
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Account {
    Table,
    Guest,
    PasswordHash,
}
