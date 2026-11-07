//! One test crate of the card tests.
#![cfg(test)]
#![allow(clippy::wildcard_imports, unused_imports, dead_code)]
// A door's module here re-exports the kit's door and adds the children it compiles.
#![allow(missing_docs, hidden_glob_reexports, clippy::too_many_lines)]

pub use baylee_card_testkit::card_tests::*;

pub mod artifacts {
    pub use baylee_card_testkit::card_tests::artifacts::*;

    mod cyclopean_tomb;
    mod door;
    mod legends;
    mod mv_0;
    mod mv_10;
    mod mv_6;
    mod mv_7;
    mod mv_8;
    mod sunglasses_of_urza;
    mod time_vault;
}
