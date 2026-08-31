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

#[cfg(test)]
mod tests {
    use super::*;

    /// Card files inherit unstated fields from [`dsl::CardDef::DEFAULT`],
    /// whose `index` is 0. That is a deliberate collision: a file that
    /// forgets its index would otherwise shadow card 0 in `by_index` and
    /// hand the engine the wrong card at cast time. This test is what
    /// turns that collision into a build failure.
    #[test]
    fn every_card_sits_at_the_index_it_claims() {
        for (i, def) in generated::BY_INDEX.iter().enumerate() {
            assert_eq!(
                def.index.get() as usize,
                i,
                "{} is registered at index {i} but claims {}",
                def.name(),
                def.index.get()
            );
        }
        assert_eq!(generated::BY_INDEX.len(), generated::ALL.len());
    }

    /// The same for the oracle-id table the gateway resolves deck lists
    /// against: an empty or copied `oracle_id` would silently resolve one
    /// card's name to another card's rules.
    #[test]
    fn every_card_answers_to_the_oracle_id_it_is_filed_under() {
        for (oracle, def) in generated::ALL {
            assert_eq!(def.oracle_id, *oracle, "{} is misfiled", def.name());
            assert!(!def.oracle_id.is_empty());
            assert!(!def.scryfall_id.is_empty());
            assert!(!def.faces.is_empty(), "{} has no faces", def.name());
            assert!(!def.name().is_empty());
        }
    }

    /// `coverage` defaults to `Unimplemented`, so a card that reaches the
    /// acceptance pool without the line is reported as a stub rather than
    /// quietly offered to deckbuilders.
    #[test]
    fn coverage_is_claimed_explicitly_or_not_at_all() {
        let unimplemented: Vec<&str> = generated::ALL
            .iter()
            .filter(|(_, d)| !d.is_implemented())
            .map(|(_, d)| d.name())
            .collect();
        assert!(
            unimplemented.is_empty(),
            "acceptance pool contains stubs: {unimplemented:?}"
        );
    }
}
