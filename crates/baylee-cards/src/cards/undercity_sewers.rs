//! Undercity Sewers — (no cost) — Land — Island Swamp
//! Oracle: ({T}: Add {U} or {B}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #270 — Murders at Karlov Manor | Scryfall ID: 2b5801fb-2026-4f25-98bc-ebb2f99684b9 | Oracle ID: 08d80efc-9542-4ba2-824c-c8615d8d07f2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1249,
    oracle_id: "08d80efc-9542-4ba2-824c-c8615d8d07f2",
    scryfall_id: "2b5801fb-2026-4f25-98bc-ebb2f99684b9",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces: &[
    face! {
        name: "Undercity Sewers",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::ISLAND, subtypes::land::SWAMP],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
