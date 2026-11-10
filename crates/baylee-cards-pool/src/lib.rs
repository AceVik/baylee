//! baylee-cards-pool — the card files and the registry's tables.
//!
//! Only programs depend on this crate: a binary, a test binary, a bench.
//! Libraries read the registry through `baylee_cards_base::generated`, which
//! the linker connects to the statics exported here, so a changed card
//! rebuilds this crate and relinks the programs that hold it, and no library
//! between them (`docs/cards.md` §"Where a card compiles"). A program that
//! looks a card up and does not hold the pool fails to link, naming
//! `baylee_cards_pool_all`; it takes `use baylee_cards_pool as _;` at its
//! root.

#![warn(missing_docs)]

pub use baylee_cards_base::{filters, generated_tokens, tokens};
use baylee_cards_dsl::CardDef;

/// Generated: one module per card.
#[path = "../../baylee-cards/src/cards/mod.rs"]
pub mod cards;
/// Generated: registry tables.
#[path = "../../baylee-cards/src/generated.rs"]
mod generated;

/// What `baylee_cards_base::generated::ALL` reads.
#[unsafe(export_name = "baylee_cards_pool_all")]
static ALL: &[(&str, &CardDef)] = generated::ALL;
/// What `baylee_cards_base::generated::BY_INDEX` reads.
#[unsafe(export_name = "baylee_cards_pool_by_index")]
static BY_INDEX: &[Option<&CardDef>] = generated::BY_INDEX;
/// What `baylee_cards_base::generated::POOL_HASH` reads.
#[unsafe(export_name = "baylee_cards_pool_hash")]
static POOL_HASH: u64 = generated::POOL_HASH;

/// The door and the exports, held to one type each: a declaration in
/// `baylee-cards-base` that drifted from what this crate exports would read
/// the wrong type through an `extern` the compiler cannot check, so this
/// makes the drift a compile error here.
const _: () = {
    const fn same<T: ?Sized>(_: &T, _: &T) {}
    #[allow(dead_code)]
    fn the_exports_have_the_types_the_door_declares() {
        same(&ALL, &baylee_cards_base::generated::ALL);
        same(&BY_INDEX, &baylee_cards_base::generated::BY_INDEX);
        same(&POOL_HASH, &baylee_cards_base::generated::POOL_HASH);
    }
};

#[cfg(test)]
mod tests {
    /// The door reads what this crate exports: the same tables, at the same
    /// addresses.
    #[test]
    fn the_door_reads_the_exported_tables() {
        use baylee_cards_base::generated as door;
        assert!(std::ptr::eq(door::ALL, super::ALL));
        assert!(std::ptr::eq(door::BY_INDEX, super::BY_INDEX));
        assert_eq!(door::POOL_HASH, super::POOL_HASH);
        assert!(door::ALL.len() > 1000, "{}", door::ALL.len());
        let (oracle_id, card) = door::ALL[0];
        assert!(std::ptr::eq(door::by_oracle_id(oracle_id).unwrap(), card));
        assert!(door::by_index(baylee_core::ids::CardIndex::new(u32::MAX)).is_none());
    }
}
