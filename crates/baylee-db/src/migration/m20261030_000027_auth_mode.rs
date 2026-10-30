//! Who a gateway lets in, as an admin chose it in the console
//! (`docs/central-login.md` §"Who a gateway lets in"): at most one row, the
//! mode (`guests`, `central` or `mixed`), when it was set and by whom. No
//! row is the default, `central`; the operator's `BAYLEE_AUTH_MODE` takes
//! precedence and is never written here.
#![forbid(unsafe_code)]
use sea_orm_migration::prelude::*;

/// The twenty-seventh migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE auth_mode (
                     id smallint PRIMARY KEY DEFAULT 1 CHECK (id = 1),
                     mode text NOT NULL CHECK (mode IN ('guests', 'central', 'mixed')),
                     set_at timestamptz NOT NULL DEFAULT clock_timestamp(),
                     set_by text NOT NULL CHECK (char_length(set_by) BETWEEN 1 AND 200)
                 );
                 REVOKE ALL ON auth_mode FROM PUBLIC;",
            )
            .await?;
        Ok(())
    }

    /// Drops the choice: the gateway goes back to the default until an
    /// admin chooses again.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE auth_mode;")
            .await?;
        Ok(())
    }
}
