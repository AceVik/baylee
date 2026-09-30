//! Room for Wizards' preconstructed decks, which arrive without a migration
//! each.
//!
//! The house decks before these were seeded one migration at a time, which
//! is right for a handful the project wrote down by hand and wrong for a
//! thousand lists a command writes (`docs/precons.md`). Those are synced
//! instead, as the gateway starts ([`crate::precons::sync`]), and what this
//! migration adds is only what a sync needs to find its own rows again and to
//! know it has nothing to do:
//!
//! - `deck.source`: the list a synced deck was written from
//!   (`E02/sun-empire`), unique where it is set. A row with no source is
//!   anybody else's — a player's, or a house deck a migration seeded — and
//!   no sync ever touches it.
//! - `deck.offered`: whether the shared listing shows the deck. A precon
//!   that stops being playable is withdrawn, never deleted: a player's copy
//!   still names the deck and the version it was copied from, and a deck
//!   that becomes playable again comes back as the same deck with its
//!   history.
//! - `deck_sync`: one row per kind of sync, holding the stamp of what it last
//!   wrote, so a restart with the same lists is one read.
//!
//! Both columns are held to kinds by a `CHECK`, as the owner is (migration
//! 2): a player's own deck has no source and is never withdrawn from a
//! listing it is not in.

use sea_orm_migration::prelude::*;

/// The twelfth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "ALTER TABLE deck ADD COLUMN IF NOT EXISTS source text \
             CONSTRAINT deck_source_is_shared CHECK (source IS NULL OR kind <> 'account')",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE deck ADD COLUMN IF NOT EXISTS offered boolean NOT NULL DEFAULT true \
             CONSTRAINT deck_withdrawn_is_shared CHECK (offered OR kind <> 'account')",
        )
        .await?;
        // What a sync looks its rows up by, and what keeps one list from
        // becoming two decks.
        db.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS deck_by_source ON deck (source) \
             WHERE source IS NOT NULL",
        )
        .await?;
        db.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS deck_sync ( \
                 name text PRIMARY KEY, \
                 stamp text NOT NULL, \
                 synced_at timestamptz NOT NULL DEFAULT now() \
             )",
        )
        .await?;
        Ok(())
    }

    /// Forgets which decks were synced and from what. The decks stay, as the
    /// rows they are, and a later sync would add them a second time — which
    /// is what going back past this migration means.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Both through `current_schema()`: a bare name that is missing here
        // would be looked for further down the search path.
        db.execute_unprepared(
            "DO $drop$ BEGIN \
               EXECUTE format('DROP TABLE IF EXISTS %I.deck_sync', current_schema()); \
               EXECUTE format('DROP INDEX IF EXISTS %I.deck_by_source', current_schema()); \
             END $drop$",
        )
        .await?;
        db.execute_unprepared("ALTER TABLE deck DROP COLUMN IF EXISTS offered")
            .await?;
        db.execute_unprepared("ALTER TABLE deck DROP COLUMN IF EXISTS source")
            .await?;
        Ok(())
    }
}
