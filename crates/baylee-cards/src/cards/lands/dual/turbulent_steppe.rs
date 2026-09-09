//! Turbulent Steppe — (no cost) — Land — Mountain Plains
//! Oracle: ({T}: Add {R} or {W}.)
//! Oracle: This land enters tapped unless your opponents control eight or more lands.
//! Set: SOC #58 — Secrets of Strixhaven Commander | Scryfall ID: c0c73dd5-0a88-4a94-9c01-c1473433e46f | Oracle ID: db444f9d-4dde-4308-b0f2-7acfe6de871a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1235,
    oracle_id: "db444f9d-4dde-4308-b0f2-7acfe6de871a",
    scryfall_id: "c0c73dd5-0a88-4a94-9c01-c1473433e46f",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::White]),
    faces: &[
    face! {
        name: "Turbulent Steppe",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::MOUNTAIN, subtypes::land::PLAINS],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
