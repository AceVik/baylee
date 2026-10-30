//! Schema migrations, in order.
//!
//! The list below is the schema's history and is append-only. A migration
//! that has run somewhere is never edited — it is followed by another one —
//! because the only thing a migrator knows about a database is which names
//! in this list it has already applied.

use sea_orm_migration::prelude::*;

mod decklist;
mod house_decks;
mod m20260915_000001_account_side;
mod m20260916_000002_deck_kinds_and_history;
mod m20260918_000003_real_decks;
mod m20260918_000004_astra_decks;
mod m20260924_000005_drop_standing_answer;
mod m20260924_000006_usernames;
mod m20260925_000007_guests;
mod m20260925_000008_upload_owners;
mod m20260927_000009_game_records;
mod m20260927_000010_invites;
mod m20260929_000011_maik_deck;
mod m20260930_000012_precons;
mod m20261006_000013_delegated_seats;
mod m20261006_000014_sessions_by_account;
mod m20261007_000015_terms;
mod m20261012_000016_account_moderation;
mod m20261012_000016_card_blocks;
mod m20261013_000017_social;
mod m20261013_000018_replays;
mod m20261013_000019_llm_access;
mod m20261013_000019_replay_shares;
mod m20261019_000020_central_accounts;
mod m20261019_000021_retire_legacy_passwords;
mod m20261019_000022_central_gateway_sessions;
mod m20261019_000023_bind_migration_credentials;
mod m20261020_000024_gateway_terms;
mod m20261030_000026_end_password_sessions;
mod m20261030_000027_auth_mode;

/// The migrator the gateway runs on connect.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260915_000001_account_side::Migration),
            Box::new(m20260916_000002_deck_kinds_and_history::Migration),
            Box::new(m20260918_000003_real_decks::Migration),
            Box::new(m20260918_000004_astra_decks::Migration),
            Box::new(m20260924_000005_drop_standing_answer::Migration),
            Box::new(m20260924_000006_usernames::Migration),
            Box::new(m20260925_000007_guests::Migration),
            Box::new(m20260925_000008_upload_owners::Migration),
            Box::new(m20260927_000009_game_records::Migration),
            Box::new(m20260927_000010_invites::Migration),
            Box::new(m20260929_000011_maik_deck::Migration),
            Box::new(m20260930_000012_precons::Migration),
            Box::new(m20261006_000013_delegated_seats::Migration),
            Box::new(m20261006_000014_sessions_by_account::Migration),
            Box::new(m20261007_000015_terms::Migration),
            Box::new(m20261012_000016_account_moderation::Migration),
            Box::new(m20261012_000016_card_blocks::Migration),
            Box::new(m20261013_000017_social::Migration),
            Box::new(m20261013_000018_replays::Migration),
            // Two lanes both took 000019; each name is its own row in
            // `seaql_migrations`, so neither is renamed, and this order is
            // the one they run in (as with the two 000016s).
            Box::new(m20261013_000019_llm_access::Migration),
            Box::new(m20261013_000019_replay_shares::Migration),
            Box::new(m20261019_000020_central_accounts::Migration),
            Box::new(m20261019_000021_retire_legacy_passwords::Migration),
            Box::new(m20261019_000022_central_gateway_sessions::Migration),
            Box::new(m20261019_000023_bind_migration_credentials::Migration),
            Box::new(m20261020_000024_gateway_terms::Migration),
            Box::new(m20261030_000026_end_password_sessions::Migration),
            Box::new(m20261030_000027_auth_mode::Migration),
        ]
    }
}
