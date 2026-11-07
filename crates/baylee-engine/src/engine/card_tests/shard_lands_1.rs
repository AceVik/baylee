//! One test crate of the card tests.
#![cfg(test)]
#![allow(clippy::wildcard_imports, unused_imports, dead_code)]
// A door's module here re-exports the kit's door and adds the children it compiles.
#![allow(missing_docs, hidden_glob_reexports, clippy::too_many_lines)]

pub use baylee_card_testkit::card_tests::*;

pub mod lands {
    pub use baylee_card_testkit::card_tests::lands::*;

    mod check;
    mod deserts;
    mod grove_of_the_burnwillows;
    mod manlands;
    mod shock;
    mod tapland;
    mod utility;
}
