//! Thundering Falls — (no cost) — Land — Island Mountain
//! Oracle: ({T}: Add {U} or {R}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #269 — Murders at Karlov Manor | Scryfall ID: 17260fff-b239-4af4-9306-3236ae3fa5a5 | Oracle ID: d2bcff58-7a8a-46ef-b6b3-39501d4c8e6e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1199,
    oracle_id: "d2bcff58-7a8a-46ef-b6b3-39501d4c8e6e",
    scryfall_id: "17260fff-b239-4af4-9306-3236ae3fa5a5",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces: &[
    face! {
        name: "Thundering Falls",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::ISLAND, subtypes::land::MOUNTAIN],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
