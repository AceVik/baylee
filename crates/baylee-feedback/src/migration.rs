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
            Box::new(Contact),
            Box::new(ChatKind),
            Box::new(Triage),
            Box::new(Source),
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

/// The fifth migration: messages from the landing page's contact form
/// (`crate::contact`), and their changes in the audit.
///
/// A message is what its sender typed (a name and an e-mail address only
/// if they gave one), the page's language and whether an admin has dealt
/// with it; no address, no device, nothing about the request. The audit
/// names a message by `contact_id` and never carries its text or address.
struct Contact;

impl MigrationName for Contact {
    fn name(&self) -> &'static str {
        "m0005_contact"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Contact {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "CREATE TABLE IF NOT EXISTS feedback_contact ( \
                 id uuid PRIMARY KEY, \
                 created_at timestamptz NOT NULL DEFAULT now(), \
                 updated_at timestamptz NOT NULL DEFAULT now(), \
                 name text, \
                 email text, \
                 message text NOT NULL, \
                 language text NOT NULL CHECK (language IN ('en', 'de')), \
                 status text NOT NULL DEFAULT 'new' CHECK (status IN ('new', 'done')), \
                 done_at timestamptz)",
            "CREATE INDEX IF NOT EXISTS feedback_contact_by_time ON feedback_contact (created_at)",
            "CREATE INDEX IF NOT EXISTS feedback_contact_by_status ON feedback_contact (status)",
            "ALTER TABLE feedback_audit ADD COLUMN IF NOT EXISTS contact_id uuid",
            "CREATE INDEX IF NOT EXISTS feedback_audit_by_contact ON feedback_audit (contact_id)",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "DELETE FROM feedback_audit WHERE contact_id IS NOT NULL",
            "DROP INDEX IF EXISTS feedback_audit_by_contact",
            "ALTER TABLE feedback_audit DROP COLUMN IF EXISTS contact_id",
            "DROP TABLE IF EXISTS feedback_contact",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }
}

/// The sixth migration: a report may be of a chat message (`kind = 'chat'`,
/// beta.8), which only a gateway sends. The first migration's check was
/// unnamed, so Postgres named it `feedback_report_kind_check`; it is
/// replaced by one that also takes `chat`.
struct ChatKind;

impl MigrationName for ChatKind {
    fn name(&self) -> &'static str {
        "m0006_chat_kind"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for ChatKind {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "ALTER TABLE feedback_report DROP CONSTRAINT IF EXISTS feedback_report_kind_check",
            "ALTER TABLE feedback_report ADD CONSTRAINT feedback_report_kind_check \
             CHECK (kind IN ('bug', 'improvement', 'feedback', 'crash', 'other', 'chat'))",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }

    /// Chat reports go with the kind.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "DELETE FROM feedback_report WHERE kind = 'chat'",
            "ALTER TABLE feedback_report DROP CONSTRAINT IF EXISTS feedback_report_kind_check",
            "ALTER TABLE feedback_report ADD CONSTRAINT feedback_report_kind_check \
             CHECK (kind IN ('bug', 'improvement', 'feedback', 'crash', 'other'))",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }
}

/// The seventh migration: triage (`docs/feedback.md` §"Triage").
///
/// On the report: who looks after it (`assignee`, an admin's name), how
/// urgent it is (`priority`), free `labels`, the report it duplicates
/// (`duplicate_of`), a `revision` that every change moves on (a change that
/// names another revision is refused), when it was dealt with (`done_at`)
/// and when it went into the trash (`deleted_at`; purged after
/// [`crate::triage::TRASH_DAYS`]). Two columns Postgres keeps itself: a
/// crash's `signature` (its message with numbers taken out and its file
/// without line numbers, hashed with MD5: a grouping key, nothing secret),
/// which groups the same crash, and `search`,
/// the words of the text and of a crash's message.
///
/// Three tables: internal notes (their own rows, not audit lines), an admin's
/// saved views, and which reports an admin has opened.
struct Triage;

