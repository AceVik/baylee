//! The closed beta is over (owner, 10.11.2026): its keys (#317, migration
//! 10) go, and with them what was kept about them.
//!
//! The table `invite` held the SHA-256 of each key, a note naming who it was
//! for, its uses, expiry and revocation; `account.invite_id` said which key
//! admitted an account (`docs/privacy.md`). Both are the keys' data and
//! nothing else reads them, so both are dropped: the index first, then the
//! column (its foreign key goes with it), then the table. The accounts stay,
//! every other column of theirs untouched.
//!
//! A gateway built before this migration selects `account.invite_id` with
//! every account it reads, so it does not run on a database migrated past
//! here; [`Migration::down`] gives it the column back, empty.
#![forbid(unsafe_code)]
use sea_orm_migration::prelude::*;

/// The thirtieth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Every name qualified with the schema the migrator runs in, as an
        // index drop here always is (`docs/dev/database.md`): unqualified, a
        // drop that finds nothing in this schema follows the search path on
        // into `public` and drops another database's table there.
        manager
            .get_connection()
            .execute_unprepared(
                "DO $drop$ BEGIN \
                   EXECUTE format('DROP INDEX IF EXISTS %I.account_by_invite', current_schema()); \
                   EXECUTE format('ALTER TABLE %I.account DROP COLUMN IF EXISTS invite_id', \
                                  current_schema()); \
                   EXECUTE format('DROP TABLE IF EXISTS %I.invite', current_schema()); \
                 END $drop$",
            )
            .await?;
        Ok(())
    }

    /// The table and the column come back empty, as migration 10 made them:
    /// no key and no admission is made again.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DO $make$ BEGIN \
                   EXECUTE format('CREATE TABLE IF NOT EXISTS %I.invite ( \
                       id uuid PRIMARY KEY DEFAULT uuidv7(), \
                       key_hash bytea NOT NULL UNIQUE, \
                       created_at timestamptz NOT NULL DEFAULT now(), \
                       note text CONSTRAINT invite_note_length \
                           CHECK (char_length(note) <= 100), \
                       uses_left integer NOT NULL DEFAULT 1 \
                           CONSTRAINT invite_uses_left_not_negative CHECK (uses_left >= 0), \
                       expires_at timestamptz, \
                       revoked_at timestamptz)', current_schema()); \
                   EXECUTE format('ALTER TABLE %1$I.account ADD COLUMN IF NOT EXISTS invite_id \
                       uuid CONSTRAINT account_invite REFERENCES %1$I.invite (id) \
                       ON DELETE SET NULL', current_schema()); \
                   EXECUTE format('CREATE INDEX IF NOT EXISTS account_by_invite \
                       ON %I.account (invite_id) WHERE invite_id IS NOT NULL', \
                       current_schema()); \
                 END $make$",
            )
            .await?;
        Ok(())
    }
}
