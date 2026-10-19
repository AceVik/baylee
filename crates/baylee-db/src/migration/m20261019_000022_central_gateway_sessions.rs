#![forbid(unsafe_code)]
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "ALTER TABLE central_account_link DROP CONSTRAINT central_account_link_central_id_check;
             ALTER TABLE central_account_link ADD CONSTRAINT central_account_link_central_id_check
                 CHECK (central_id IS NULL OR uuid_extract_version(central_id) IS NOT DISTINCT FROM 7);
             ALTER TABLE account DROP CONSTRAINT account_can_sign_in;
             ALTER TABLE account ADD CONSTRAINT account_can_sign_in CHECK (
                 guest OR central_only OR (password_hash IS NOT NULL
                 AND (username_key IS NOT NULL OR email IS NOT NULL)));
             CREATE TABLE central_gateway_session (
                 token_hash bytea PRIMARY KEY REFERENCES session_token(token_hash) ON DELETE CASCADE,
                 lease bytea NOT NULL CHECK (octet_length(lease) = 32),
                 purpose text NOT NULL CHECK (purpose IN ('admit', 'admin')),
                 expires_at timestamptz NOT NULL
             );
             REVOKE ALL ON central_gateway_session FROM PUBLIC;"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // A previously accepted identity must never regain the nullable UUID check.
        // Central projections have no local credential to reconstruct either.
        manager
            .get_connection()
            .execute_unprepared(
                "DO $guard$ BEGIN
                 IF EXISTS (SELECT 1 FROM central_account_link WHERE central_id IS NOT NULL) THEN
                     RAISE EXCEPTION 'Central identities prevent gateway session rollback';
                 END IF;
             END $guard$;
             DROP TABLE central_gateway_session;
             ALTER TABLE account DROP CONSTRAINT account_can_sign_in;
             ALTER TABLE account ADD CONSTRAINT account_can_sign_in CHECK (
                 guest OR ((password_hash IS NOT NULL OR central_only)
                 AND (username_key IS NOT NULL OR email IS NOT NULL)));",
            )
            .await?;
        Ok(())
    }
}
