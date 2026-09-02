//! Scorched Geyser — (no cost) — Land — Island Mountain
//! Oracle: ({T}: Add {U} or {R}.)
//! Oracle: This land enters tapped unless you control two or more basic lands.
//! Set: MSC #264 — Marvel Super Heroes Commander | Scryfall ID: 43abdfba-0a55-4b8a-858e-3372ee40a579 | Oracle ID: f808b510-907a-4c3c-aea1-efb825c8e13e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 974,
    oracle_id: "f808b510-907a-4c3c-aea1-efb825c8e13e",
    scryfall_id: "43abdfba-0a55-4b8a-858e-3372ee40a579",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces: &[
    face! {
        name: "Scorched Geyser",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::ISLAND, subtypes::land::MOUNTAIN],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
