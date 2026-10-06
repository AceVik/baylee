//! A seat a host handed to a seat bridge (a language model at its table,
//! `docs/protocol.md` §"A host's chair for a seat bridge").
//!
//! Such a chair is played by no account: the bridge redeems a chair ticket
//! its host asked for and sits on the host's word. `game_record_seat` keeps
//! which account vouched for it in `delegated_by`, beside an `account_id`
//! that stays `NULL` (no account played the seat), so the record says whose
//! seat bridge played it and never that the host did. The link goes with the
//! account (`ON DELETE SET NULL`), as `account_id`'s does.

use sea_orm_migration::prelude::*;

/// The thirteenth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "ALTER TABLE game_record_seat ADD COLUMN IF NOT EXISTS delegated_by uuid \
             CONSTRAINT game_record_seat_delegate REFERENCES account (id) ON DELETE SET NULL \
             CONSTRAINT game_record_seat_one_occupant \
                 CHECK (delegated_by IS NULL OR account_id IS NULL)",
        )
        .await?;
        // An account's deletion sets its rows to NULL, and asks for them by
        // account to do it.
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS game_record_seat_by_delegate \
             ON game_record_seat (delegated_by) WHERE delegated_by IS NOT NULL",
        )
        .await?;
        Ok(())
    }

    /// Forgets who vouched for a bridge's seat; the records stay.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Through `current_schema()`: a bare name that is missing here would
        // be looked for further down the search path.
        db.execute_unprepared(
            "DO $drop$ BEGIN \
               EXECUTE format('DROP INDEX IF EXISTS %I.game_record_seat_by_delegate', \
                              current_schema()); \
             END $drop$",
        )
        .await?;
        db.execute_unprepared("ALTER TABLE game_record_seat DROP COLUMN IF EXISTS delegated_by")
            .await?;
        Ok(())
    }
}
