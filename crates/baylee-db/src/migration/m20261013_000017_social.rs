//! Friends, chat devices, groups and undelivered chat messages (beta.8,
//! `docs/protocol.md` §"Friends and chat (beta.8)").
//!
//! Number 17 is provisional: other beta.8 lanes add migrations too, and the
//! one that lands later is renumbered. Nothing in it depends on another
//! lane's.
//!
//! - Three columns on `account`, the social settings the gateway enforces:
//!   `whispers_from` (0 friends, 1 friends and table mates), `requests_open`,
//!   `share_table`.
//! - `social_device`: one row per chat device, its public keys and the
//!   statement that binds them to the account's tag. Never a secret.
//! - `friendship` (one row per pair, `a < b`), `friend_request`,
//!   `social_block`.
//! - `chat_group`, `group_member` (at most one owner, by a partial unique
//!   index), `group_invite`.
//! - `chat_message`, the ciphertext stored once without its wraps, and
//!   `chat_delivery`, one slot per recipient device holding that device's
//!   wrap until it acks or the slot expires.
//!
//! Every reference to an account cascades, so deleting an account leaves
//! nothing about its chat behind; a group's ownership passes on in the
//! statement that follows the deletion (`social::reown`).

use sea_orm_migration::prelude::*;

/// The seventeenth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

