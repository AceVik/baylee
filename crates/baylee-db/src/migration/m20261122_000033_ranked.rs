//! Host-private launch fence. Public ranked data belongs to Central.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE ranked_run (
                match_id uuid PRIMARY KEY, game uuid NOT NULL UNIQUE,
                launched boolean NOT NULL DEFAULT false,
                seen_at timestamptz NOT NULL DEFAULT clock_timestamp(),
                serviced_at timestamptz NOT NULL DEFAULT clock_timestamp()
             );
             CREATE INDEX ranked_delivery ON ranked_run(serviced_at,match_id);",
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("ranked_run")).to_owned())
            .await
    }
}
