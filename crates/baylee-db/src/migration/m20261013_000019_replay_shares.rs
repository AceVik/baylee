//! Replays, the owner's changes of 13.10.2026 (`docs/protocol.md`
//! §"Replays"): each seat's consent to show its hidden cards in a game's
//! replays, and the games an account hid from its own list.
//!
//! Number 19 follows replays' 18; a lane that lands another 19 first
//! renumbers this one.
//!
//! - `game_record_share`: one row per seat that said, `shared` true or
//!   false; no row is "shared" (the default: the box is ticked and a player
//!   unticks it, owner 13.10.2026). An AI seat is always shared. Goes with the record (which is never deleted); stays when the
//!   seat's account is deleted, as the seat itself does, anonymous.
//! - `replay_hidden`: a game an account hid from its list ("delete" in the
//!   client). The record stays; the row goes with the account.

use sea_orm_migration::prelude::*;

/// The nineteenth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

/// The statements `up` runs, in order. Each is idempotent.
const UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS game_record_share ( \
         game_id text NOT NULL REFERENCES game_record (game_id) ON DELETE CASCADE, \
         seat smallint NOT NULL CONSTRAINT game_record_share_seat CHECK (seat BETWEEN 0 AND 15), \
         shared boolean NOT NULL DEFAULT true, \
         at timestamptz NOT NULL DEFAULT now(), \
         PRIMARY KEY (game_id, seat) \
     )",
    "CREATE TABLE IF NOT EXISTS replay_hidden ( \
         account_id uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         game_id text NOT NULL REFERENCES game_record (game_id) ON DELETE CASCADE, \
         at timestamptz NOT NULL DEFAULT now(), \
         PRIMARY KEY (account_id, game_id) \
     )",
    // Deleting a record (which nothing does today) asks this table by game.
    "CREATE INDEX IF NOT EXISTS replay_hidden_game ON replay_hidden (game_id)",
];

/// What `down` runs, in order.
const DOWN: &[&str] = &[
    "DROP TABLE IF EXISTS replay_hidden",
    "DROP TABLE IF EXISTS game_record_share",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in UP {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in DOWN {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }
}
