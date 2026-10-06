//! An index on `session_token (account_id)`.
//!
//! Every other table that follows an account away (`ON DELETE CASCADE`) or
//! forgets it (`SET NULL`) is indexed on the account; the sessions were not.
//! Deleting an account then scanned every session in the gateway once, and
//! the guest purge does that for each guest it takes: measured on 50 000
//! guests with two sessions each, purging the 2 000 whose sessions had
//! lapsed held the rows for 9.1 s, and 76 ms with this index (06.10.2026).

use sea_orm_migration::prelude::*;

/// The fourteenth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS session_token_by_account \
                 ON session_token (account_id)",
            )
            .await?;
        Ok(())
    }

    /// Drops the index; the sessions stay.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Through `current_schema()`: a bare name that is missing here would
        // be looked for further down the search path.
        manager
            .get_connection()
            .execute_unprepared(
                "DO $drop$ BEGIN \
                   EXECUTE format('DROP INDEX IF EXISTS %I.session_token_by_account', \
                                  current_schema()); \
                 END $drop$",
            )
            .await?;
        Ok(())
    }
}
