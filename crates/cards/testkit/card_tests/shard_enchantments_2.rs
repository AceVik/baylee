//! One test crate of the card tests.
#![cfg(test)]
#![allow(clippy::wildcard_imports, unused_imports, dead_code)]
// A door's module here re-exports the kit's door and adds the children it compiles.
#![allow(missing_docs, hidden_glob_reexports, clippy::too_many_lines)]

// The tests look cards up, so their binary holds the pool.
#[cfg(test)]
use baylee_cards_pool as _;

pub use baylee_card_testkit::card_tests::*;

pub mod enchantments {
    pub use baylee_card_testkit::card_tests::enchantments::*;

    mod animate_artifact;
    mod aura_bindings;
    mod consecrate_land;
    mod creature_bond;
    mod door;
    mod earthbind;
    mod earthbind_independent;
    mod gloom;
    mod kindred;
    mod legends;
    mod lich;
    mod mv_1;
    mod mv_2;
    mod mv_4;
    mod mv_6;
    mod mv_8;
    mod rooms;
    mod sagas;
}
