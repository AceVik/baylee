//! Running games the gateway keeps across its own restart
//! (`docs/proposals/games-survive-restarts.md` §"Step 3b").
//!
//! One row per running game: what its record cannot say (the room, and who
//! may open each chair), as the gateway's versioned JSON (`kept` is that
//! version). The preset and the engine's execution identity are the record's
//! header and are not stored twice. A row goes when its record completes
//! (`records::append`), when the game ends any other way, and with its
//! record.
//!
//! It holds account ids, seat token hashes and the session keys chairs are
//! bound to, of running games only: `docs/privacy.md` names it.

use sea_orm_migration::prelude::*;

/// The thirty-sixth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE running_game (
                    game_id text PRIMARY KEY
                        REFERENCES game_record(game_id) ON DELETE CASCADE,
                    kept integer NOT NULL CHECK (kept > 0),
                    body jsonb NOT NULL,
                    written_at timestamptz NOT NULL DEFAULT now()
                )",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // What it held is lost: the games it kept end at the next restart,
        // as every running game did before it.
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE running_game")
            .await?;
        Ok(())
    }
}
