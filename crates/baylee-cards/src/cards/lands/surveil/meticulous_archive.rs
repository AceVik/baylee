//! Meticulous Archive — (no cost) — Land — Plains Island
//! Oracle: ({T}: Add {W} or {U}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #264 — Murders at Karlov Manor | Scryfall ID: 652236c2-84ef-45e4-b5fc-ed6170bc3d6c | Oracle ID: ccfb8b4d-651c-418a-aa19-cb23105b3f2f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 754,
    oracle_id: "ccfb8b4d-651c-418a-aa19-cb23105b3f2f",
    scryfall_id: "652236c2-84ef-45e4-b5fc-ed6170bc3d6c",
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces: &[
    face! {
        name: "Meticulous Archive",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::ISLAND],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
