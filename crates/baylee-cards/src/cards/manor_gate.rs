//! Manor Gate — (no cost) — Land — Gate
//! Oracle: This land enters tapped.
//! Oracle: As this land enters, choose a color other than green.
//! Oracle: {T}: Add {G} or one mana of the chosen color.
//! Set: CLB #356 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 793d4978-9e00-453d-8dc2-6d51ad6c26b7 | Oracle ID: dd6e67c0-66a1-49b7-8a86-3cf4b209fd07
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 739,
    oracle_id: "dd6e67c0-66a1-49b7-8a86-3cf4b209fd07",
    scryfall_id: "793d4978-9e00-453d-8dc2-6d51ad6c26b7",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Manor Gate",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::GATE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
