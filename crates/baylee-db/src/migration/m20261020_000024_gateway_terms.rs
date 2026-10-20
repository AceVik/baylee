//! The terms of use an admin publishes in the console (`docs/protocol.md`
//! §"Terms of use (WG-1)"): every version, immutable once published, the
//! newest one in force. Acceptances are not here: a Central account's live on
//! Baylee Central, a legacy account's in `account.terms_version`.
#![forbid(unsafe_code)]
use sea_orm_migration::prelude::*;

/// The twenty-fourth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE gateway_terms (
                     version text PRIMARY KEY CHECK (version ~ '^[!-~]{1,64}$' AND version <> 'draft'),
                     digest bytea NOT NULL CHECK (octet_length(digest) = 32),
                     published_at timestamptz NOT NULL DEFAULT clock_timestamp(),
                     published_by text NOT NULL CHECK (char_length(published_by) BETWEEN 1 AND 200)
                 );
                 CREATE INDEX gateway_terms_newest ON gateway_terms (published_at DESC, version DESC);
                 CREATE TABLE gateway_terms_text (
                     version text NOT NULL REFERENCES gateway_terms(version) ON DELETE CASCADE,
                     lang text NOT NULL CHECK (lang ~ '^[a-z]{2,3}$'),
                     updated text CHECK (updated ~ '^[!-~]{1,32}$'),
                     markdown text NOT NULL CHECK (octet_length(markdown) BETWEEN 1 AND 65536),
                     PRIMARY KEY (version, lang)
                 );
                 REVOKE ALL ON gateway_terms, gateway_terms_text FROM PUBLIC;",
            )
            .await?;
        Ok(())
    }

    /// Drops every published version: the gateway falls back to its file or
    /// the default terms, and players are asked again.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE gateway_terms_text; DROP TABLE gateway_terms;")
            .await?;
        Ok(())
    }
}
