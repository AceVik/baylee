//! Ban lists (owner, 4.11.2026; `docs/banlists-and-formats.md`).
//!
//! `banlists` is one row per list: an official one (`builtin`, no owner),
//! which the gateway keeps in step with the card feed and nobody edits, or a
//! player's own, gone with its owner by cascade. At most one official list a
//! format. `version` counts up with every change to the list's cards or its
//! format, which is what a game freezes as it starts.
//!
//! `banlist_entries` is one row per card on a list, keyed on the oracle id
//! and not on the registry index: a list outlives a build and may name cards
//! this pool does not hold yet.

use sea_orm_migration::prelude::*;

/// The twenty-eighth migration (ban lists).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS banlists ( \
                 id uuid PRIMARY KEY DEFAULT uuidv7(), \
                 name text NOT NULL \
                     CONSTRAINT banlists_name_length CHECK (char_length(name) BETWEEN 1 AND 60), \
                 format text NOT NULL \
                     CONSTRAINT banlists_format_length \
                     CHECK (char_length(format) BETWEEN 1 AND 32), \
                 builtin boolean NOT NULL DEFAULT false, \
                 owner uuid REFERENCES account (id) ON DELETE CASCADE, \
                 version bigint NOT NULL DEFAULT 1, \
                 updated_at timestamptz NOT NULL DEFAULT now(), \
                 CONSTRAINT banlists_owner_or_builtin CHECK (builtin = (owner IS NULL)) \
             )",
        )
        .await?;
        // One official list a format, which is what a sync upserts on.
        db.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS banlists_official \
             ON banlists (format) WHERE builtin",
        )
        .await?;
        // A player's lists, and an account's deletion.
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS banlists_owner \
             ON banlists (owner) WHERE owner IS NOT NULL",
        )
        .await?;
        db.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS banlist_entries ( \
                 banlist_id uuid NOT NULL REFERENCES banlists (id) ON DELETE CASCADE, \
                 oracle_id uuid NOT NULL, \
                 status text NOT NULL \
                     CONSTRAINT banlist_entries_status \
                     CHECK (status IN ('banned', 'restricted', 'banned_as_commander')), \
                 PRIMARY KEY (banlist_id, oracle_id) \
             )",
        )
        .await?;
        Ok(())
    }

    /// Forgets every list.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP TABLE IF EXISTS banlist_entries")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS banlists")
            .await?;
        Ok(())
    }
}
