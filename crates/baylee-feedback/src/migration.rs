//! The service's schema, in order. Append-only, as the gateway's is.
//!
//! Its own migration table (`feedback_migrations`) and a table name no
//! gateway uses, so a service pointed at the gateway's database by mistake
//! neither trips over the gateway's migrator nor it over this one.

use sea_orm_migration::prelude::*;

/// The migrator the service runs on connect.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migration_table_name() -> DynIden {
        "feedback_migrations".into_iden()
    }

    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(Reports),
            Box::new(Admins),
            Box::new(Channels),
            Box::new(ConsoleAudit),
        ]
    }
}

/// The first migration: one table of reports.
///
/// No column for an address, a name or an account: a reporter is the
/// gateway's pseudonym, and `gateway` is the name its intake token was
/// configured under, not what the gateway claims to be.
#[derive(DeriveMigrationName)]
struct Reports;

#[async_trait::async_trait]
impl MigrationTrait for Reports {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS feedback_report ( \
                 id uuid PRIMARY KEY, \
                 created_at timestamptz NOT NULL DEFAULT now(), \
                 updated_at timestamptz NOT NULL DEFAULT now(), \
                 gateway text NOT NULL, \
                 gateway_name text, \
                 gateway_url text, \
                 gateway_version text NOT NULL, \
                 reporter text NOT NULL, \
                 kind text NOT NULL \
                     CHECK (kind IN ('bug', 'improvement', 'feedback', 'crash', 'other')), \
                 status text NOT NULL DEFAULT 'new' \
                     CHECK (status IN ('new', 'triaged', 'in_progress', 'resolved', \
                                       'wont_fix', 'duplicate')), \
                 text text NOT NULL, \
                 game_id text, \
                 client jsonb NOT NULL, \
                 record bytea, \
                 record_complete boolean)",
        )
        .await?;
        for (name, column) in [
            ("feedback_report_by_time", "created_at"),
            ("feedback_report_by_status", "status"),
            ("feedback_report_by_reporter", "reporter"),
            ("feedback_report_by_game", "game_id"),
        ] {
            db.execute_unprepared(&format!(
                "CREATE INDEX IF NOT EXISTS {name} ON feedback_report ({column})"
            ))
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS feedback_report")
            .await
            .map(|_| ())
    }
}

/// The second migration (#311): the web UI's admins, their sessions, what
/// they changed, and a report's GitHub issue.
///
/// An admin is a name and an Argon2id hash, made only by the binary's
/// `admin` subcommand. A session is kept as the SHA-256 of its token, so a
/// copy of the table opens nothing. The audit names the admin (or `token`,
/// for the admin token) and the report by id, and outlives the report: a
/// deletion is the one change whose record has nowhere else to be.
struct Admins;

/// Named by hand: `DeriveMigrationName` names a migration after its file,
/// and the first one already took this file's name.
impl MigrationName for Admins {
    fn name(&self) -> &'static str {
        "m0002_admins_sessions_audit_issue"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Admins {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "CREATE TABLE IF NOT EXISTS feedback_admin ( \
                 id uuid PRIMARY KEY, \
                 name text NOT NULL UNIQUE, \
                 password_hash text NOT NULL, \
                 created_at timestamptz NOT NULL DEFAULT now())",
            "CREATE TABLE IF NOT EXISTS feedback_session ( \
                 token_hash bytea PRIMARY KEY, \
                 admin_id uuid NOT NULL REFERENCES feedback_admin (id) ON DELETE CASCADE, \
                 created_at timestamptz NOT NULL, \
                 last_seen_at timestamptz NOT NULL)",
            "CREATE INDEX IF NOT EXISTS feedback_session_by_admin ON feedback_session (admin_id)",
            "CREATE TABLE IF NOT EXISTS feedback_audit ( \
                 id uuid PRIMARY KEY, \
                 at timestamptz NOT NULL DEFAULT now(), \
                 actor text NOT NULL, \
                 report_id uuid NOT NULL, \
                 action text NOT NULL, \
                 detail text)",
            "CREATE INDEX IF NOT EXISTS feedback_audit_by_report ON feedback_audit (report_id)",
            "ALTER TABLE feedback_report ADD COLUMN IF NOT EXISTS issue_number integer \
                 CHECK (issue_number > 0)",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "ALTER TABLE feedback_report DROP COLUMN IF EXISTS issue_number",
            "DROP TABLE IF EXISTS feedback_audit",
            "DROP TABLE IF EXISTS feedback_session",
            "DROP TABLE IF EXISTS feedback_admin",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }
}

/// The third migration: how a report came, and who wrote its record.
///
/// `channel` is `gateway` for a report a gateway passed on and `direct` for
/// one a client sent itself, unauthenticated (`POST /client/reports`).
/// `record_origin` is `gateway` or `client` where there is a record: a
/// client's record of a game it hosted itself is unverified and never a
/// gateway's. Every report kept before this came through a gateway, with
/// the gateway's record.
struct Channels;

impl MigrationName for Channels {
    fn name(&self) -> &'static str {
        "m0003_channel_record_origin"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Channels {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "ALTER TABLE feedback_report ADD COLUMN IF NOT EXISTS channel text NOT NULL \
                 DEFAULT 'gateway' CHECK (channel IN ('gateway', 'direct'))",
            "ALTER TABLE feedback_report ADD COLUMN IF NOT EXISTS record_origin text \
                 CHECK (record_origin IN ('gateway', 'client'))",
            "UPDATE feedback_report SET record_origin = 'gateway' \
                 WHERE record IS NOT NULL AND record_origin IS NULL",
            "CREATE INDEX IF NOT EXISTS feedback_report_by_channel ON feedback_report (channel)",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "DROP INDEX IF EXISTS feedback_report_by_channel",
            "ALTER TABLE feedback_report DROP COLUMN IF EXISTS record_origin",
            "ALTER TABLE feedback_report DROP COLUMN IF EXISTS channel",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }
}

/// The fourth migration: the audit also holds changes made through the
/// admin console (`crate::console`), which are about a gateway's keys and
/// no report, so `report_id` may be empty. A report's history still reads
/// only its own rows.
struct ConsoleAudit;

impl MigrationName for ConsoleAudit {
    fn name(&self) -> &'static str {
        "m0004_console_audit"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for ConsoleAudit {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE feedback_audit ALTER COLUMN report_id DROP NOT NULL")
            .await
            .map(|_| ())
    }

    /// The console's rows go with the column's freedom.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "DELETE FROM feedback_audit WHERE report_id IS NULL",
            "ALTER TABLE feedback_audit ALTER COLUMN report_id SET NOT NULL",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }
}
