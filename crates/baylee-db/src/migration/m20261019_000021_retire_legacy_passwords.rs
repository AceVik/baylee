#![forbid(unsafe_code)]
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "ALTER TABLE account ADD COLUMN central_only boolean NOT NULL DEFAULT false;
             ALTER TABLE account DROP CONSTRAINT account_can_sign_in;
             ALTER TABLE account ADD CONSTRAINT account_can_sign_in CHECK (
                 guest OR ((password_hash IS NOT NULL OR central_only)
                 AND (username_key IS NOT NULL OR email IS NOT NULL)));
             ALTER TABLE account ADD CONSTRAINT account_central_holds_no_password
                 CHECK (NOT central_only OR (NOT guest AND password_hash IS NULL));
             UPDATE account SET central_only = true, password_hash = NULL
                 WHERE id IN (SELECT account_id FROM central_account_link WHERE central_id IS NOT NULL);"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // A retired verifier cannot be reconstructed. Empty/test databases may roll back;
        // a real cutover must keep its mapping and never re-enable the old password route.
        manager.get_connection().execute_unprepared(
            "DO $guard$ BEGIN
                 IF EXISTS (SELECT 1 FROM account WHERE central_only) THEN
                     RAISE EXCEPTION 'Central-only accounts prevent credential rollback';
                 END IF;
             END $guard$;
             ALTER TABLE account DROP CONSTRAINT account_central_holds_no_password;
             ALTER TABLE account DROP CONSTRAINT account_can_sign_in;
             ALTER TABLE account ADD CONSTRAINT account_can_sign_in CHECK (
                 guest OR (password_hash IS NOT NULL AND (username_key IS NOT NULL OR email IS NOT NULL)));
             ALTER TABLE account DROP COLUMN central_only;"
        ).await?;
        Ok(())
    }
}
