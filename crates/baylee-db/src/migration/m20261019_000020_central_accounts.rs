use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE central_account_link (
                account_id uuid PRIMARY KEY REFERENCES account(id) ON DELETE CASCADE,
                central_id uuid UNIQUE,
                snapshot_digest bytea NOT NULL CHECK (octet_length(snapshot_digest) = 32),
                prepared_at timestamptz NOT NULL DEFAULT clock_timestamp(),
                linked_at timestamptz,
                CHECK ((central_id IS NULL) = (linked_at IS NULL)),
                CHECK (central_id IS NULL OR uuid_extract_version(central_id) = 7)
            );
            REVOKE ALL ON central_account_link FROM PUBLIC;",
            )
            .await?;
        Ok(())
    }
}
