//! The tests of the cards under `cards/artifacts/equipment/mv_3/`, one module per card.

#[allow(unused_imports)] // the rig the Equipment of `mv_2` share
use super::mv_2::{
    Rig, colors_of, equipped_gets, equipped_stays_without, goblin_piker, jasmine_boreal, rig,
    rig_vs, steel_wall, subtypes_of,
};
#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod angelic_armaments;
mod barbed_battlegear;
mod behemoth_sledge;
mod brawler_s_plate;
mod butcher_s_cleaver;
mod champion_s_helm;
mod chariot_of_victory;
mod chitinous_cloak;
mod darksteel_plate;
mod ensouled_scimitar;
mod experimental_armor;
mod fireshrieker;
mod gimli_s_axe;
mod greatsword;
mod haunted_cloak;
mod heavy_mattock;
mod loxodon_warhammer;
mod nemesis_mask;
mod nettlecyst;
mod slayer_s_plate;
mod strider_harness;
mod sword_of_hearth_and_home;
mod sword_of_vengeance;
mod tenza_godo_s_maul;
mod umbral_mantle;
mod vulshok_battlegear;
mod warlord_s_axe;
