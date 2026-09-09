//! Shadowy Backstreet — (no cost) — Land — Plains Swamp
//! Oracle: ({T}: Add {W} or {B}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #268 — Murders at Karlov Manor | Scryfall ID: 69c1b656-1d67-499c-bf0f-417682a86c7d | Oracle ID: 216a2a92-9ca3-4ca3-8af7-686c13b04290
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1007,
    oracle_id: "216a2a92-9ca3-4ca3-8af7-686c13b04290",
    scryfall_id: "69c1b656-1d67-499c-bf0f-417682a86c7d",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    faces: &[
    face! {
        name: "Shadowy Backstreet",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::SWAMP],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
