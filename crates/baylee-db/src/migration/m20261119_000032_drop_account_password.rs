//! The accounts' password hashes go (owner, 19.11.2026): nobody has signed
//! in here with a password since the cutover of 30.10.2026, and the hashes
//! of the accounts that have not moved yet are in Baylee Central's legacy
//! store, which signs them in until 30.06.2027 (`docs/central-login.md`
//! §"Cutover"). Nothing reads `account.password_hash` any more, so it is
//! dropped. The accounts stay, every other column untouched: a moved account
//! is still linked to its row here at its first admission (`central::adopt`).
//!
//! Three checks named the column, and PostgreSQL drops a check together
//! with a column it names, without saying so. Each is therefore dropped by
//! name and made again for what it still has to say:
//!
//! - `account_can_sign_in`: a local account, one that is neither a guest nor
//!   Central's (an account from before Central that has not moved yet, or
//!   one written straight into the database, as the trainer's teachers are),
//!   has a username or an address to be known by. It no longer asks for a
//!   password.
//! - `account_guest_holds_no_credentials`: a guest has no username and no
//!   address.
//! - `account_central_is_no_guest`, which replaces
//!   `account_central_holds_no_password`: a Central account is not a guest.
//!
//! **The hashes are destroyed and no step back restores them.** Before this
//! runs on a gateway that still has password accounts, their export must be
//! in Central's legacy store (`docs/history/legacy-cutover-2026-11-19.md`);
//! a hash that is only here is lost with this migration, and its account
//! with it. Take a backup first.
//!
//! A gateway built before this migration does not start on a database
//! migrated past here: its migrator finds a row in `seaql_migrations` it has
//! no file for and refuses (and past that it would read
//! `account.password_hash` with every account). Nothing in the repository
//! runs a migration's `down` outside the tests, so going back to an older
//! build means restoring the backup taken before the deploy.
#![forbid(unsafe_code)]
use sea_orm_migration::prelude::*;

/// The thirty-second migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Every name qualified with the schema the migrator runs in, as a
        // drop here always is (`docs/dev/database.md`): unqualified, a drop
        // that finds nothing in this schema follows the search path on into
        // `public` and changes another database's table there.
        manager
            .get_connection()
            .execute_unprepared(
                "DO $drop$ BEGIN \
                   EXECUTE format('ALTER TABLE %I.account \
                       DROP CONSTRAINT IF EXISTS account_can_sign_in, \
                       DROP CONSTRAINT IF EXISTS account_guest_holds_no_credentials, \
                       DROP CONSTRAINT IF EXISTS account_central_holds_no_password, \
                       DROP COLUMN IF EXISTS password_hash', current_schema()); \
                   EXECUTE format('ALTER TABLE %I.account \
                       ADD CONSTRAINT account_can_sign_in CHECK ( \
                           guest OR central_only \
                           OR username_key IS NOT NULL OR email IS NOT NULL), \
                       ADD CONSTRAINT account_guest_holds_no_credentials CHECK ( \
                           NOT guest OR (username_key IS NULL AND username IS NULL \
                                         AND email IS NULL)), \
                       ADD CONSTRAINT account_central_is_no_guest CHECK ( \
                           NOT (central_only AND guest))', current_schema()); \
                 END $drop$",
            )
            .await?;
        Ok(())
    }

    /// The column comes back empty and the three checks as migrations 7, 21
    /// and 22 left them, so the migrator can step back past here. That is
    /// only possible on a database without a local account (neither a guest
    /// nor Central's): the old schema holds such an account to a password,
    /// and one without a password cannot be given one back. With one left
    /// this refuses and changes nothing.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DO $restore$ DECLARE local_accounts boolean; BEGIN \
                   EXECUTE format('SELECT EXISTS (SELECT 1 FROM %I.account \
                       WHERE NOT guest AND NOT central_only)', current_schema()) \
                       INTO local_accounts; \
                   IF local_accounts THEN \
                     RAISE EXCEPTION 'local accounts have no password to give back'; \
                   END IF; \
                   EXECUTE format('ALTER TABLE %I.account \
                       DROP CONSTRAINT IF EXISTS account_can_sign_in, \
                       DROP CONSTRAINT IF EXISTS account_guest_holds_no_credentials, \
                       DROP CONSTRAINT IF EXISTS account_central_is_no_guest, \
                       ADD COLUMN IF NOT EXISTS password_hash text', current_schema()); \
                   EXECUTE format('ALTER TABLE %I.account \
                       ADD CONSTRAINT account_can_sign_in CHECK ( \
                           guest OR central_only OR (password_hash IS NOT NULL \
                           AND (username_key IS NOT NULL OR email IS NOT NULL))), \
                       ADD CONSTRAINT account_guest_holds_no_credentials CHECK ( \
                           NOT guest OR (password_hash IS NULL AND username_key IS NULL \
                                         AND username IS NULL AND email IS NULL)), \
                       ADD CONSTRAINT account_central_holds_no_password CHECK ( \
                           NOT central_only OR (NOT guest AND password_hash IS NULL))', \
                       current_schema()); \
                 END $restore$",
            )
            .await?;
        Ok(())
    }
}
