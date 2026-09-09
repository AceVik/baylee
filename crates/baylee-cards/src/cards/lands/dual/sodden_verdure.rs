//! Sodden Verdure — (no cost) — Land — Forest Island
//! Oracle: ({T}: Add {G} or {U}.)
//! Oracle: This land enters tapped unless you control two or more basic lands.
//! Set: MSC #267 — Marvel Super Heroes Commander | Scryfall ID: 0a451ab5-6244-48a0-a638-07da72b156b5 | Oracle ID: 0070db93-142b-4d04-afd3-836792dc134b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1062,
    oracle_id: "0070db93-142b-4d04-afd3-836792dc134b",
    scryfall_id: "0a451ab5-6244-48a0-a638-07da72b156b5",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces: &[
    face! {
        name: "Sodden Verdure",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::FOREST, subtypes::land::ISLAND],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
