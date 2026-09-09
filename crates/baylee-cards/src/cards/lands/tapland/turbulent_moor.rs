//! Turbulent Moor — (no cost) — Land — Plains Swamp
//! Oracle: ({T}: Add {W} or {B}.)
//! Oracle: This land enters tapped unless your opponents control eight or more lands.
//! Set: SOC #56 — Secrets of Strixhaven Commander | Scryfall ID: da54562d-8c09-4728-bb2b-ae8d464106b8 | Oracle ID: 2eb4da30-2600-4a7f-8e6c-6a090faa9a8d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1233,
    oracle_id: "2eb4da30-2600-4a7f-8e6c-6a090faa9a8d",
    scryfall_id: "da54562d-8c09-4728-bb2b-ae8d464106b8",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    faces: &[
    face! {
        name: "Turbulent Moor",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::SWAMP],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
