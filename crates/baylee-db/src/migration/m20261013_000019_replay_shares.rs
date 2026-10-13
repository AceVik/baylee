//! Replays, the owner's changes of 13.10.2026 (`docs/protocol.md`
//! §"Replays"): each seat's consent to show its hidden cards in a game's
//! replays, the moment from which that consent defaults to yes, and the
//! games an account archived.
//!
//! Number 19 follows replays' 18; a lane that lands another 19 first
//! renumbers this one.
//!
//! - `game_record_share`: one row per seat that said, `shared` true or
//!   false. An AI seat is always shared. Goes with the record (which is
//!   never deleted); stays when the seat's account is deleted, as the seat
//!   itself does, anonymous.
//! - `replay_share_cutoff`: one row, the moment this migration ran (the
//!   beta.7 deploy). A seat that never said is shared in a game recorded
//!   from then on (the box is ticked and a player unticks it), and not
//!   shared in one recorded before: the terms those games were played under
//!   promised nobody at the table the recording (owner, 13.10.2026).
//!   Written once; a restart changes nothing.
//! - `replay_archived`: a game an account archived (moved out of its main
//!   list, still watchable). The record stays; the row goes with the
//!   account.

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
    "CREATE TABLE IF NOT EXISTS replay_share_cutoff ( \
         only_row boolean PRIMARY KEY DEFAULT true CONSTRAINT replay_share_cutoff_one CHECK (only_row), \
         at timestamptz NOT NULL DEFAULT now() \
     )",
    // Once: a second run keeps the first moment.
    "INSERT INTO replay_share_cutoff (only_row) VALUES (true) ON CONFLICT DO NOTHING",
    "CREATE TABLE IF NOT EXISTS replay_archived ( \
         account_id uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         game_id text NOT NULL REFERENCES game_record (game_id) ON DELETE CASCADE, \
         at timestamptz NOT NULL DEFAULT now(), \
         PRIMARY KEY (account_id, game_id) \
     )",
    // Deleting a record (which nothing does today) asks this table by game.
    "CREATE INDEX IF NOT EXISTS replay_archived_game ON replay_archived (game_id)",
];

/// What `down` runs, in order.
const DOWN: &[&str] = &[
    "DROP TABLE IF EXISTS replay_archived",
    "DROP TABLE IF EXISTS replay_share_cutoff",
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