/// The statements `up` runs, in order. Each is idempotent, so a restart
/// against a schema that already has them changes nothing.
const UP: &[&str] = &[
    "ALTER TABLE account ADD COLUMN IF NOT EXISTS whispers_from smallint NOT NULL DEFAULT 1",
    "ALTER TABLE account ADD COLUMN IF NOT EXISTS requests_open boolean NOT NULL DEFAULT true",
    "ALTER TABLE account ADD COLUMN IF NOT EXISTS share_table boolean NOT NULL DEFAULT true",
    "CREATE TABLE IF NOT EXISTS social_device ( \
         id text PRIMARY KEY CONSTRAINT social_device_id_shape CHECK (id ~ '^[0-9a-f]{16}$'), \
         account_id uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         ik bytea NOT NULL UNIQUE CONSTRAINT social_device_ik_length CHECK (length(ik) = 32), \
         dk bytea NOT NULL CONSTRAINT social_device_dk_length CHECK (length(dk) = 32), \
         dsig bytea NOT NULL CONSTRAINT social_device_dsig_length CHECK (length(dsig) = 64), \
         label text NOT NULL CONSTRAINT social_device_label_length \
             CHECK (char_length(label) BETWEEN 1 AND 24), \
         added_at timestamptz NOT NULL DEFAULT now(), \
         seen_at timestamptz NOT NULL DEFAULT now() \
     )",
    "CREATE INDEX IF NOT EXISTS social_device_account ON social_device (account_id)",
    "CREATE TABLE IF NOT EXISTS friendship ( \
         a uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         b uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         since timestamptz NOT NULL DEFAULT now(), \
         PRIMARY KEY (a, b), \
         CONSTRAINT friendship_ordered CHECK (a < b) \
     )",
    "CREATE INDEX IF NOT EXISTS friendship_b ON friendship (b)",
    "CREATE TABLE IF NOT EXISTS friend_request ( \
         from_id uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         to_id uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         at timestamptz NOT NULL DEFAULT now(), \
         PRIMARY KEY (from_id, to_id), \
         CONSTRAINT friend_request_not_self CHECK (from_id <> to_id) \
     )",
    "CREATE INDEX IF NOT EXISTS friend_request_to ON friend_request (to_id)",
    "CREATE TABLE IF NOT EXISTS social_block ( \
         blocker uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         blocked uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         at timestamptz NOT NULL DEFAULT now(), \
         PRIMARY KEY (blocker, blocked), \
         CONSTRAINT social_block_not_self CHECK (blocker <> blocked) \
     )",
    "CREATE INDEX IF NOT EXISTS social_block_blocked ON social_block (blocked)",
    "CREATE TABLE IF NOT EXISTS chat_group ( \
         id uuid PRIMARY KEY DEFAULT uuidv7(), \
         name text NOT NULL CONSTRAINT chat_group_name_length \
             CHECK (char_length(name) BETWEEN 1 AND 48), \
         created_at timestamptz NOT NULL DEFAULT now() \
     )",
    "CREATE TABLE IF NOT EXISTS group_member ( \
         group_id uuid NOT NULL REFERENCES chat_group (id) ON DELETE CASCADE, \
         account_id uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         owner boolean NOT NULL DEFAULT false, \
         joined_at timestamptz NOT NULL DEFAULT clock_timestamp(), \
         PRIMARY KEY (group_id, account_id) \
     )",
    "CREATE UNIQUE INDEX IF NOT EXISTS group_one_owner ON group_member (group_id) WHERE owner",
    "CREATE INDEX IF NOT EXISTS group_member_account ON group_member (account_id)",
    "CREATE TABLE IF NOT EXISTS group_invite ( \
         group_id uuid NOT NULL REFERENCES chat_group (id) ON DELETE CASCADE, \
         account_id uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         inviter_id uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         at timestamptz NOT NULL DEFAULT now(), \
         PRIMARY KEY (group_id, account_id) \
     )",
    "CREATE INDEX IF NOT EXISTS group_invite_account ON group_invite (account_id)",
    "CREATE TABLE IF NOT EXISTS chat_message ( \
         id uuid PRIMARY KEY, \
         conv text NOT NULL, \
         sender_id uuid NOT NULL REFERENCES account (id) ON DELETE CASCADE, \
         sender_dev text NOT NULL, \
         sealed bytea NOT NULL, \
         at timestamptz NOT NULL DEFAULT now() \
     )",
    "CREATE INDEX IF NOT EXISTS chat_message_sender ON chat_message (sender_id)",
    "CREATE TABLE IF NOT EXISTS chat_delivery ( \
         seq bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, \
         device_id text NOT NULL REFERENCES social_device (id) ON DELETE CASCADE, \
         message_id uuid NOT NULL REFERENCES chat_message (id) ON DELETE CASCADE, \
         wrap bytea NOT NULL CONSTRAINT chat_delivery_wrap_length CHECK (length(wrap) = 48), \
         expires_at timestamptz NOT NULL \
     )",
    "CREATE INDEX IF NOT EXISTS chat_delivery_device ON chat_delivery (device_id, seq)",
    "CREATE INDEX IF NOT EXISTS chat_delivery_expires ON chat_delivery (expires_at)",
    "CREATE INDEX IF NOT EXISTS chat_delivery_message ON chat_delivery (message_id)",
];

/// The tables `down` drops, children first. Each is dropped from the schema
/// being migrated and from nowhere else: a bare `DROP TABLE IF EXISTS`
/// walks the search path and would drop the next one of that name it found.
const TABLES: &[&str] = &[
    "chat_delivery",
    "chat_message",
    "group_invite",
    "group_member",
    "chat_group",
    "social_block",
    "friend_request",
    "friendship",
    "social_device",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for statement in UP {
            db.execute_unprepared(statement).await?;
        }
        Ok(())
    }

    /// Forgets every chat device, friendship, request, block, group and
    /// undelivered message, and the three settings. The accounts stay.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for table in TABLES {
            db.execute_unprepared(&format!(
                "DO $drop$ BEGIN \
                   EXECUTE format('DROP TABLE IF EXISTS %I.{table}', current_schema()); \
                 END $drop$"
            ))
            .await?;
        }
        for column in ["share_table", "requests_open", "whispers_from"] {
            db.execute_unprepared(&format!(
                "ALTER TABLE account DROP COLUMN IF EXISTS {column}"
            ))
            .await?;
        }
        Ok(())
    }
}
