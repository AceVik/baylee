//! Replays (beta.8, `docs/protocol.md` §"Replays"): what the list of a
//! player's games shows, kept beside the record it was played from.
//!
//! Number 18 follows social's 17; a lane that lands another 18 first
//! renumbers this one. Nothing in it depends on another lane's.
//!
//! - `game_record`: the engine's `build` (from its hello), whether its record
//!   carries `seen` (`checked`), the table's `allow_spectators` at start
//!   (`NULL` for older games), and the end: `winners`, `turn`, `end_reason`.
//! - `game_record_seat`: who the chair was (`seated`: `person`, `house`,
//!   `model`), so a seat whose account link was cut still reads as a former
//!   player rather than the house; the deck's colour identity (`colours`,
//!   `WUBRG` letters) and its first commander (`commander`, a `CardIndex`).
//!   Card data, not personal: they stay when the account link is cut.
//!
//! No tape is stored: a replay is projected when asked.

use sea_orm_migration::prelude::*;

/// The eighteenth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

/// The statements `up` runs, in order. Each is idempotent.
const UP: &[&str] = &[
    "ALTER TABLE game_record ADD COLUMN IF NOT EXISTS build text NULL",
    "ALTER TABLE game_record ADD COLUMN IF NOT EXISTS checked boolean NOT NULL DEFAULT false",
    "ALTER TABLE game_record ADD COLUMN IF NOT EXISTS allow_spectators boolean NULL",
    "ALTER TABLE game_record ADD COLUMN IF NOT EXISTS winners smallint[] NULL",
    "ALTER TABLE game_record ADD COLUMN IF NOT EXISTS turn integer NULL",
    "ALTER TABLE game_record ADD COLUMN IF NOT EXISTS end_reason text NULL",
    "ALTER TABLE game_record_seat ADD COLUMN IF NOT EXISTS seated text NULL \
         CONSTRAINT game_record_seat_seated CHECK (seated IN ('person', 'house', 'model'))",
    "ALTER TABLE game_record_seat ADD COLUMN IF NOT EXISTS colours text NULL \
         CONSTRAINT game_record_seat_colours CHECK (colours ~ '^W?U?B?R?G?$')",
    "ALTER TABLE game_record_seat ADD COLUMN IF NOT EXISTS commander integer NULL",
];

/// What `down` runs, in order.
const DOWN: &[&str] = &[
    "ALTER TABLE game_record_seat DROP COLUMN IF EXISTS commander",
    "ALTER TABLE game_record_seat DROP COLUMN IF EXISTS colours",
    "ALTER TABLE game_record_seat DROP COLUMN IF EXISTS seated",
    "ALTER TABLE game_record DROP COLUMN IF EXISTS end_reason",
    "ALTER TABLE game_record DROP COLUMN IF EXISTS turn",
    "ALTER TABLE game_record DROP COLUMN IF EXISTS winners",
    "ALTER TABLE game_record DROP COLUMN IF EXISTS allow_spectators",
    "ALTER TABLE game_record DROP COLUMN IF EXISTS checked",
    "ALTER TABLE game_record DROP COLUMN IF EXISTS build",
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
