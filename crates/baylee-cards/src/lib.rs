//! baylee-cards — the compiled card registry.
//!
//! One file per card in [`cards`], dense lookup tables in [`generated`]
//! (both produced by `cargo xtask codegen`).

#![warn(missing_docs)]

use baylee_cards_dsl::CardDef;
use baylee_core::ids::CardIndex;

/// Generated: one module per card.
pub mod cards;
/// Deck parsing and name resolution against the registry (acceptance
/// deck format, `"N Card Name"` lines, preset assembly).
pub mod decks;
/// Generated: registry tables.
pub mod generated;
/// Central named token definitions (referenced by card files).
pub mod tokens;

pub use baylee_cards_dsl as dsl;

/// Looks up a card definition by Scryfall oracle id.
#[must_use]
pub fn by_oracle_id(oracle_id: &str) -> Option<&'static CardDef> {
    generated::by_oracle_id(oracle_id)
}

/// Looks up a card definition by its dense runtime index.
#[must_use]
pub fn by_index(index: CardIndex) -> Option<&'static CardDef> {
    generated::by_index(index)
}

/// Number of registered cards.
#[must_use]
pub fn count() -> usize {
    generated::ALL.len()
}

/// Hash of the whole pool (client cache invalidation / gateway handshake).
#[must_use]
pub fn pool_hash() -> u64 {
    generated::POOL_HASH
}
