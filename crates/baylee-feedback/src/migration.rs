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
        vec![Box::new(Reports)]
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
