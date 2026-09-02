//! Hedge Maze — (no cost) — Land — Forest Island
//! Oracle: ({T}: Add {G} or {U}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #262 — Murders at Karlov Manor | Scryfall ID: 5260f8ae-805b-4eae-badf-62de0f768867 | Oracle ID: ca4b6689-04ee-4227-9bdc-cb5a9590c745
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 613,
    oracle_id: "ca4b6689-04ee-4227-9bdc-cb5a9590c745",
    scryfall_id: "5260f8ae-805b-4eae-badf-62de0f768867",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces: &[
    face! {
        name: "Hedge Maze",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::FOREST, subtypes::land::ISLAND],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
