//! A Central session lives in the gateway's memory from now on
//! (`docs/central-login.md` §"Sessions"), so the leases go, and with them every
//! stored session of a Central account: its bearer was only ever good together
//! with its lease, and a client signs in again with a gateway token. Guests and
//! local accounts keep their sessions.
#![forbid(unsafe_code)]
use sea_orm_migration::prelude::*;

/// The twenty-ninth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TABLE central_gateway_session;
                 DELETE FROM session_token WHERE account_id IN
                     (SELECT id FROM account WHERE central_only);",
            )
            .await?;
        Ok(())
    }

    /// The table comes back empty: ended sessions are not made again, and a
    /// client of the release before signs in again.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE central_gateway_session (
                     token_hash bytea PRIMARY KEY REFERENCES session_token(token_hash) ON DELETE CASCADE,
                     lease bytea NOT NULL CHECK (octet_length(lease) = 32),
                     purpose text NOT NULL CHECK (purpose IN ('admit', 'admin')),
                     expires_at timestamptz NOT NULL
                 );
                 REVOKE ALL ON central_gateway_session FROM PUBLIC;",
            )
            .await?;
        Ok(())
    }
}
