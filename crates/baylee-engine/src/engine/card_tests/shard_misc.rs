//! One test crate of the card tests.
#![cfg(test)]
#![allow(clippy::wildcard_imports, unused_imports, dead_code)]
// A door's module here re-exports the kit's door and adds the children it compiles.
#![allow(missing_docs, hidden_glob_reexports, clippy::too_many_lines)]

pub use baylee_card_testkit::card_tests::*;

mod activation_cost_bounds;
mod alpha_completion_review;
mod archon_kismet;
mod batch43;
mod batch45;
mod batch46;
mod batch47;
mod batch48;
mod batch48_activations;
mod batch49_nafs;
mod batch49_returns;
mod batch49_turns;
mod batch49_turtle;
mod batch49_untap;
mod batch49_wall;
mod batch52_guardian;
mod batch52_horse_johan;
mod batch52_protection;
mod batch52_song;
mod batch52_wall;
mod batch53_cards;
mod batch54_final_two;
mod batch54_gond;
mod batch54_riftstone;
mod battles;
mod counted_activation_costs;
mod counter_auras;
mod counter_limits;
mod counter_movement;
mod death_returns;
mod divided_spells;
mod entry_replacements;
mod hack_mana_routes;
mod hone_counters;
mod keyword_counters;
mod land_equilibrium;
mod library_followups;
mod looting;
mod luna_oracle_pilot;
mod own;
mod planeswalkers;
mod printed_costs;
mod radiation;
mod rules;
mod sacrifice_outcomes;
mod spell_endings;
mod storm;
mod stun_counters;
mod underdome_acceptance;
pub mod counter_resources {
    pub use baylee_card_testkit::card_tests::counter_resources::*;

    mod door;
}
pub mod creature_count_combat {
    pub use baylee_card_testkit::card_tests::creature_count_combat::*;
}
pub mod hidden_exile {
    pub use baylee_card_testkit::card_tests::hidden_exile::*;

    mod casts;
    mod door;
    mod land_entry;
    mod permissions;
    mod vault_edges;
}
pub mod library_choices {
    pub use baylee_card_testkit::card_tests::library_choices::*;

    mod door;
}
