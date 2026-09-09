//! Sapseep Forest — (no cost) — Land — Forest
//! Oracle: ({T}: Add {G}.)
//! Oracle: This land enters tapped.
//! Oracle: {G}, {T}: You gain 1 life. Activate only if you control two or more green permanents.
//! Set: C21 #313 — Commander 2021 | Scryfall ID: 81d3099d-4f22-425c-8955-903b6cfb88d3 | Oracle ID: 8d4dcab0-86e5-4ff8-a90f-78a062664e16
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 965,
    oracle_id: "8d4dcab0-86e5-4ff8-a90f-78a062664e16",
    scryfall_id: "81d3099d-4f22-425c-8955-903b6cfb88d3",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Sapseep Forest",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::FOREST],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
