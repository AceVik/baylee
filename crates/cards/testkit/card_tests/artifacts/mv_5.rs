//! The tests of the cards under `cards/artifacts/mv_5/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sets the counters of `kind` on `id` (harness door) and refreshes the offer.
fn set_counters(engine: &mut Engine<RegistryLookup>, id: ObjectId, kind: CounterKind, n: u16) {
    let state = engine.dev_state_mut(PlayerId::new(0)).expect("harness");
    state.object_mut(id).expect("object").counters.set(kind, n);
    state.invalidate_projections();
    engine.refresh_offer();
}
mod al_abara_s_carpet;
mod amulet_of_unmaking;
mod blinkmoth_urn;
mod chocobo_racetrack;
mod citanul_flute;
mod farsight_mask;
mod forethought_amulet;
mod gauntlets_of_chaos;
mod gilded_lotus;
mod mercadian_atlas;
mod meteorite;
mod mind_s_eye;
mod mishra_s_helix;
mod myr_matrix;
mod predator_flagship;
mod puffer_extract;
mod ring_of_immortals;
mod ring_of_renewal;
mod ring_of_three_wishes;
mod stone_calendar;
mod the_hive;
mod timeless_lotus;
mod trigon_of_thought;