impl MigrationName for Triage {
    fn name(&self) -> &'static str {
        "m0007_triage"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Triage {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "ALTER TABLE feedback_report \
                 ADD COLUMN IF NOT EXISTS assignee text \
                     CHECK (assignee IS NULL OR char_length(assignee) BETWEEN 1 AND 64), \
                 ADD COLUMN IF NOT EXISTS priority text \
                     CHECK (priority IN ('low', 'normal', 'high', 'critical')), \
                 ADD COLUMN IF NOT EXISTS labels text[] NOT NULL DEFAULT '{}' \
                     CHECK (cardinality(labels) <= 16), \
                 ADD COLUMN IF NOT EXISTS duplicate_of uuid \
                     REFERENCES feedback_report (id) ON DELETE SET NULL, \
                 ADD COLUMN IF NOT EXISTS revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0), \
                 ADD COLUMN IF NOT EXISTS done_at timestamptz, \
                 ADD COLUMN IF NOT EXISTS deleted_at timestamptz, \
                 ADD CONSTRAINT feedback_report_not_its_own_duplicate CHECK (duplicate_of <> id)",
            "ALTER TABLE feedback_report ADD COLUMN IF NOT EXISTS signature text \
                 GENERATED ALWAYS AS (CASE WHEN kind = 'crash' THEN left(md5( \
                     regexp_replace(coalesce(client -> 'crash' ->> 'message', ''), '[0-9]+', '#', 'g') \
                     || '|' || \
                     regexp_replace(coalesce(client -> 'crash' ->> 'location', ''), ':[0-9:]+$', '') \
                     ), 16) END) STORED",
            "ALTER TABLE feedback_report ADD COLUMN IF NOT EXISTS search tsvector \
                 GENERATED ALWAYS AS (to_tsvector('simple', text || ' ' || \
                     coalesce(client -> 'crash' ->> 'message', ''))) STORED",
            "UPDATE feedback_report SET done_at = updated_at \
                 WHERE status IN ('resolved', 'wont_fix', 'duplicate') AND done_at IS NULL",
            "CREATE INDEX IF NOT EXISTS feedback_report_by_signature ON feedback_report (signature) \
                 WHERE signature IS NOT NULL",
            "CREATE INDEX IF NOT EXISTS feedback_report_by_assignee ON feedback_report (assignee)",
            "CREATE INDEX IF NOT EXISTS feedback_report_by_search ON feedback_report USING gin (search)",
            "CREATE INDEX IF NOT EXISTS feedback_report_by_labels ON feedback_report USING gin (labels)",
            "CREATE INDEX IF NOT EXISTS feedback_report_trashed ON feedback_report (deleted_at) \
                 WHERE deleted_at IS NOT NULL",
            "CREATE TABLE IF NOT EXISTS feedback_note ( \
                 id uuid PRIMARY KEY, \
                 report_id uuid NOT NULL REFERENCES feedback_report (id) ON DELETE CASCADE, \
                 at timestamptz NOT NULL DEFAULT now(), \
                 author text NOT NULL, \
                 text text NOT NULL CHECK (char_length(text) BETWEEN 1 AND 4000))",
            "CREATE INDEX IF NOT EXISTS feedback_note_by_report ON feedback_note (report_id, at)",
            "CREATE TABLE IF NOT EXISTS feedback_view ( \
                 id uuid PRIMARY KEY, \
                 admin_id uuid NOT NULL REFERENCES feedback_admin (id) ON DELETE CASCADE, \
                 name text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 60), \
                 query text NOT NULL CHECK (char_length(query) <= 2000), \
                 created_at timestamptz NOT NULL DEFAULT now(), \
                 UNIQUE (admin_id, name))",
            "CREATE TABLE IF NOT EXISTS feedback_seen ( \
                 admin_id uuid NOT NULL REFERENCES feedback_admin (id) ON DELETE CASCADE, \
                 report_id uuid NOT NULL REFERENCES feedback_report (id) ON DELETE CASCADE, \
                 at timestamptz NOT NULL DEFAULT now(), \
                 PRIMARY KEY (admin_id, report_id))",
            "CREATE INDEX IF NOT EXISTS feedback_seen_by_report ON feedback_seen (report_id)",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in [
            "DROP TABLE IF EXISTS feedback_seen",
            "DROP TABLE IF EXISTS feedback_view",
            "DROP TABLE IF EXISTS feedback_note",
            "ALTER TABLE feedback_report DROP CONSTRAINT IF EXISTS feedback_report_not_its_own_duplicate, \
                 DROP COLUMN IF EXISTS search, DROP COLUMN IF EXISTS signature, \
                 DROP COLUMN IF EXISTS deleted_at, DROP COLUMN IF EXISTS done_at, \
                 DROP COLUMN IF EXISTS revision, DROP COLUMN IF EXISTS duplicate_of, \
                 DROP COLUMN IF EXISTS labels, DROP COLUMN IF EXISTS priority, \
                 DROP COLUMN IF EXISTS assignee",
        ] {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }
}

/// The eighth migration: what sent a report, as a column of its own
/// (`docs/feedback.md` §"Reading"). The list, its facets and the source
/// filter read it instead of unpacking `client`, which holds up to 2 MiB
/// (a screenshot, a log) out of line. Only a string counts: an empty one,
/// a number or an object is no source.
struct Source;

impl MigrationName for Source {
    fn name(&self) -> &'static str {
        "m0008_source"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Source {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE feedback_report ADD COLUMN IF NOT EXISTS source text \
                 GENERATED ALWAYS AS (CASE WHEN jsonb_typeof(client -> 'source') = 'string' \
                 THEN nullif(left(client ->> 'source', 64), '') END) STORED",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE feedback_report DROP COLUMN IF EXISTS source")
            .await?;
        Ok(())
    }
}
