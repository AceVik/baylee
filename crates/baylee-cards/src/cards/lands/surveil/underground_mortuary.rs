//! Underground Mortuary — (no cost) — Land — Swamp Forest
//! Oracle: ({T}: Add {B} or {G}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #271 — Murders at Karlov Manor | Scryfall ID: f6ca59cd-8779-4a84-a54b-e863b79c61f0 | Oracle ID: 840119bf-e60f-4ff7-9c9b-d420d09df545
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1252,
    oracle_id: "840119bf-e60f-4ff7-9c9b-d420d09df545",
    scryfall_id: "f6ca59cd-8779-4a84-a54b-e863b79c61f0",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces: &[
    face! {
        name: "Underground Mortuary",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::SWAMP, subtypes::land::FOREST],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
