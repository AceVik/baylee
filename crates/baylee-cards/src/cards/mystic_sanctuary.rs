//! Mystic Sanctuary — (no cost) — Land — Island
//! Oracle: ({T}: Add {U}.)
//! Oracle: This land enters tapped unless you control three or more other Islands.
//! Oracle: When this land enters untapped, you may put target instant or sorcery card from your graveyard on top of your library.
//! Set: SOC #388 — Secrets of Strixhaven Commander | Scryfall ID: 4cd86997-d7b9-4b5b-9488-11f5c679e4d3 | Oracle ID: 17b60106-a4c7-410a-8ac3-ec8e74e29a7c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 799,
    oracle_id: "17b60106-a4c7-410a-8ac3-ec8e74e29a7c",
    scryfall_id: "4cd86997-d7b9-4b5b-9488-11f5c679e4d3",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Mystic Sanctuary",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::ISLAND],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
