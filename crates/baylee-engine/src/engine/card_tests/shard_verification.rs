//! The verification hooks' own tests, and the card tests they run as subjects:
//! a hook is tested the way its consumer runs it, in one test binary.
#![cfg(test)]
#![allow(clippy::wildcard_imports, unused_imports, dead_code)]
#![allow(missing_docs, hidden_glob_reexports, clippy::too_many_lines)]

pub use baylee_card_testkit::card_tests::*;
pub use baylee_card_testkit::testkit;

#[path = "../card_rider_tests.rs"]
mod card_rider_tests;
#[path = "../verification_tests.rs"]
mod verification_tests;

pub mod artifacts {
    pub use baylee_card_testkit::card_tests::artifacts::*;

    #[path = "equipment/mv_3/loxodon_warhammer.rs"]
    mod loxodon_warhammer;
    #[path = "time_vault.rs"]
    mod time_vault;
}

pub mod creatures {
    pub use baylee_card_testkit::card_tests::creatures::*;

    #[path = "mv_3/prodigal_sorcerer.rs"]
    mod prodigal_sorcerer;
    #[path = "mv_1/soul_warden.rs"]
    mod soul_warden;
}

pub mod enchantments {
    pub use baylee_card_testkit::card_tests::enchantments::*;

    #[path = "mv_3/glorious_anthem.rs"]
    mod glorious_anthem;
    #[path = "mv_2/island_sanctuary.rs"]
    mod island_sanctuary;
    #[path = "lich.rs"]
    mod lich;
}

pub mod instants {
    pub use baylee_card_testkit::card_tests::instants::*;

    #[path = "mv_1/lightning_bolt.rs"]
    mod lightning_bolt;
}

pub mod lands {
    pub use baylee_card_testkit::card_tests::lands::*;

    #[path = "storage/bottomless_vault.rs"]
    mod bottomless_vault;
}

pub mod sorceries {
    pub use baylee_card_testkit::card_tests::sorceries::*;

    #[path = "mv_7/temporal_mastery.rs"]
    mod temporal_mastery;
}
