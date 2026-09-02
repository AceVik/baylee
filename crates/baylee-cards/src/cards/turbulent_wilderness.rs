//! Turbulent Wilderness — (no cost) — Land — Forest Island
//! Oracle: ({T}: Add {G} or {U}.)
//! Oracle: This land enters tapped unless your opponents control eight or more lands.
//! Set: SOC #59 — Secrets of Strixhaven Commander | Scryfall ID: b6ca755c-4b17-450b-b995-bfec8a55396f | Oracle ID: bd8adca6-4f16-45f8-994a-fe55bd573bd0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1236,
    oracle_id: "bd8adca6-4f16-45f8-994a-fe55bd573bd0",
    scryfall_id: "b6ca755c-4b17-450b-b995-bfec8a55396f",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces: &[
    face! {
        name: "Turbulent Wilderness",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::FOREST, subtypes::land::ISLAND],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
