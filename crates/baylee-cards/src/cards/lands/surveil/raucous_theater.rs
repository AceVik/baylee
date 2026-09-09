//! Raucous Theater — (no cost) — Land — Swamp Mountain
//! Oracle: ({T}: Add {B} or {R}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #266 — Murders at Karlov Manor | Scryfall ID: b598c93e-dae1-4d71-a9e4-917abf76d2d0 | Oracle ID: 04e5e84f-8fd4-43ab-8f9d-5b24646f7ae5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 898,
    oracle_id: "04e5e84f-8fd4-43ab-8f9d-5b24646f7ae5",
    scryfall_id: "b598c93e-dae1-4d71-a9e4-917abf76d2d0",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces: &[
    face! {
        name: "Raucous Theater",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::SWAMP, subtypes::land::MOUNTAIN],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
