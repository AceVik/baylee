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
mod chocobo_racetrack;
mod citanul_flute;
mod coat_of_arms;
mod forethought_amulet;
mod gauntlets_of_chaos;
mod gilded_lotus;
mod jester_s_mask;
mod mangara_s_tome;
mod meteorite;
mod myr_matrix;
mod naked_singularity;
mod predator_flagship;
mod ring_of_immortals;
mod ring_of_renewal;
mod ring_of_three_wishes;
mod stone_calendar;
mod storm_cauldron;
mod the_hive;
mod timeless_lotus;
mod trigon_of_thought;
mod volrath_s_laboratory;
