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
        ]
    }
}
