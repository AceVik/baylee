//! Smoldering Marsh — (no cost) — Land — Swamp Mountain
//! Oracle: ({T}: Add {B} or {R}.)
//! Oracle: This land enters tapped unless you control two or more basic lands.
//! Set: MSC #266 — Marvel Super Heroes Commander | Scryfall ID: d707c477-440f-417c-970a-0e7426a58045 | Oracle ID: 390f1b56-264e-4336-83be-dc1fe79bfdcf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1052,
    oracle_id: "390f1b56-264e-4336-83be-dc1fe79bfdcf",
    scryfall_id: "d707c477-440f-417c-970a-0e7426a58045",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces: &[
    face! {
        name: "Smoldering Marsh",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::SWAMP, subtypes::land::MOUNTAIN],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
