//! One test crate of the card tests.
#![cfg(test)]
#![allow(clippy::wildcard_imports, unused_imports, dead_code)]
// A door's module here re-exports the kit's door and adds the children it compiles.
#![allow(missing_docs, hidden_glob_reexports, clippy::too_many_lines)]

// The tests look cards up, so their binary holds the pool.
#[cfg(test)]
use baylee_cards_pool as _;

pub use baylee_card_testkit::card_tests::*;

pub mod lands {
    pub use baylee_card_testkit::card_tests::lands::*;

    mod adventurers_guildhouse;
    mod an_havva_township;
    mod ancient_ziggurat;
    mod archaeological_dig;
    mod artifacts;
    mod aysen_abbey;
    mod basic;
    mod battle;
    mod bounce;
    mod brotherhood_headquarters;
    mod cabal_coffers;
    mod cascading_cataracts;
    mod castle_sengir;
    mod cathedral_of_serra;
    mod caves;
    mod corrupted_crossroads;
    mod creatures;
    mod crowd;
    mod crystal_quarry;
    mod crystal_vein;
    mod cycling;
    mod door;
    mod dual;
    mod eclipsed_realms;
    mod eldrazi_temple;
    mod exotic_orchard;
    mod fetch;
    mod filter;
    mod gaea_s_cradle;
    mod gain;
    mod gates;
    mod gemstone_caverns;
    mod gemstone_mine;
    mod great_hall_of_the_citadel;
    mod guildmages_forum;
    mod halls_of_mist;
    mod henge_of_ramos;
    mod horizon;
    mod koskun_keep;
    mod lake_of_the_dead;
    mod legendary;
    mod loci;
    mod lotus_vale;
    mod mana_confluence;
    mod mana_scope;
    mod mana_spend;
    mod mines;
    mod mishra_s_workshop;
    mod mountain_stronghold;
    mod muraganda_raceway;
    mod nephalia_academy;
    mod no_untap;
    mod opal_palace;
    mod pain;
    mod pathway;
    mod phyrexian_tower;
    mod pillar_of_the_paruns;
    mod planar_nexus;
    mod planets;
    mod rainbow_vale;
    mod reflecting_pool;
    mod refuge;
    mod reliquary_tower;
    mod restricted;
    mod reveal;
    mod riftstone_portal;
    mod river_of_tears;
    mod saddle;
    mod school_of_the_unseen;
    mod scorched_ruins;
    mod scry;
    mod seafarer_s_quay;
    mod secluded_courtyard;
    mod secret_base;
    mod shimmering_grotto;
    mod slow;
    mod slumbering_waterways;
    mod spheres;
    mod storage;
    mod study_hall;
    mod surveil;
    mod tendo_ice_bridge;
    mod the_tabernacle_at_pendrell_vale;
    mod top_card_choices;
    mod tournament_grounds;
    mod towers;
    mod towns;
    mod triome;
    mod unclaimed_territory;
    mod underdome;
    mod undiscovered_paradise;
    mod unholy_citadel;
    mod unknown_shores;
    mod unlucky;
    mod urza_s_power_plant;
    mod vesuva;
    mod watermarket;
    mod white_lotus_hideout;
    mod wizards_school;
    mod yavimaya_cradle_of_growth;
    mod zoetic_cavern;
}
