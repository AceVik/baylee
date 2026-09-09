//! Commercial District — (no cost) — Land — Mountain Forest
//! Oracle: ({T}: Add {R} or {G}.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Set: MKM #259 — Murders at Karlov Manor | Scryfall ID: bf220c06-3cce-4bdd-aa58-83940c223e9c | Oracle ID: b33656ae-3473-4223-845f-f9147f87678b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 366,
    oracle_id: "b33656ae-3473-4223-845f-f9147f87678b",
    scryfall_id: "bf220c06-3cce-4bdd-aa58-83940c223e9c",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces: &[
    face! {
        name: "Commercial District",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::MOUNTAIN, subtypes::land::FOREST],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
