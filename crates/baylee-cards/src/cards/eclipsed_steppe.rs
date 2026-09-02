//! Eclipsed Steppe — (no cost) — Land — Plains Swamp
//! Oracle: ({T}: Add {W} or {B}.)
//! Oracle: This land enters tapped unless you control two or more basic lands.
//! Set: SOC #53 — Secrets of Strixhaven Commander | Scryfall ID: d890999a-dcc7-479e-b0f0-60388c737043 | Oracle ID: 6216635f-8e6e-40a3-9659-ef6352ab92ce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 457,
    oracle_id: "6216635f-8e6e-40a3-9659-ef6352ab92ce",
    scryfall_id: "d890999a-dcc7-479e-b0f0-60388c737043",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    faces: &[
    face! {
        name: "Eclipsed Steppe",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::SWAMP],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
