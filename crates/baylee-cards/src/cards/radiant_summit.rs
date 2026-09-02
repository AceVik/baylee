//! Radiant Summit — (no cost) — Land — Mountain Plains
//! Oracle: ({T}: Add {R} or {W}.)
//! Oracle: This land enters tapped unless you control two or more basic lands.
//! Set: MSC #258 — Marvel Super Heroes Commander | Scryfall ID: 81b619cc-900c-4d65-83da-8b9dcb370984 | Oracle ID: 5dd0cc44-4647-4857-ad3b-22494099d08a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 889,
    oracle_id: "5dd0cc44-4647-4857-ad3b-22494099d08a",
    scryfall_id: "81b619cc-900c-4d65-83da-8b9dcb370984",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::White]),
    faces: &[
    face! {
        name: "Radiant Summit",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::MOUNTAIN, subtypes::land::PLAINS],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
