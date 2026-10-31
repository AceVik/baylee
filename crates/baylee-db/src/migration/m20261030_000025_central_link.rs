//! The gateway's link to Baylee Central by a token an admin pasted into the
//! console (baylee-central `docs/gateway-registry.md` §"Gateway tokens"): at
//! most one row, the token itself (the gateway presents it to Central, so it
//! cannot be a digest here), the gateway's id and origin as Central named
//! them when the token was set, and who set it. The operator's file
//! (`BAYLEE_CENTRAL_CONFIG_FILE`) takes precedence and is never written here.
#![forbid(unsafe_code)]
use sea_orm_migration::prelude::*;

/// The twenty-fifth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE central_link (
                     id smallint PRIMARY KEY DEFAULT 1 CHECK (id = 1),
                     token bytea NOT NULL CHECK (octet_length(token) = 32),
                     gateway uuid NOT NULL,
                     origin text NOT NULL CHECK (char_length(origin) BETWEEN 1 AND 256),
                     set_at timestamptz NOT NULL DEFAULT clock_timestamp(),
                     set_by text NOT NULL CHECK (char_length(set_by) BETWEEN 1 AND 200)
                 );
                 REVOKE ALL ON central_link FROM PUBLIC;",
            )
            .await?;
        Ok(())
    }

    /// Drops the link: the gateway runs unlinked until a token is pasted again.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE central_link;")
            .await?;
        Ok(())
    }
}
