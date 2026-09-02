//! Elegant Parlor — (no cost) — Land — Mountain Plains
//! Oracle: ({T}: Add {R} or {W}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #260 — Murders at Karlov Manor | Scryfall ID: 72c6d541-e2cb-4d6e-acac-90a8f53b7006 | Oracle ID: 9ea747cf-5d04-4aa7-bdc3-8145860cd1ba
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 463,
    oracle_id: "9ea747cf-5d04-4aa7-bdc3-8145860cd1ba",
    scryfall_id: "72c6d541-e2cb-4d6e-acac-90a8f53b7006",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::White]),
    faces: &[
    face! {
        name: "Elegant Parlor",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::MOUNTAIN, subtypes::land::PLAINS],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
