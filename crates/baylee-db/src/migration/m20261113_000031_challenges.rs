//! Private official challenge claims and durable result delivery.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE challenge_run (
                attempt uuid PRIMARY KEY,
                game uuid NOT NULL UNIQUE,
                account uuid NOT NULL REFERENCES account(id) ON DELETE CASCADE,
                central_account uuid NOT NULL,
                launched boolean NOT NULL DEFAULT false,
                touched_at timestamptz NOT NULL DEFAULT clock_timestamp(),
                result bytea CHECK (octet_length(result) <= 4096)
            );
            CREATE INDEX challenge_run_delivery ON challenge_run(touched_at, attempt);
            CREATE INDEX challenge_run_account ON challenge_run(account);",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "DO $drop$ BEGIN EXECUTE format('DROP TABLE %I.challenge_run', current_schema()); END $drop$"
        ).await?;
        Ok(())
    }
}
