//! Savannah — (no cost) — Land — Forest Plains
//! Oracle: ({T}: Add {G} or {W}.)
//! Set: VMA #311 — Vintage Masters | Scryfall ID: b0d161fc-4a2a-4f1d-82b4-a746552552df | Oracle ID: 703243f0-8cb3-420f-958f-5fd4bde30293
// IMPLEMENTED — mana from current basic land types (CR 305.6).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SUBS: &[SubtypeId] = &[land::FOREST, land::PLAINS];

card!(
    index = index::SAVANNAH,
    oracle_id = "703243f0-8cb3-420f-958f-5fd4bde30293",
    scryfall_id = "b0d161fc-4a2a-4f1d-82b4-a746552552df",
    faces = &[face!(
        name = "Savannah",
        types = TypeSet::LAND,
        subtypes = SUBS,
    )],
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::intrinsic_mana()])],
);
