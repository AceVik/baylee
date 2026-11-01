//! The cutover's end of the password sessions (`docs/central-login.md`
//! §"Cutover"): from this release on nobody signs in here with a password,
//! so every session of an account that is neither a guest nor Baylee
//! Central's ends with the deploy. The accounts stay, game data and all:
//! Central signs them in from its legacy store and moves them, and the first
//! admission links each one back (`central::adopt`). Running it again ends
//! nothing new, since no such session can be made any more.
#![forbid(unsafe_code)]
use sea_orm_migration::prelude::*;

/// The twenty-sixth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DELETE FROM session_token WHERE account_id IN
                     (SELECT id FROM account WHERE NOT guest AND NOT central_only)",
            )
            .await?;
        Ok(())
    }

    /// Ended sessions are not made again; going back changes nothing.
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
