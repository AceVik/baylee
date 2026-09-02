//! Festering Gulch — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, it deals 1 damage to target opponent.
//! Oracle: {T}: Add {B} or {G}.
//! Set: OTJ #257 — Outlaws of Thunder Junction | Scryfall ID: 4ad841eb-da0d-43d4-8b60-efe30922990b | Oracle ID: 9d3b60af-3e38-4d36-95fc-11b31c38f955
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 488,
    oracle_id: "9d3b60af-3e38-4d36-95fc-11b31c38f955",
    scryfall_id: "4ad841eb-da0d-43d4-8b60-efe30922990b",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces: &[
    face! {
        name: "Festering Gulch",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
