//! One test crate of the card tests.
#![cfg(test)]
#![allow(clippy::wildcard_imports, unused_imports, dead_code)]
// A door's module here re-exports the kit's door and adds the children it compiles.
#![allow(missing_docs, hidden_glob_reexports, clippy::too_many_lines)]

pub use baylee_card_testkit::card_tests::*;

pub mod creatures {
    pub use baylee_card_testkit::card_tests::creatures::*;

    mod enchantments;
    mod landwalk_permissions;
    mod mv_10;
    mod mv_8;
    mod nether_shadow;
    mod pkg5_non_triggered;
    mod sengir_vampire;
}
