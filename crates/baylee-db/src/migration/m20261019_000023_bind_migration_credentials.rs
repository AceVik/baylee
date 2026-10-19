#![forbid(unsafe_code)]
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                // Keep old pending markers, but never infer which historical verifier
                // authorized them. NULL requires a fresh proof and still blocks password
                // changes while the official invalidation bridge is unavailable.
                "ALTER TABLE central_account_link ADD COLUMN source_credential_hash text;",
            )
            .await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DO $guard$ BEGIN
                 IF EXISTS (SELECT 1 FROM central_account_link) THEN
                     RAISE EXCEPTION 'Migration bindings prevent credential guard rollback';
                 END IF;
             END $guard$;
             ALTER TABLE central_account_link DROP COLUMN source_credential_hash;",
            )
            .await?;
        Ok(())
    }
}
