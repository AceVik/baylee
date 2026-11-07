//! One test crate of the card tests.
#![cfg(test)]
#![allow(clippy::wildcard_imports, unused_imports, dead_code)]
// A door's module here re-exports the kit's door and adds the children it compiles.
#![allow(missing_docs, hidden_glob_reexports, clippy::too_many_lines)]

pub use baylee_card_testkit::card_tests::*;

pub mod instants {
    pub use baylee_card_testkit::card_tests::instants::*;

    mod mv_2;
    mod mv_3;
    mod mv_5;
    mod mv_7;
    mod mv_8;
    mod mv_9;
}
