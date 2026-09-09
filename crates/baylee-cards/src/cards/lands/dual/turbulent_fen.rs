//! Turbulent Fen — (no cost) — Land — Swamp Forest
//! Oracle: ({T}: Add {B} or {G}.)
//! Oracle: This land enters tapped unless your opponents control eight or more lands.
//! Set: SOC #55 — Secrets of Strixhaven Commander | Scryfall ID: 9737159b-256c-4004-ba5f-0417c35e1b30 | Oracle ID: 114dd40d-5ad8-4913-a08f-572b9521eb5b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1232,
    oracle_id: "114dd40d-5ad8-4913-a08f-572b9521eb5b",
    scryfall_id: "9737159b-256c-4004-ba5f-0417c35e1b30",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces: &[
    face! {
        name: "Turbulent Fen",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::SWAMP, subtypes::land::FOREST],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
