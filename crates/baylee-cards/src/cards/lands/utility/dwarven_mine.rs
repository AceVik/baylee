//! Dwarven Mine — (no cost) — Land — Mountain
//! Oracle: ({T}: Add {R}.)
//! Oracle: This land enters tapped unless you control three or more other Mountains.
//! Oracle: When this land enters untapped, create a 1/1 red Dwarf creature token.
//! Set: ELD #243 — Throne of Eldraine | Scryfall ID: 5c83074d-0c9b-4b58-94ca-d75240485579 | Oracle ID: 74ed0bd3-ac31-41a4-8220-d8e7c8c1c437
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 452,
    oracle_id: "74ed0bd3-ac31-41a4-8220-d8e7c8c1c437",
    scryfall_id: "5c83074d-0c9b-4b58-94ca-d75240485579",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Dwarven Mine",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::MOUNTAIN],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
