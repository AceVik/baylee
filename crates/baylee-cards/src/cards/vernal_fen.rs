//! Vernal Fen — (no cost) — Land — Swamp Forest
//! Oracle: ({T}: Add {B} or {G}.)
//! Oracle: This land enters tapped unless you control two or more basic lands.
//! Set: SOC #419 — Secrets of Strixhaven Commander | Scryfall ID: 0466cf57-bb3e-4359-8fe8-d6cc1288fdc6 | Oracle ID: 40544d12-0391-4a61-af95-9b8ec01ed8fc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1286,
    oracle_id: "40544d12-0391-4a61-af95-9b8ec01ed8fc",
    scryfall_id: "0466cf57-bb3e-4359-8fe8-d6cc1288fdc6",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces: &[
    face! {
        name: "Vernal Fen",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::SWAMP, subtypes::land::FOREST],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
