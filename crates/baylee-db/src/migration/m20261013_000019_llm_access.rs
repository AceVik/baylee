//! Who may seat which hosted model (owner, 13.10.2026; `docs/protocol.md`
//! §"Who may seat which model").
//!
//! Number 19 follows replays' 18; a lane that lands another 19 first
//! renumbers this one. Nothing in it depends on another lane's.
//!
//! - `llm_profile_access`: per hosted profile id (the id players order by,
//!   over every seat agent) its policy: `registered` (every registered
//!   account), `listed` (only the accounts in `llm_profile_allow`) or
//!   `nobody`. A profile without a row is `listed` with nobody on its list.
//!   `default_for_registered` marks the one profile the operator offers to
//!   every registered account by default; at most one row holds it.
//! - `llm_profile_allow`: the accounts a `listed` profile admits. A row goes
//!   with its account (cascade), so deleting an account needs no new step.
//!
//! Profile ids are the seat agents' (1-64 of `A-Za-z0-9._-`); a row may name
//! a profile no seat agent reports now, so an operator can set it up first.

use sea_orm_migration::prelude::*;

/// The nineteenth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

/// The statements `up` runs, in order. Each is idempotent.
const UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS llm_profile_access ( \
         profile_id text PRIMARY KEY \
             CONSTRAINT llm_profile_access_id CHECK (profile_id ~ '^[A-Za-z0-9._-]{1,64}$'), \
         policy text NOT NULL \
             CONSTRAINT llm_profile_access_policy \
             CHECK (policy IN ('registered', 'listed', 'nobody')), \
         default_for_registered boolean NOT NULL DEFAULT false, \
         updated_at timestamptz NOT NULL DEFAULT now(), \
         updated_by text NOT NULL \
             CONSTRAINT llm_profile_access_by CHECK (char_length(updated_by) <= 64) \
     )",
    "CREATE UNIQUE INDEX IF NOT EXISTS llm_profile_access_one_default \
         ON llm_profile_access (default_for_registered) WHERE default_for_registered",
    "CREATE TABLE IF NOT EXISTS llm_profile_allow ( \
         profile_id text NOT NULL \
             REFERENCES llm_profile_access (profile_id) ON DELETE CASCADE, \
         account_id uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         added_at timestamptz NOT NULL DEFAULT now(), \
         added_by text NOT NULL \
             CONSTRAINT llm_profile_allow_by CHECK (char_length(added_by) <= 64), \
         PRIMARY KEY (profile_id, account_id) \
     )",
    // Which profiles an account is on, at every listing of the offer.
    "CREATE INDEX IF NOT EXISTS llm_profile_allow_account \
         ON llm_profile_allow (account_id)",
];

/// What `down` runs, in order: every policy and list goes.
const DOWN: &[&str] = &[
    "DO $drop$ BEGIN \
       EXECUTE format('DROP INDEX IF EXISTS %I.llm_profile_allow_account', current_schema()); \
       EXECUTE format('DROP INDEX IF EXISTS %I.llm_profile_access_one_default', current_schema()); \
     END $drop$",
    "DROP TABLE IF EXISTS llm_profile_allow",
    "DROP TABLE IF EXISTS llm_profile_access",
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
