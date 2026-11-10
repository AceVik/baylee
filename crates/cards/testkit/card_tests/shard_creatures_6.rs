//! One test crate of the card tests.
#![cfg(test)]
#![allow(clippy::wildcard_imports, unused_imports, dead_code)]
// A door's module here re-exports the kit's door and adds the children it compiles.
#![allow(missing_docs, hidden_glob_reexports, clippy::too_many_lines)]

// The tests look cards up, so their binary holds the pool.
#[cfg(test)]
use baylee_cards_pool as _;

pub use baylee_card_testkit::card_tests::*;

pub mod creatures {
    pub use baylee_card_testkit::card_tests::creatures::*;

    mod artifacts;
    mod mv_5;
}
