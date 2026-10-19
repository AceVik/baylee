//! The tests of the cards under `cards/artifacts/mv_4/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod barl_s_cage;
mod birthing_pod;
mod bottle_of_suleiman;
mod conduit_of_worlds;
mod conservator;
mod cyclopean_tomb;
mod diamond_kaleidoscope;
mod dingus_egg;
mod elixir_of_vitality;
mod flying_carpet;
mod fodder_cannon;
mod gauntlet_of_might;
mod goblin_cannon;
mod icy_manipulator;
mod jade_monolith;
mod jade_statue;
mod jayemdae_tome;
mod kormus_bell;
mod life_chisel;
mod living_armor;
mod machine_god_s_effigy;
mod mightstone;
mod nevinyrral_s_disk;
mod nova_pentacle;
mod panharmonicon;
mod power_armor;
mod power_matrix;
mod primal_amulet;
mod reflecting_mirror;
mod rocket_launcher;
mod rod_of_ruin;
mod sandals_of_abdallah;
mod sisay_s_ring;
mod skull_catapult;
mod staff_of_zegon;
mod tawnos_s_wand;
mod the_one_ring;
mod thousand_moons_smithy;
mod thran_dynamo;
mod tower_of_champions;
mod tower_of_eons;
mod tower_of_fortunes;
mod tower_of_murmurs;
mod triassic_egg;
mod ur_golem_s_eye;
mod wand_of_ith;
mod wand_of_the_elements;
mod war_barge;
mod weakstone;

mod life_matrix;

mod cursed_rack;

mod alchor_s_tomb;

mod anchovy_banana_pizza;
mod arcane_spyglass;
mod cumber_stone;
mod dingus_staff;
mod emmessi_tome;

/// Sets the counters of `kind` on `id` (harness door) and refreshes the offer.
fn set_counters(engine: &mut Engine<RegistryLookup>, id: ObjectId, kind: CounterKind, n: u16) {
    let state = engine.dev_state_mut(PlayerId::new(0)).expect("harness");
    state.object_mut(id).expect("object").counters.set(kind, n);
    state.invalidate_projections();
    engine.refresh_offer();
}
mod empowered_autogenerator;
mod erratic_portal;
mod five_hundred_year_diary;
mod font_of_mythos;
mod gemstone_array;
mod goblin_boom_keg;
mod grafted_skullcap;
mod gruul_war_plow;
mod hedron_archive;
mod isolation_cell;
mod jade_idol;
mod lux_cannon;
mod magnetic_mine;
mod mishra_s_groundbreaker;
mod otherworld_atlas;
mod pentagram_of_the_ages;
mod relic_of_sauron;
mod ring_of_the_lucii;
mod seer_s_sundial;
mod skullmead_cauldron;
mod tawnos_s_coffin;
mod time_bomb;
mod tower_of_calamities;
mod trading_post;
mod unbender_tine;
