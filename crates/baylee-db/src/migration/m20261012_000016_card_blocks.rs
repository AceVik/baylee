//! Blocked cards: a card the operator has taken out of play on this
//! gateway, for good or until a moment, with a reason (owner, 12.10.2026;
//! `docs/protocol.md` §"Blocked cards").
//!
//! `card_block` is one row per block the admin console made. `card_index`
//! is the card's `CardIndex` (the append-only ledger's number, which never
//! changes), `card_name` its English name when it was blocked, kept so a
//! row still reads after a build that no longer has the card. `until` is
//! `NULL` for a block without an end; a block whose `until` has passed has
//! ended by itself and nothing has to sweep it. `lifted_at`/`lifted_by`
//! end one early. Rows are never deleted by the gateway: the list is also
//! the history of what was blocked, by whom and why.
//!
//! At most one block of a card is open (not lifted) at a time; an open
//! block whose `until` has passed does not count, which the partial index
//! cannot say, so [`crate::card_blocks::create`] checks it in the
//! statement that inserts.

use sea_orm_migration::prelude::*;

/// The sixteenth migration (card blocks).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS card_block ( \
                 id uuid PRIMARY KEY DEFAULT uuidv7(), \
                 card_index integer NOT NULL \
                     CONSTRAINT card_block_index_not_negative CHECK (card_index >= 0), \
                 card_name text NOT NULL \
                     CONSTRAINT card_block_name_length CHECK (char_length(card_name) <= 200), \
                 reason text NOT NULL \
                     CONSTRAINT card_block_reason_length \
                     CHECK (char_length(reason) BETWEEN 1 AND 200), \
                 created_at timestamptz NOT NULL DEFAULT now(), \
                 created_by text NOT NULL \
                     CONSTRAINT card_block_created_by_length CHECK (char_length(created_by) <= 64), \
                 until timestamptz, \
                 lifted_at timestamptz, \
                 lifted_by text \
                     CONSTRAINT card_block_lifted_by_length CHECK (char_length(lifted_by) <= 64) \
             )",
        )
        .await?;
        // The gateway reads the open blocks at start and after every change.
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS card_block_open ON card_block (card_index) \
             WHERE lifted_at IS NULL",
        )
        .await?;
        Ok(())
    }

    /// Forgets every block, and with them their history.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "DO $drop$ BEGIN \
               EXECUTE format('DROP INDEX IF EXISTS %I.card_block_open', current_schema()); \
             END $drop$",
        )
        .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS card_block")
            .await?;
        Ok(())
    }
}
