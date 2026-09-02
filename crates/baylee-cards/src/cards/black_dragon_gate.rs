//! Black Dragon Gate — (no cost) — Land — Gate
//! Oracle: This land enters tapped.
//! Oracle: As this land enters, choose a color other than black.
//! Oracle: {T}: Add {B} or one mana of the chosen color.
//! Set: CLB #347 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: c4ceb589-c741-44ac-98c8-3d997953ee61 | Oracle ID: dde6bce5-8bbe-4866-b5aa-2c05c7d37241
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 273,
    oracle_id: "dde6bce5-8bbe-4866-b5aa-2c05c7d37241",
    scryfall_id: "c4ceb589-c741-44ac-98c8-3d997953ee61",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Black Dragon Gate",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::GATE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
