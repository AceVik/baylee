//! Closed-beta keys (#317): what lets a stranger make an account, or a new
//! guest, on a gateway with `BAYLEE_REGISTRATION=invite`.
//!
//! `invite` is one row per key the operator made. The key itself is never
//! stored: `key_hash` is the SHA-256 of its canonical sixteen characters, so
//! a row read out of a backup admits nobody. `uses_left` counts down with
//! each account it admits and never below zero; `expires_at` and
//! `revoked_at` close it early. `note` says who it was for, in the
//! operator's words, and is bounded here as well as by the command that
//! writes it.
//!
//! `account.invite_id` says which key admitted an account. It goes
//! `ON DELETE SET NULL`: removing a key forgets which accounts it let in and
//! keeps the accounts. Deleting an account leaves the key's row as it was.

use sea_orm_migration::prelude::*;

/// The tenth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS invite ( \
                 id uuid PRIMARY KEY DEFAULT uuidv7(), \
                 key_hash bytea NOT NULL UNIQUE, \
                 created_at timestamptz NOT NULL DEFAULT now(), \
                 note text CONSTRAINT invite_note_length CHECK (char_length(note) <= 100), \
                 uses_left integer NOT NULL DEFAULT 1 \
                     CONSTRAINT invite_uses_left_not_negative CHECK (uses_left >= 0), \
                 expires_at timestamptz, \
                 revoked_at timestamptz \
             )",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE account ADD COLUMN IF NOT EXISTS invite_id uuid \
             CONSTRAINT account_invite REFERENCES invite (id) ON DELETE SET NULL",
        )
        .await?;
        // A key's deletion looks for the accounts it admitted, and `invite
        // list` counts them.
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS account_by_invite ON account (invite_id) \
             WHERE invite_id IS NOT NULL",
        )
        .await?;
        Ok(())
    }

    /// Forgets every key and which accounts they admitted. The accounts stay.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "DO $drop$ BEGIN \
               EXECUTE format('DROP INDEX IF EXISTS %I.account_by_invite', current_schema()); \
             END $drop$",
        )
        .await?;
        db.execute_unprepared("ALTER TABLE account DROP COLUMN IF EXISTS invite_id")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS invite").await?;
        Ok(())
    }
}
