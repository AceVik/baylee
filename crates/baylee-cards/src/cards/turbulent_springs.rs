//! Turbulent Springs — (no cost) — Land — Island Mountain
//! Oracle: ({T}: Add {U} or {R}.)
//! Oracle: This land enters tapped unless your opponents control eight or more lands.
//! Set: SOC #57 — Secrets of Strixhaven Commander | Scryfall ID: a25cb813-aa6d-469c-aa29-61ffa32267f2 | Oracle ID: 9aef7510-9f06-4939-8cae-f71330d1105e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1234,
    oracle_id: "9aef7510-9f06-4939-8cae-f71330d1105e",
    scryfall_id: "a25cb813-aa6d-469c-aa29-61ffa32267f2",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces: &[
    face! {
        name: "Turbulent Springs",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::ISLAND, subtypes::land::MOUNTAIN],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
