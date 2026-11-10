//! baylee-cards-base — what a card file is written with, and the door to the
//! registry.
//!
//! A card file names shared tokens and filters and nothing else of this
//! workspace (`docs/cards.md`), so those live here, below the cards. The
//! registry's tables are compiled into `baylee-cards-pool` and reach every
//! other crate through [`generated`], which the linker connects to the pool:
//! a library that looks a card up depends on this crate and not on the
//! twelve thousand card files, so a changed card rebuilds the pool and
//! relinks the programs, and nothing in between.

#![warn(missing_docs)]

/// Filters shared by more than one card file.
#[path = "../../baylee-cards/src/filters.rs"]
pub mod filters;
/// Generated: the token ledger — which id every token there is was assigned.
#[path = "../../baylee-cards/src/generated_tokens.rs"]
pub mod generated_tokens;
/// Central named token definitions (referenced by card files).
#[path = "../../baylee-cards/src/tokens.rs"]
pub mod tokens;

/// The registry's tables, as `baylee-cards-pool` exports them.
pub mod generated {
    use baylee_cards_dsl::CardDef;
    use baylee_core::ids::CardIndex;

    // SAFETY: each name is defined exactly once, by `baylee-cards-pool`
    // (`src/lib.rs`), as an immutable `static` of exactly the type declared
    // here: the pool's `the_exports_have_the_types_the_door_declares` holds
    // the types to each other at compile time. A program that does not link
    // the pool fails to link, naming the symbol, so a read never meets an
    // undefined name; a second definition is a duplicate symbol at link time.
    // Nothing writes to them. Reading an immutable static is then sound.
    unsafe extern "Rust" {
        /// All registered cards, sorted by oracle id for binary search.
        #[link_name = "baylee_cards_pool_all"]
        pub safe static ALL: &'static [(&'static str, &'static CardDef)];
        /// Every card at its `CardIndex`; `None` where an index names no
        /// implemented card.
        #[link_name = "baylee_cards_pool_by_index"]
        pub safe static BY_INDEX: &'static [Option<&'static CardDef>];
        /// FNV-1a hash over the registry content.
        #[link_name = "baylee_cards_pool_hash"]
        pub safe static POOL_HASH: u64;
    }

    /// The card with this Scryfall oracle id, if the pool has it.
    #[must_use]
    pub fn by_oracle_id(oracle_id: &str) -> Option<&'static CardDef> {
        ALL.binary_search_by(|(id, _)| (*id).cmp(oracle_id))
            .ok()
            .map(|i| ALL[i].1)
    }

    /// The card at `index`, if the pool has one there. Bounds-checked: an
    /// index from outside (a deck, a frame) past the table is `None`.
    #[must_use]
    pub fn by_index(index: CardIndex) -> Option<&'static CardDef> {
        BY_INDEX.get(index.get() as usize).copied().flatten()
    }
}

/// The card files as text, for `tokens`' test that no card defines a token of
/// its own (shared with `baylee-cards`' tests).
#[cfg(test)]
#[path = "../../baylee-cards/src/tests/card_files.rs"]
mod card_files;

#[cfg(test)]
mod tests {
    pub(crate) use super::card_files::every_card_file;
}
