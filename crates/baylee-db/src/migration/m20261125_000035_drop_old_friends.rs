//! The friend lists from before Baylee Central go (owner, 25.11.2026).
//!
//! Contacts and groups have lived at Central since the cutover
//! (`docs/central-chat-sync.md`); no client sends a friend request or a group
//! change to a gateway any more, and nothing was carried over from here. What
//! the gateway still held of them was kept only until the player deleted it,
//! so this deletes it for every account: every friendship and friend request,
//! every group with its members and invitations, and every message still
//! waiting to be delivered in a group's conversation (`grp:…`), its
//! deliveries by cascade.
//!
//! Blocks (`social_block`) stay: they still keep a blocked player's table
//! chat away (`blockers_of`), and Central's blocks do not reach a gateway.
//! Devices, settings and the messages of tables and teams stay as they are.
//!
//! Every table is named with the schema the migrator runs in: unqualified, a
//! statement that finds no table in this schema follows the search path on
//! into `public` (`docs/dev/database.md`). Nothing comes back on the way
//! down; the rows are gone, and the tables were never dropped.
#![forbid(unsafe_code)]
use sea_orm_migration::prelude::*;

/// The thirty-fifth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DO $forget$ BEGIN \
                   EXECUTE format('DELETE FROM %I.chat_message WHERE starts_with(conv, ''grp:'')', \
                                  current_schema()); \
                   EXECUTE format('DELETE FROM %I.group_invite', current_schema()); \
                   EXECUTE format('DELETE FROM %I.group_member', current_schema()); \
                   EXECUTE format('DELETE FROM %I.chat_group', current_schema()); \
                   EXECUTE format('DELETE FROM %I.friend_request', current_schema()); \
                   EXECUTE format('DELETE FROM %I.friendship', current_schema()); \
                 END $forget$",
            )
            .await?;
        Ok(())
    }

    /// The tables stay as they were; what was deleted does not come back.
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
